//! How a long list scrolls as a sitting lives it: the list stands still while
//! the cursor moves within it, a change of direction moves no line of it, and
//! it moves only as the cursor reaches the line beside an edge. A list with no
//! line kept, or a cursor that has jumped, is drawn three lines down.
//!
//! @spec TUI-RENDER-009, TUI-RENDER-039, TUI-RENDER-025, TUI-NAV-130
//!
//! **Why through the loop.** Where a list begins depends on where the last
//! frame that drew it began, which the loop keeps from each frame's pointer
//! record. So every claim here is driven through `App::step`, frame after
//! frame, with the clock held still so nothing ticks beneath the test, and is
//! read off the frame's own pointer record: the rows it drew, each on its line.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use clowder_core::{
    content, ColonyId, ContentId, GameState, IdeaId, JobId, Kitten, KittenTrait, Location,
    ResourceId, StructureId,
};
use clowder_tui::nav::rows::{drawn_rows, RowKind};
use clowder_tui::nav::{opening, RowKey, Section, ViewState};
use clowder_tui::render::pointer::PointerRecord;
use clowder_tui::runtime::clock::Reading;
use clowder_tui::runtime::{App, TickAccumulator};
use crossterm::event::{
    Event as TermEvent, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind,
};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

const SIZES: [(u16, u16); 2] = [(80, 24), (160, 50)];

// ---------------------------------------------------------------------------
// A long list
// ---------------------------------------------------------------------------

fn kitten(name: &str) -> Kitten {
    Kitten {
        name: name.to_string(),
        job: Some(JobId::ALL[0]),
        location: Location::Home,
        arrived_day: 0,
        job_seq: None,
        away_seq: None,
        skills: [0.0; 8],
        exp: 0.0,
        rank: 0,
        trait_id: KittenTrait::None,
        tinkering: None,
        profile: None,
    }
}

/// A meadow far along, every structure and idea revealed, so the home
/// section's list runs to well over a hundred lines.
fn far_along() -> GameState {
    let mut s = GameState::new(7);
    s.ticks = 362 * content::TICKS_PER_DAY;
    for r in ResourceId::ALL {
        s.resources[r.ix()] = 1.0e9;
    }
    for b in StructureId::ALL {
        s.structures[b.ix()] = 2;
    }
    s.ideas = IdeaId::ALL.iter().copied().collect::<BTreeSet<_>>();
    s.revealed = StructureId::ALL
        .iter()
        .map(|b| ContentId::Structure(*b))
        .chain(IdeaId::ALL.iter().map(|i| ContentId::Idea(*i)))
        .collect::<BTreeSet<_>>();
    s.peak_devotion = 1.0e9;
    s.discovered = ColonyId::ALL.iter().copied().collect::<BTreeSet<_>>();
    s.generation = 1;
    s.kittens = (0..6).map(|i| kitten(&format!("resident-{i}"))).collect();
    s.roster_high_water = 100;
    s.tick();
    s
}

fn save_path(name: &str) -> PathBuf {
    std::env::temp_dir()
        .join(format!("clowder-scroll-{}-{name}", std::process::id()))
        .join("save.json")
}

/// A sitting opened on `state` exactly as start-up opens one.
fn opened(state: GameState, name: &str) -> App {
    let mut view = ViewState::default();
    opening(&mut view, &state);
    App {
        state,
        view,
        clock: TickAccumulator::default(),
        save_path: save_path(name),
        last_save: Instant::now(),
        prev_layout: None,
        pointer_record: None,
        quitting: false,
        save_failed: false,
    }
}

fn terminal((w, h): (u16, u16)) -> Terminal<TestBackend> {
    Terminal::new(TestBackend::new(w, h)).expect("TestBackend is infallible")
}

fn still() -> Reading {
    Reading {
        monotonic: Duration::ZERO,
        wall: Duration::ZERO,
    }
}

fn frame(app: &mut App, term: &mut Terminal<TestBackend>, input: &[TermEvent]) {
    app.step(term, still(), input)
        .expect("a step against a TestBackend does not fail");
}

fn press(code: KeyCode) -> TermEvent {
    TermEvent::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn wheel(kind: MouseEventKind, x: u16, y: u16) -> TermEvent {
    TermEvent::Mouse(MouseEvent {
        kind,
        column: x,
        row: y,
        modifiers: KeyModifiers::NONE,
    })
}

// ---------------------------------------------------------------------------
// Reading a frame
// ---------------------------------------------------------------------------

fn record(app: &App) -> &PointerRecord {
    app.pointer_record.as_ref().expect("a frame was drawn")
}

/// The list's space as the frame recorded it: its left column and its lines.
fn list_space(app: &App, (w, h): (u16, u16)) -> (u16, u16, u16) {
    let r = record(app);
    let mut cells = (0..h).flat_map(|y| (0..w).map(move |x| (x, y)));
    let (x, top) = cells
        .find(|(x, y)| r.list_at(*x, *y).is_some())
        .expect("the frame drew a list");
    let bottom = (top..h)
        .take_while(|y| r.list_at(x, *y).is_some())
        .last()
        .expect("the list has lines");
    (x, top, bottom)
}

/// The rows the frame drew, each with its line.
fn drawn(app: &App, size: (u16, u16)) -> Vec<(u16, RowKey)> {
    let (x, top, bottom) = list_space(app, size);
    let r = record(app);
    (top..=bottom)
        .filter_map(|y| r.row_at(x, y).map(|(_, k)| (y, k.clone())))
        .collect()
}

fn cursor_key(app: &App) -> RowKey {
    let section = app.view.section.expect("a section in focus");
    app.view.focus[section.index()]
        .clone()
        .expect("the cursor rests on a row")
}

/// The line the cursor's row is drawn on.
fn cursor_y(app: &App, size: (u16, u16)) -> u16 {
    let key = cursor_key(app);
    drawn(app, size)
        .into_iter()
        .find(|(_, k)| *k == key)
        .map(|(y, _)| y)
        .expect("the cursor's row is drawn")
}

/// The place of `key` in the focused section's list.
fn index_of(app: &App, key: &RowKey) -> usize {
    let section = app.view.section.expect("a section in focus");
    drawn_rows(&app.state, section, &app.view)
        .iter()
        .position(|r| &r.key == key)
        .expect("a listed row")
}

/// How many of the list's own lines the frame drew above the cursor's row:
/// its rows and headers from the line the list began on, never an overflow
/// note.
fn lines_before_cursor(app: &App) -> usize {
    let (_, line) = record(app)
        .list_line()
        .expect("the record names the line the list began on");
    index_of(app, &cursor_key(app)) - line.place
}

/// Walk the cursor down, a press a frame, until the list has scrolled and
/// the cursor stands at the line beside the bottom edge, then a few more.
fn scrolled_down(app: &mut App, term: &mut Terminal<TestBackend>, size: (u16, u16)) {
    frame(app, term, &[]);
    let first = drawn(app, size);
    let mut moves = 0;
    for n in 0.. {
        assert!(n < 400, "fixture: the list scrolls within the walk");
        frame(app, term, &[press(KeyCode::Down)]);
        if drawn(app, size) != first {
            moves += 1;
            if moves == 4 {
                break;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// TUI-RENDER-009: the list stands still until the cursor reaches its edge
// ---------------------------------------------------------------------------

/// After the list has scrolled down with the cursor, the first press back up
/// moves the cursor and no line of the list.
/// @spec TUI-RENDER-009
#[test]
fn a_change_of_direction_moves_the_cursor_and_not_the_list() {
    for size in SIZES {
        let mut app = opened(far_along(), "reverse");
        let mut term = terminal(size);
        scrolled_down(&mut app, &mut term, size);
        let before = drawn(&app, size);
        let y = cursor_y(&app, size);

        frame(&mut app, &mut term, &[press(KeyCode::Up)]);
        assert_eq!(
            drawn(&app, size),
            before,
            "at {size:?}, the first press back up scrolled the list"
        );
        assert!(
            cursor_y(&app, size) < y,
            "at {size:?}, the first press back up did not move the cursor up"
        );
    }
}

/// Walking back up, the list stands still the whole way to the line beside
/// its top edge, and only then moves, by no more than the lines a step passed.
/// @spec TUI-RENDER-009
#[test]
fn walking_back_the_list_stands_still_until_the_cursor_reaches_the_top() {
    for size in SIZES {
        let mut app = opened(far_along(), "walk-back");
        let mut term = terminal(size);
        scrolled_down(&mut app, &mut term, size);
        let (_, top, bottom) = list_space(&app, size);
        let body = usize::from(bottom - top + 1);
        let mut still_steps = 0;
        loop {
            let before = drawn(&app, size);
            let from = index_of(&app, &cursor_key(&app));
            let at_edge = lines_before_cursor(&app) <= 1;
            frame(&mut app, &mut term, &[press(KeyCode::Up)]);
            let after = drawn(&app, size);
            if after == before {
                still_steps += 1;
                continue;
            }
            assert!(
                at_edge,
                "at {size:?}, the list moved before the cursor reached the line beside its top"
            );
            let passed = from - index_of(&app, &cursor_key(&app));
            for (was, row) in &before {
                if let Some((now, _)) = after.iter().find(|(_, k)| k == row) {
                    assert!(
                        usize::from(now.abs_diff(*was)) <= passed,
                        "at {size:?}, {row:?} moved from line {was} to {now}, the step passing \
                         {passed}"
                    );
                }
            }
            break;
        }
        assert!(
            still_steps + 6 >= body,
            "at {size:?}, the cursor crossed only {still_steps} lines of a {body}-line pane before \
             the list moved; it should cross nearly all of it"
        );
    }
}

/// The wheel moves the cursor as the arrows do, so a change of the wheel's
/// direction too moves the cursor and not the list.
/// @spec TUI-RENDER-009, TUI-NAV-130
#[test]
fn a_change_of_the_wheels_direction_moves_the_cursor_and_not_the_list() {
    for size in SIZES {
        let mut app = opened(far_along(), "wheel");
        let mut term = terminal(size);
        frame(&mut app, &mut term, &[]);
        let (x, top, _) = list_space(&app, size);
        let mut moves = 0;
        let first = drawn(&app, size);
        for n in 0.. {
            assert!(n < 400, "fixture: the wheel scrolls the list");
            frame(
                &mut app,
                &mut term,
                &[wheel(MouseEventKind::ScrollDown, x, top + 2)],
            );
            if drawn(&app, size) != first {
                moves += 1;
                if moves == 4 {
                    break;
                }
            }
        }
        let before = drawn(&app, size);
        let y = cursor_y(&app, size);
        frame(
            &mut app,
            &mut term,
            &[wheel(MouseEventKind::ScrollUp, x, top + 2)],
        );
        assert_eq!(
            drawn(&app, size),
            before,
            "at {size:?}, the first wheel step back up scrolled the list"
        );
        assert!(cursor_y(&app, size) < y, "at {size:?}, the cursor moved up");
    }
}

/// The line is kept by its identity: a row revealed above the list moves no
/// row the frame draws.
/// @spec TUI-RENDER-009
#[test]
fn a_row_revealed_above_the_list_slides_nothing_in_view() {
    let size = SIZES[0];
    let mut state = far_along();
    let hidden = StructureId::ALL[1];
    state.revealed.remove(&ContentId::Structure(hidden));
    let mut app = opened(state, "identity");
    let mut term = terminal(size);
    scrolled_down(&mut app, &mut term, size);
    for _ in 0..30 {
        frame(&mut app, &mut term, &[press(KeyCode::Down)]);
    }
    let before = drawn(&app, size);
    let key = RowKey::Content(ContentId::Structure(hidden), "");
    assert!(
        before.iter().all(|(_, k)| *k != key),
        "fixture: the hidden structure would stand above the list"
    );
    let first_ix = index_of(&app, &before[0].1);

    app.state.revealed.insert(ContentId::Structure(hidden));
    frame(&mut app, &mut term, &[]);
    let list = drawn_rows(&app.state, Section::Meadow, &app.view);
    let now_at = list.iter().position(|r| r.key == key).expect("now listed");
    assert!(
        now_at < first_ix,
        "fixture: the reveal lands above the list"
    );
    assert_eq!(
        drawn(&app, size),
        before,
        "a row revealed above the list slid the rows in view"
    );
}

// ---------------------------------------------------------------------------
// TUI-RENDER-039: no line yet, or a jump
// ---------------------------------------------------------------------------

/// A list no frame of the sitting has drawn draws the cursor's row with three
/// of the list's lines before it.
/// @spec TUI-RENDER-039
#[test]
fn a_list_drawn_for_the_first_time_puts_the_cursor_three_lines_down() {
    for size in SIZES {
        let state = far_along();
        let mut app = opened(state, "first");
        let list = drawn_rows(&app.state, Section::Meadow, &app.view);
        let deep = list
            .iter()
            .filter(|r| r.kind != RowKind::Group)
            .nth(list.len() / 2)
            .expect("a deep row")
            .key
            .clone();
        app.view.focus[Section::Meadow.index()] = Some(deep);
        let mut term = terminal(size);
        frame(&mut app, &mut term, &[]);
        assert_eq!(
            lines_before_cursor(&app),
            3,
            "at {size:?}, a list drawn for the first time puts the cursor three lines down"
        );
    }
}

/// A cursor that has gone further from what the kept line shows than a step
/// can carry it is drawn three lines down; one that has gone a step past the
/// edge is drawn beside the edge, the list moving by the lines the step passed.
/// @spec TUI-RENDER-039, TUI-RENDER-009
#[test]
fn a_jump_puts_the_cursor_three_lines_down() {
    for size in SIZES {
        let mut app = opened(far_along(), "jump");
        let mut term = terminal(size);
        frame(&mut app, &mut term, &[]);
        let list = drawn_rows(&app.state, Section::Meadow, &app.view);
        let far = list
            .iter()
            .rev()
            .find(|r| r.kind != RowKind::Group)
            .expect("rows")
            .key
            .clone();
        app.view.focus[Section::Meadow.index()] = Some(far);
        frame(&mut app, &mut term, &[]);
        assert_eq!(
            lines_before_cursor(&app),
            3.min(index_of(&app, &cursor_key(&app))),
            "at {size:?}, a cursor jumped far down is drawn three lines down, as near as the \
             list's end allows"
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-RENDER-025: the record carries the line
// ---------------------------------------------------------------------------

/// The frame's pointer record names the line its list began on, by its
/// identity and its place, and the loop keeps it for that list alone; a frame
/// that draws no list keeps every line as it stood.
/// @spec TUI-RENDER-025, TUI-RENDER-009
#[test]
fn the_record_names_the_line_the_list_began_on_and_the_loop_keeps_it() {
    let size = SIZES[0];
    let mut app = opened(far_along(), "record");
    let mut term = terminal(size);
    assert!(
        app.view.list_line.iter().all(Option::is_none),
        "a sitting begins with no line kept"
    );
    scrolled_down(&mut app, &mut term, size);
    let (section, line) = record(&app).list_line().expect("a line");
    assert_eq!(section, Section::Meadow);
    let list = drawn_rows(&app.state, section, &app.view);
    assert_eq!(
        list[line.place].key, line.key,
        "the line's identity is the list's line at its place"
    );
    let first_row = &drawn(&app, size)[0];
    let first_row_ix = index_of(&app, &first_row.1);
    assert!(
        line.place <= first_row_ix && first_row_ix - line.place <= 1,
        "the line is the first the list drew: the first row, or a header just above it"
    );
    for s in Section::ALL {
        assert_eq!(
            app.view.list_line[s.index()].is_some(),
            s == Section::Meadow,
            "the loop keeps the line of the list it drew, and no other: {s:?}"
        );
    }
    assert_eq!(
        app.view.list_line[Section::Meadow.index()].as_ref(),
        Some(&line),
        "the loop keeps the record's line"
    );

    let kept = app.view.list_line.clone();
    frame(
        &mut app,
        &mut term,
        &[TermEvent::Key(KeyEvent::new(
            KeyCode::Char('e'),
            KeyModifiers::NONE,
        ))],
    );
    frame(&mut app, &mut term, &[press(KeyCode::Down)]);
    assert_eq!(
        app.view.list_line, kept,
        "a frame drawing the Journal over the list keeps every line as it stood"
    );
}
