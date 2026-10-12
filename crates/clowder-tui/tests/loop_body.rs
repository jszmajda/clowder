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
/// carries the appointment's own note (TUI-NAV-107) and no refusal's.
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
        // The one accepted act the footer confirms (TUI-NAV-107) — swept on
        // its own below; here only that the note is the appointment's, never
        // a refusal's.
        assert!(
            a.view.footer.as_ref().is_some_and(|(note, _)| note.contains(a.state.kittens[ix].name.as_str())),
            "{whr}, the accepted appointment did not leave its footer note naming the Elder: {:?}",
            a.view.footer
        );
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
// Whether the core accepted the command a press issued is known only where
// the command is applied, which is the loop; so is which line of the frame a
// footer note is drawn on. All of it is asserted through `step`.
//
// One clause has no reachable case: a press of `n` the core refuses. The key
// is live only on a listed kitten's row, a row is listed only for a kitten in
// the roster, and the core accepts AppointElder for every kitten in the
// roster — the press and its command meet the same state within one step —
// so no meadow makes the core refuse one. What a refusal leaves is asserted
// on the refusal the same row does reach, its `Enter`, which is where "the
// footer line that carries TUI-NAV-014's refusals" and "lasting as such a
// note does" are read from.
// ---------------------------------------------------------------------------

/// The text of one row of the terminal, as the last frame drew it.
fn screen_row(term: &Terminal<TestBackend>, rect: ratatui::layout::Rect) -> String {
    let buf = term.backend().buffer();
    (rect.x..rect.right()).map(|x| buf.cell((x, rect.y)).map(|c| c.symbol()).unwrap_or(" ")).collect()
}

/// The whole screen, as the last frame drew it.
fn screen(term: &Terminal<TestBackend>) -> String {
    let buf = term.backend().buffer();
    let area = buf.area;
    (area.y..area.bottom())
        .map(|y| (area.x..area.right()).map(|x| buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(" ")).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

/// How many journal entries record an appointment of a kitten of this name.
fn appointments_journalled(state: &GameState, name: &str) -> usize {
    use clowder_core::{Event, JournalItem};
    state
        .journal
        .recent
        .iter()
        .chain(state.journal.kept.iter())
        .filter(|e| matches!(&e.item, JournalItem::Happened(Event::ElderAppointed { name: n }) if n == name))
        .count()
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

/// A press of `n` on a kitten's row that the core accepts leaves one footer
/// note, on the line of the frame a refusal's note is drawn on and lasting as
/// long as one does, saying in plain words that the kitten the core then holds
/// as the Elder is the Elder — by that kitten's whole name as the core holds
/// it, never the form its row shortened — whether nobody, another kitten, or
/// that very kitten was the Elder before the press. The event's ticker line
/// and its Journal entry stand beside the note on the same frame. Quantified
/// over every kitten's row, over each of those three standings, in the narrow
/// list, the narrow-mode inspector swap, and the three-pane layout.
///
/// @spec TUI-NAV-107
#[test]
fn an_accepted_n_leaves_one_footer_note_naming_the_elder_the_core_holds_by_its_whole_name() {
    use clowder_tui::nav::Section;
    use crossterm::event::KeyCode;

    let base = small_roster();
    let kittens = kitten_row_keys(&base);
    assert_eq!(kittens.len(), ROSTER_NAMES.len(), "fixture: every kitten's row is listed");
    let long = ROSTER_NAMES.iter().max_by_key(|n| n.len()).copied().unwrap();

    let layouts: [(&str, u16, u16, bool); 3] = [
        ("the narrow list", 80, 30, false),
        ("the narrow-mode inspector swap", 80, 30, true),
        ("the three-pane layout", 130, 40, false),
    ];
    let (mut checked, mut shortened_on_its_row) = (0u32, 0u32);
    for (layout_name, w, h, swap) in layouts {
        for (ix, row_key) in &kittens {
            let ix = *ix;
            let name = base.kittens[ix].name.clone();
            let other = (ix + 1) % base.kittens.len();
            for (standing, elder_before) in
                [("nobody the Elder", None), ("another kitten the Elder", Some(other)), ("that kitten already the Elder", Some(ix))]
            {
                let whr = format!("in {layout_name}, with `n` pressed on kitten {ix}'s row and {standing} before the press");
                let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
                let mut a = app(scratch(&format!("appoint-note-{w}-{swap}-{ix}-{}", elder_before.map_or(99, |e| e))));
                a.state = base.clone();
                a.state.elder = elder_before;
                a.view.section = Some(Section::Clowder);
                a.view.focus[Section::Clowder.index()] = Some(row_key.clone());
                a.view.inspector_wanted = swap;
                a.step(&mut term, elapsed(0.0), &[]).expect("a first frame draws");

                // The control: this row's `Enter` is refused, and its note is
                // a refusal's — where it is drawn and how long it lasts.
                a.step(&mut term, elapsed(0.0), &[key(KeyCode::Enter)]).expect("a step with Enter does not fail");
                let asked = Instant::now();
                let (refusal, refusal_until) =
                    a.view.footer.clone().unwrap_or_else(|| panic!("fixture: {whr}, the row's Enter was not refused with a note"));
                let refusal_lasts = refusal_until.duration_since(asked);
                let hints = a.prev_layout.expect("a frame was drawn").hints;
                assert_eq!(
                    screen_row(&term, hints).trim_end(),
                    refusal,
                    "fixture: {whr}, a refusal's note is drawn on the frame's footer line"
                );
                assert_eq!(appointments_journalled(&a.state, &name), 0, "fixture: nothing appointed yet");
                if !swap && w < 100 && name == long {
                    assert!(
                        !screen(&term).contains(&name),
                        "fixture: {whr}, the roster's row shortens the long name {name:?}"
                    );
                    shortened_on_its_row += 1;
                }
                // The control leaves nothing behind: no note, and no repeat
                // gate still closed on the press under test.
                a.view.footer = None;
                a.view.last_repeat = None;

                a.step(&mut term, elapsed(0.0), &[key(KeyCode::Char('n'))]).expect("a step with `n` does not fail");
                let pressed = Instant::now();

                assert_eq!(a.state.elder, Some(ix), "fixture: {whr}, the core accepted the appointment");
                let (note, until) = a.view.footer.clone().unwrap_or_else(|| {
                    panic!(
                        "{whr}, the core accepted the appointment and the footer carries no note; an \
                         accepted appointment is answered where the player is looking (TUI-NAV-107)"
                    )
                });
                assert_ne!(note, refusal, "{whr}, the footer still carries the refusal's note");
                assert!(
                    note.contains(&name),
                    "{whr}, the note {note:?} does not name the Elder the core holds by its whole \
                     name {name:?} (TUI-NAV-107)"
                );
                assert!(
                    note.contains("Elder"),
                    "{whr}, the note {note:?} does not say in plain words that {name:?} is the Elder (TUI-NAV-107)"
                );
                assert!(!note.contains('\u{2026}'), "{whr}, the note {note:?} names the Elder by a shortened form");
                for (k, kitten) in a.state.kittens.iter().enumerate() {
                    if k != ix {
                        assert!(
                            !note.contains(kitten.name.as_str()),
                            "{whr}, the note {note:?} names kitten {k}, who is not the Elder the core holds"
                        );
                    }
                }

                // Lasting as a refusal's note does.
                let lasts = until.duration_since(pressed);
                let apart = lasts.abs_diff(refusal_lasts);
                assert!(
                    apart < Duration::from_millis(500),
                    "{whr}, the note lasts {lasts:?} where a refusal's lasts {refusal_lasts:?}; it \
                     lasts as such a note does (TUI-NAV-107)"
                );

                // On the footer line, with the ticker's line beside it and
                // the Journal's entry kept.
                let layout = a.prev_layout.expect("a frame was drawn");
                assert_eq!(layout.hints, hints, "fixture: {whr}, the footer line stands where it stood");
                let footer_line = screen_row(&term, layout.hints);
                if note.chars().count() <= layout.hints.width as usize {
                    assert_eq!(
                        footer_line.trim_end(),
                        note,
                        "{whr}, the note is not drawn on the footer line that carries a refusal's \
                         note (TUI-NAV-107)"
                    );
                } else {
                    let head: String = note.chars().take(layout.hints.width as usize / 2).collect();
                    assert!(footer_line.starts_with(&head), "{whr}, the note is not drawn on the footer line: {footer_line:?}");
                }
                let name_head: String = name.chars().take(12).collect();
                let ticker_line = screen_row(&term, layout.ticker);
                assert!(
                    ticker_line.contains(&name_head),
                    "{whr}, the ticker does not carry the appointment's line beside the footer's \
                     note (TUI-NAV-107, TUI-NAV-026): {ticker_line:?}"
                );
                assert_eq!(
                    appointments_journalled(&a.state, &name),
                    1,
                    "{whr}, the Journal does not keep the appointment's one entry beside the \
                     footer's note (TUI-NAV-107, TUI-NAV-026)"
                );
                checked += 1;
            }
        }
    }
    assert!(checked > 0, "fixture: appointments were checked");
    assert!(shortened_on_its_row > 0, "fixture: no kitten's name was shortened on its row");
}

/// Each further press of `n` the core accepts leaves the note anew — a held
/// key's presses included, each as the repeat gate lets it through, and a
/// press on the Elder's own row as much as any — always naming the Elder the
/// core then holds: the same kitten while the cursor stays, the new one once
/// it has moved to another kitten's row. A press the repeat gate swallows is
/// issued to nobody and answers nothing.
///
/// @spec TUI-NAV-107
#[test]
fn each_further_accepted_n_leaves_the_note_anew_naming_the_elder_the_core_then_holds() {
    use clowder_tui::nav::Section;
    use crossterm::event::KeyCode;

    let base = small_roster();
    let kittens = kitten_row_keys(&base);
    let (first, first_key) = kittens[0].clone();
    let (second, second_key) = kittens[1].clone();
    let name_of = |k: usize| base.kittens[k].name.clone();

    let mut term = Terminal::new(TestBackend::new(100, 30)).unwrap();
    let mut a = app(scratch("appoint-note-anew"));
    a.state = base.clone();
    a.view.section = Some(Section::Clowder);
    a.view.focus[Section::Clowder.index()] = Some(first_key);
    a.step(&mut term, elapsed(0.0), &[]).expect("a first frame draws");

    // A key held: press after press on one row, the second onward on the
    // Elder's own row, each let through by the repeat gate.
    let mut last_until: Option<Instant> = None;
    for press in 1..=4usize {
        std::thread::sleep(Duration::from_millis(120));
        a.step(&mut term, elapsed(0.0), &[key(KeyCode::Char('n'))]).expect("a step with `n` does not fail");
        assert_eq!(a.state.elder, Some(first), "fixture: press {press} appointed kitten {first}");
        assert_eq!(
            appointments_journalled(&a.state, &name_of(first)),
            press,
            "fixture: the core accepted press {press} of `n` on kitten {first}'s row"
        );
        let (note, until) = a.view.footer.clone().unwrap_or_else(|| {
            panic!("press {press} of `n` on kitten {first}'s row was accepted and left no footer note (TUI-NAV-107)")
        });
        assert!(
            note.contains(&name_of(first)) && !note.contains(&name_of(second)),
            "press {press} of `n` on kitten {first}'s row left the note {note:?}; it names the Elder \
             the core then holds, {:?} (TUI-NAV-107)",
            name_of(first)
        );
        if let Some(earlier) = last_until {
            assert!(
                until > earlier,
                "press {press} of `n`, accepted on the Elder's own row, did not leave the note \
                 anew: it still expires when the press before it set it to (TUI-NAV-107)"
            );
        }
        last_until = Some(until);
    }

    // Two presses inside the repeat gate: one is issued, one swallowed.
    std::thread::sleep(Duration::from_millis(120));
    let before = appointments_journalled(&a.state, &name_of(first));
    a.step(&mut term, elapsed(0.0), &[key(KeyCode::Char('n')), key(KeyCode::Char('n'))]).unwrap();
    assert_eq!(
        appointments_journalled(&a.state, &name_of(first)),
        before + 1,
        "fixture: of two presses inside the repeat gate, one is issued"
    );

    // The note gone — expired, or cleared — a further accepted press leaves it again.
    a.view.footer = None;
    std::thread::sleep(Duration::from_millis(120));
    a.step(&mut term, elapsed(0.0), &[key(KeyCode::Char('n'))]).unwrap();
    assert!(
        a.view.footer.as_ref().is_some_and(|(note, _)| note.contains(&name_of(first))),
        "with the earlier note gone, a further accepted press on the Elder's own row left no note \
         naming the Elder (TUI-NAV-107): {:?}",
        a.view.footer
    );

    // The cursor moved to another kitten's row: the note names the new Elder.
    a.view.focus[Section::Clowder.index()] = Some(second_key);
    std::thread::sleep(Duration::from_millis(120));
    a.step(&mut term, elapsed(0.0), &[key(KeyCode::Char('n'))]).unwrap();
    assert_eq!(a.state.elder, Some(second), "fixture: the core accepted the appointment of kitten {second}");
    let (note, _) = a.view.footer.clone().expect("the appointment of another kitten left no footer note (TUI-NAV-107)");
    assert!(
        note.contains(&name_of(second)) && !note.contains(&name_of(first)),
        "after kitten {second} was appointed, the note {note:?} does not name the Elder the core \
         then holds, {:?}, alone (TUI-NAV-107)",
        name_of(second)
    );
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

/// An appointment is the one accepted act the footer confirms: no other
/// command the core accepts leaves a footer note. Every selectable row of
/// every section is pressed with each key that acts on a row — Enter, `+`,
/// `-`, and each mnemonic letter — and a heavy row with the distinct second
/// Enter that commits it, each press in a session of its own; wherever the
/// core accepts the command the press issues and that command is not
/// AppointElder, the footer stands empty after the step. In the same meadow,
/// `n` on a kitten's row does leave its note.
///
/// @spec TUI-NAV-107
#[test]
fn no_accepted_command_but_an_appointment_leaves_a_footer_note() {
    use clowder_core::Command;
    use clowder_tui::nav::rows::rows;
    use clowder_tui::nav::{keys, update, Outcome, RowKey, Section};
    use crossterm::event::{KeyCode, KeyEvent};

    let base = everything_open();
    let presses = [
        KeyCode::Enter,
        KeyCode::Char('+'),
        KeyCode::Char('-'),
        KeyCode::Char('g'),
        KeyCode::Char('f'),
        KeyCode::Char('r'),
        KeyCode::Char('p'),
    ];

    // What a press would issue, asked of a copy of the view so the session's
    // own press still meets the view as it stood.
    let would_issue = |view: &ViewState, state: &GameState, code: KeyCode| -> Option<Command> {
        let mut shadow = view.clone();
        let action = keys::resolve(KeyEvent::from(code), &shadow);
        match update(action, &mut shadow, state) {
            Outcome::Apply(cmd) => Some(cmd),
            _ => None,
        }
    };

    let mut accepted_kinds: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut sections_accepting: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
    let (mut accepted, mut heavy_accepted, mut session) = (0u32, 0u32, 0u32);
    for section in Section::ALL {
        for row in rows(&base, section).into_iter().filter(|r| r.selectable()) {
            for code in presses {
                let mut view = ViewState { section: Some(section), ..ViewState::default() };
                view.focus[section.index()] = Some(row.key.clone());
                let mut issued = would_issue(&view, &base, code);
                // A heavy row's first Enter arms; a distinct second commits.
                let arms_first = issued.is_none() && row.heavy && code == KeyCode::Enter;
                if issued.is_none() && !arms_first {
                    continue;
                }

                session += 1;
                let mut term = Terminal::new(TestBackend::new(100, 30)).unwrap();
                let mut a = app(scratch(&format!("no-other-note-{session}")));
                a.state = base.clone();
                a.view = view;
                if arms_first {
                    a.step(&mut term, elapsed(0.0), &[key(code)]).unwrap();
                    if a.view.armed.is_none() {
                        continue;
                    }
                    a.view.footer = None;
                    std::thread::sleep(Duration::from_millis(120));
                    issued = would_issue(&a.view, &a.state, code);
                }
                let Some(cmd) = issued else { continue };
                if matches!(cmd, Command::AppointElder(_)) || a.state.clone().apply(cmd).is_err() {
                    continue;
                }

                a.step(&mut term, elapsed(0.0), &[key(code)]).expect("a step with a press does not fail");

                assert_eq!(
                    a.view.footer, None,
                    "in {}, {code:?} on the row {:?} issued {cmd:?}, which the core accepted, and \
                     the footer carries a note; an appointment is the one accepted act the footer \
                     confirms (TUI-NAV-107)",
                    section.title(),
                    row.key
                );
                accepted += 1;
                if row.heavy {
                    heavy_accepted += 1;
                }
                let kind = format!("{cmd:?}");
                accepted_kinds.insert(kind.split(['(', ' ', '{']).next().unwrap_or("").to_string());
                sections_accepting.insert(section.index());
            }
        }
    }
    assert!(accepted > 0, "fixture: the core accepted commands");
    assert!(heavy_accepted > 0, "fixture: the core accepted a heavy row's commit");
    assert!(
        accepted_kinds.len() >= 12 && sections_accepting.len() >= 6,
        "fixture: the core accepted commands of many kinds ({accepted_kinds:?}) from rows of many \
         sections ({} of them)",
        sections_accepting.len()
    );

    // The one that does: `n` on a kitten's row, in this very meadow.
    let (ix, row_key) = rows(&base, Section::Clowder)
        .into_iter()
        .find_map(|r| match r.key {
            RowKey::Kitten(ix, _) if base.elder != Some(ix) => Some((ix, r.key)),
            _ => None,
        })
        .expect("fixture: a kitten's row is listed");
    let mut term = Terminal::new(TestBackend::new(100, 30)).unwrap();
    let mut a = app(scratch("no-other-note-appointment"));
    a.state = base.clone();
    a.view.section = Some(Section::Clowder);
    a.view.focus[Section::Clowder.index()] = Some(row_key);
    a.step(&mut term, elapsed(0.0), &[]).expect("a first frame draws");
    a.step(&mut term, elapsed(0.0), &[key(KeyCode::Char('n'))]).expect("a step with `n` does not fail");
    assert_eq!(a.state.elder, Some(ix), "fixture: the core accepted the appointment");
    assert!(
        a.view.footer.as_ref().is_some_and(|(note, _)| note.contains(a.state.kittens[ix].name.as_str())),
        "in the meadow where no other accepted command left a footer note, an accepted appointment \
         left none either ({:?}); it is the one accepted act the footer confirms (TUI-NAV-107)",
        a.view.footer
    );
}
