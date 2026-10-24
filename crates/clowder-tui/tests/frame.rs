//! The frame itself: `render::frame`, which composes layout, theme and rows
//! into one drawing of a `GameState` snapshot.
//!
//! The thesis of this leaf is that **a frame is a pure function of (snapshot,
//! view state, terminal size)** — nothing cached between frames, no game value
//! mirrored shell-side, no decision depending on what was drawn last. The
//! design states outright why that property is worth paying for: it makes a
//! frame testable against an in-memory buffer at an exact size, which is what
//! turns every size-dependent claim from something someone remembers to check
//! into something a test asserts. So these tests take the design at its word
//! and *are* that in-memory buffer: a frame at 80×24 and a frame at 130×40 are
//! two `Buffer`s, and purity is the claim that the same inputs give the same
//! one.
//!
//! Three of this file's specs are only meaningful with their reasons attached,
//! and the tests are written to exercise the reasons rather than the return
//! values:
//!
//! * the scroll position is **derived** from the cursor each frame rather than
//!   stored, because a stored offset can disagree with the cursor after a
//!   reveal inserts rows or a resize changes the body height — so the tests
//!   drive exactly those two events and demand the cursor still be on screen;
//! * a group header **scrolls like any other line**, because pinning it costs a
//!   permanent body row for something `[` / `]` already provide — so the test
//!   scrolls past a header and demands it be gone, and demands that no line at
//!   all survives a scroll in place;
//! * the overflow indicator is **a note, not a scrollbar column**, because a
//!   column is expensive in a narrow pane — so the test asserts both that the
//!   note is there and that no column was spent.
//!
//! **How rows are identified in a buffer.** The renderer's exact row format is
//! its own business, but TUI-NAV-009 requires every actionable row to carry the
//! content's flavour name, and the core owns that name (`structure_def(..).name`).
//! So a row is located by looking for its name — via a prefix, so that a name
//! shortened by TUI-RENDER-014's ellipsis still matches. Rows whose label the
//! *shell* invents (resources, group headers, fixed actions) are never searched
//! for by a guessed string: the group header is found by rendering the list
//! unscrolled and reading the line at the header's own index in the row list.
//!
//! **What is normalised away.** Whether the list pane carries a border or a
//! title is not specified by anything in this leaf, so `content_lines` strips a
//! box-drawing frame if one is present and the tests locate the first drawn row
//! by name rather than assuming it lands on line zero. Nothing here should pass
//! or fail because of a border.
//!
//! `clowder-tui` is a binary with no library target, so the modules under test
//! are included by path. `render/mod.rs` reaches the view state through
//! `crate::nav`, so `nav` is declared at this test crate's root under its real
//! name — that is what makes the `crate::`-rooted path in the implementation
//! resolve without editing the implementation to suit the test. `theme` is
//! declared for the same reason, ahead of a frame that will need colour roles.

use clowder_tui::nav;
use clowder_tui::render;

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use clowder_core::content::structure_def;
use clowder_core::{
    save_string, ContentId, GameState, Kitten, KittenTrait, Location, ResourceId, StructureId,
};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::Terminal;

use nav::rows::{drawn_rows, rows, Row, RowKey, RowKind};
use nav::{Section, ViewState};
use render::layout::{compute, Body, Layout};

// ---------------------------------------------------------------------------
// Sizes
//
// The floor and the two breakpoints come from the specs. `WIDE` sits clear of
// both breakpoints and of any dead-band, so a layout is never in question in a
// test that is about something else; at that width the list pane is wide enough
// that a row's name is not at risk of being truncated, which is what keeps the
// name-matching below honest.
// ---------------------------------------------------------------------------

const FLOOR_W: u16 = 80;
const FLOOR_H: u16 = 24;

/// A comfortable three-pane terminal: past the 120-column cap, so the list has
/// its full capped width.
const WIDE: u16 = 130;
const TALL: u16 = 40;

/// Tall enough that even a long list fits with room to spare — the control for
/// every "when the list overflows" assertion.
const VERY_TALL: u16 = 100;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// A meadow with a chosen set of structures revealed and a chosen number of
/// resources in stock.
///
/// Both knobs move the Meadow section's row list: each revealed structure is a
/// row, and any resource with stock at all puts the stores group on screen with
/// a row per held resource. That is the only lever in the game that makes a
/// list far longer than a viewport without inventing content, which is exactly
/// what the viewport, the group header and the overflow indicator all need.
fn meadow(structures: &[StructureId], stocked: usize) -> GameState {
    let mut s = GameState::new(11);
    s.revealed = structures
        .iter()
        .map(|b| ContentId::Structure(*b))
        .collect::<BTreeSet<_>>();
    for r in ResourceId::ALL.into_iter().take(stocked) {
        s.resources[r.ix()] = 1_234.0;
    }
    s
}

/// Every structure revealed and every resource held: the longest list the game
/// can produce without a roster.
fn long_meadow() -> GameState {
    meadow(&StructureId::ALL, ResourceId::ALL.len())
}

/// A list that fits any viewport in this file with room to spare.
fn short_meadow() -> GameState {
    meadow(&StructureId::ALL[..3], 1)
}

/// The Meadow's rows for a state, as the shell will build them — the single
/// source both the cursor and the drawing read (TUI-NAV-009).
fn meadow_rows(state: &GameState) -> Vec<Row> {
    rows(state, Section::Meadow)
}

/// The index of a row in the Meadow's list, by identity.
fn row_index(state: &GameState, key: &RowKey) -> usize {
    meadow_rows(state)
        .iter()
        .position(|r| &r.key == key)
        .unwrap_or_else(|| panic!("fixture: {key:?} is not a row of this meadow"))
}

/// The index of the stores group header.
fn group_index(state: &GameState) -> usize {
    meadow_rows(state)
        .iter()
        .position(|r| matches!(r.kind, RowKind::Group))
        .expect("fixture: this meadow should have a group header")
}

/// A view focused on the Meadow with the cursor on one row.
fn looking_at(key: RowKey) -> ViewState {
    let mut v = ViewState {
        section: Some(Section::Meadow),
        ..Default::default()
    };
    v.focus[Section::Meadow.index()] = Some(key);
    v
}

fn structure_row(b: StructureId) -> RowKey {
    RowKey::Content(ContentId::Structure(b), "")
}

/// A roster of plain kittens. Their names are empty so that a long roster costs
/// as little as possible to *clone*, which keeps the frame-cost measurement
/// below measuring drawing rather than allocation.
fn roster(n: usize) -> Vec<Kitten> {
    (0..n)
        .map(|_| Kitten {
            name: String::new(),
            job: None,
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
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Drawing
// ---------------------------------------------------------------------------

/// One drawn frame: the cells, and the layout the frame reported using.
struct Drawn {
    buffer: Buffer,
    layout: Layout,
}

fn area(w: u16, h: u16) -> Rect {
    Rect::new(0, 0, w, h)
}

fn terminal(w: u16, h: u16) -> Terminal<TestBackend> {
    Terminal::new(TestBackend::new(w, h)).expect("TestBackend is infallible")
}

/// Draw one frame into an existing terminal, as the loop does — so a test can
/// ask what a frame does *after* other frames.
fn draw(
    term: &mut Terminal<TestBackend>,
    state: &GameState,
    view: &ViewState,
    prev: Option<&Layout>,
) -> Drawn {
    let mut layout: Option<Layout> = None;
    term.draw(|f| layout = Some(render::frame(f, state, view, prev)))
        .expect("TestBackend is infallible");
    Drawn {
        buffer: term.backend().buffer().clone(),
        layout: layout.expect("frame returns the layout it used"),
    }
}

/// Draw one frame into a terminal that has never drawn anything.
fn cold(state: &GameState, view: &ViewState, w: u16, h: u16) -> Drawn {
    let mut term = terminal(w, h);
    draw(&mut term, state, view, None)
}

// ---------------------------------------------------------------------------
// Reading a buffer
// ---------------------------------------------------------------------------

/// Glyphs a pane frame, a divider or a scrollbar could be drawn from.
const BOX_GLYPHS: &str = "─│┌┐└┘├┤┬┴┼━┃┏┓┗┛╭╮╰╯═║╔╗╚╝╌╎▏▎▍▌▋▊▉█▐░▒▓";

fn is_box(c: char) -> bool {
    BOX_GLYPHS.contains(c)
}

/// The text of a rectangle, one string per row, padded to the full width.
fn region_lines(buf: &Buffer, rect: Rect) -> Vec<String> {
    (rect.y..rect.bottom())
        .map(|y| {
            (rect.x..rect.right())
                .map(|x| buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "))
                .collect::<String>()
        })
        .collect()
}

/// The text of a rectangle with any box-drawing frame stripped off, so a test
/// about rows is not a test about whether the pane has a border.
fn content_lines(buf: &Buffer, rect: Rect) -> Vec<String> {
    let mut lines = region_lines(buf, rect);
    // A border line is box glyphs (and blanks) only; a group header's rule
    // carries its words (TUI-RENDER-023), so it is a line of the list, not a
    // frame around it.
    let bordered = |l: &String| {
        l.chars().next().is_some_and(is_box) && !l.chars().any(|c| c.is_alphanumeric())
    };
    if lines.first().is_some_and(bordered) {
        lines.remove(0);
    }
    if lines.last().is_some_and(bordered) {
        lines.pop();
    }
    if !lines.is_empty() && lines.iter().all(|l| l.chars().next().is_some_and(is_box)) {
        lines = lines.iter().map(|l| l.chars().skip(1).collect()).collect();
    }
    if !lines.is_empty() && lines.iter().all(|l| l.chars().last().is_some_and(is_box)) {
        lines = lines
            .iter()
            .map(|l| {
                let n = l.chars().count();
                l.chars().take(n.saturating_sub(1)).collect()
            })
            .collect();
    }
    lines
}

/// The pane the list is drawn into, whichever layout the width chose — with
/// its pane divider (TUI-RENDER-016) set aside, since the divider is inert
/// chrome on the pane's edge column, not a row of the list's own content.
fn list_rect(l: &Layout) -> Rect {
    match l.body {
        Body::Narrow { main, .. } => Rect::new(main.x + 1, main.y, main.width - 1, main.height),
        Body::Wide { list, .. } => Rect::new(list.x + 1, list.y, list.width - 2, list.height),
        Body::TooSmall(_) => panic!("this test needs a terminal at or above the 80x24 floor"),
    }
}

fn list_lines(d: &Drawn) -> Vec<String> {
    content_lines(&d.buffer, list_rect(&d.layout))
}

/// A prefix of a content name, long enough to be unique and short enough to
/// survive TUI-RENDER-014's ellipsis if the column ever gets tight.
fn needle(name: &str) -> String {
    name.chars().take(10).collect()
}

/// A structure's search key, checked for uniqueness so a hit can only be that
/// structure's row.
fn probe(b: StructureId) -> String {
    let n = needle(structure_def(b).name);
    let clashes = StructureId::ALL
        .into_iter()
        .filter(|o| *o != b && structure_def(*o).name.contains(&n))
        .count();
    assert_eq!(clashes, 0, "fixture: {n:?} does not identify one structure");
    n
}

fn line_of(lines: &[String], text: &str) -> Option<usize> {
    lines.iter().position(|l| l.contains(text))
}

fn shows(lines: &[String], text: &str) -> bool {
    line_of(lines, text).is_some()
}

fn drawn_anywhere(d: &Drawn, text: &str) -> bool {
    shows(&region_lines(&d.buffer, d.buffer.area), text)
}

/// Where the first drawn row lands inside the list pane.
///
/// A pane could carry a title line above its rows; nothing in this leaf says it
/// may not, and nothing here should depend on the answer. The offset is
/// measured from a row whose position in the row list is known, rather than
/// assumed to be zero.
fn row_offset(lines: &[String], anchor: &str, anchor_row: usize, top_row: usize) -> usize {
    let at = line_of(lines, anchor).unwrap_or_else(|| {
        panic!(
            "expected the row for {anchor:?} to be drawn in the list pane; \
             a row carries its flavour name (TUI-NAV-009). Pane read as:\n{}",
            lines.join("\n")
        )
    });
    let expected = anchor_row - top_row;
    assert!(
        at >= expected && at - expected <= 2,
        "the list's rows should start within a line or two of the top of the pane; \
         {anchor:?} is row {anchor_row} of a list scrolled to {top_row} but was drawn on line {at}"
    );
    at - expected
}

// ---------------------------------------------------------------------------
// TUI-RENDER-001 — a frame is a pure function of snapshot, view and size
// ---------------------------------------------------------------------------

/// The same inputs give the same frame. Not approximately: the same cells.
/// @spec TUI-RENDER-001
#[test]
fn the_same_inputs_draw_the_same_frame() {
    let state = long_meadow();
    let view = looking_at(structure_row(StructureId::ALL[20]));

    for (w, h) in [
        (FLOOR_W, FLOOR_H),
        (WIDE, TALL),
        (VERY_TALL + 20, VERY_TALL),
    ] {
        let first = cold(&state, &view, w, h);
        let second = cold(&state, &view, w, h);
        assert_eq!(
            first.buffer, second.buffer,
            "{w}x{h}: two frames from identical inputs must be identical"
        );
        assert_eq!(
            first.layout, second.layout,
            "{w}x{h}: and so must the layouts they used"
        );
    }
}

/// Nothing is cached between frames: a frame drawn after other frames is the
/// frame it would have been drawn cold. This is the property the whole leaf is
/// built on, and it is what leaves the dirty-frame optimisation available later
/// — "did the inputs change" is only a well-formed question if the output is a
/// function of the inputs.
/// @spec TUI-RENDER-001
#[test]
fn a_frame_does_not_depend_on_the_frames_before_it() {
    let quiet = short_meadow();
    let busy = long_meadow();
    let view = looking_at(structure_row(StructureId::ALL[1]));
    let deep = looking_at(structure_row(StructureId::ALL[40]));

    let reference = cold(&busy, &view, WIDE, TALL);

    // A terminal that has drawn several other frames, at other states and other
    // cursors, and then draws this one.
    let mut term = terminal(WIDE, TALL);
    let mut prev: Option<Layout> = None;
    for (s, v) in [(&quiet, &view), (&busy, &deep), (&quiet, &deep)] {
        prev = Some(draw(&mut term, s, v, prev.as_ref()).layout);
    }
    let after = draw(&mut term, &busy, &view, prev.as_ref());

    assert_eq!(
        after.buffer, reference.buffer,
        "a frame drawn after other frames must equal the same frame drawn cold — \
         anything else is state surviving between frames"
    );
}

/// A resize is just another size. A frame at 80×24 drawn on a terminal that was
/// 130×40 a moment ago is the frame a fresh 80×24 terminal draws.
/// @spec TUI-RENDER-001
#[test]
fn a_frame_after_a_resize_is_the_frame_that_size_always_draws() {
    let state = long_meadow();
    let view = looking_at(structure_row(StructureId::ALL[30]));

    let reference = cold(&state, &view, FLOOR_W, FLOOR_H);

    let mut term = terminal(WIDE, TALL);
    let wide = draw(&mut term, &state, &view, None);
    term.backend_mut().resize(FLOOR_W, FLOOR_H);
    term.clear().expect("TestBackend is infallible");
    let narrow = draw(&mut term, &state, &view, Some(&wide.layout));

    assert_eq!(
        narrow.buffer, reference.buffer,
        "the frame at a size must not remember the size before it"
    );
}

/// The frame reads the snapshot it is handed. No game value is mirrored
/// shell-side, so a state that has moved on draws differently with a view state
/// that has not changed at all — nothing had to tell the shell.
/// @spec TUI-RENDER-001
#[test]
fn the_frame_reads_the_snapshot_rather_than_a_shell_side_copy() {
    let view = looking_at(RowKey::Fixed("GatherCatnip"));

    let early = GameState::new(11);
    let mut later = GameState::new(11);
    later.ticks = 5_000_000;
    later.kittens = roster(12);
    for r in ResourceId::ALL {
        later.resources[r.ix()] = 4_321.0;
    }
    for b in StructureId::ALL {
        later.structures[b.ix()] = 3;
    }
    later.revealed = StructureId::ALL
        .iter()
        .map(|b| ContentId::Structure(*b))
        .collect();

    let a = cold(&early, &view, WIDE, TALL);
    let b = cold(&later, &view, WIDE, TALL);
    assert_ne!(
        a.buffer, b.buffer,
        "the same view state over two different snapshots must not draw the same frame; \
         a frame that cannot tell them apart is drawing from something other than the core"
    );

    // ...and the first snapshot still draws the first frame, so the difference
    // was the snapshot rather than the passage of frames.
    let again = cold(&early, &view, WIDE, TALL);
    assert_eq!(
        again.buffer, a.buffer,
        "the snapshot is the input; nothing else moved"
    );
}

/// A frame is a read. The core owns the game state and the shell owns the view
/// state, and drawing changes neither — which is what lets the loop draw as
/// often as it likes without the drawing being part of the simulation.
/// @spec TUI-RENDER-001
#[test]
fn drawing_a_frame_changes_neither_the_snapshot_nor_the_view() {
    let state = long_meadow();
    let view = looking_at(structure_row(StructureId::ALL[9]));

    let state_before = save_string(&state, 0);
    let view_before = format!("{view:?}");

    let mut term = terminal(WIDE, TALL);
    let mut prev: Option<Layout> = None;
    for _ in 0..3 {
        prev = Some(draw(&mut term, &state, &view, prev.as_ref()).layout);
    }

    assert_eq!(
        save_string(&state, 0),
        state_before,
        "the frame must not touch the snapshot"
    );
    assert_eq!(
        format!("{view:?}"),
        view_before,
        "the frame must not touch the view state"
    );
}

/// The layout escapes. The dead-band at each breakpoint is a function of the
/// *previous* frame's layout, so a frame that computed one and dropped it would
/// leave the next frame unable to tell which side of a boundary it was already
/// on — and the returned layout must be the one the frame actually used, not a
/// second guess.
/// @spec TUI-RENDER-001
#[test]
fn the_frame_returns_the_layout_it_drew() {
    let state = long_meadow();
    let view = looking_at(structure_row(StructureId::ALL[4]));

    for (w, h) in [
        (FLOOR_W, FLOOR_H),
        (99, 30),
        (100, 30),
        (119, 30),
        (WIDE, TALL),
        (200, 60),
    ] {
        let drawn = cold(&state, &view, w, h);
        assert_eq!(
            drawn.layout,
            compute(area(w, h), None, false),
            "{w}x{h}: the layout a frame returns must be the layout it laid the frame out with"
        );
    }

    // And the previous layout it is handed must be the one it passes on: a
    // width inside the dead-band lays out differently depending on which side
    // it was approached from, and the frame is the only thing carrying that.
    let entered = cold(&state, &view, 100, TALL);
    let mut term = terminal(99, TALL);
    let wobbled = draw(&mut term, &state, &view, Some(&entered.layout));
    assert_eq!(
        wobbled.layout,
        compute(area(99, TALL), Some(&entered.layout), false),
        "the frame must lay out from the previous layout it was given, not from the width alone"
    );
}

// ---------------------------------------------------------------------------
// TUI-RENDER-009 — the scroll position is derived from the cursor
// ---------------------------------------------------------------------------

/// However deep in the list the cursor sits, the row it is on is drawn. A list
/// of a hundred rows in a viewport of thirty-six is the ordinary case, not an
/// edge one.
/// @spec TUI-RENDER-009
#[test]
fn the_cursor_is_visible_however_deep_in_the_list_it_sits() {
    let state = long_meadow();

    for ix in [0usize, 1, 12, 25, 40, 52] {
        let b = StructureId::ALL[ix];
        let name = probe(b);
        let d = cold(&state, &looking_at(structure_row(b)), WIDE, TALL);
        let lines = list_lines(&d);
        assert!(
            shows(&lines, &name),
            "the cursor is on row {} of {} and its row was not drawn; the viewport is derived \
             from the cursor, so it cannot be anywhere else. Pane read as:\n{}",
            row_index(&state, &structure_row(b)),
            meadow_rows(&state).len(),
            lines.join("\n")
        );
    }
}

/// A margin above and below: the cursor is kept visible *with company*, never
/// pressed against an edge while the list continues past it. Both neighbours of
/// the focused row are drawn wherever the list has rows on both sides.
/// @spec TUI-RENDER-009
#[test]
fn the_rows_either_side_of_the_cursor_are_drawn_too() {
    let state = long_meadow();

    for ix in [10usize, 26, 44] {
        let (above, at, below) = (
            StructureId::ALL[ix - 1],
            StructureId::ALL[ix],
            StructureId::ALL[ix + 1],
        );
        let d = cold(&state, &looking_at(structure_row(at)), WIDE, TALL);
        let lines = list_lines(&d);

        assert!(shows(&lines, &probe(at)), "the focused row is drawn");
        assert!(
            shows(&lines, &probe(above)),
            "the row above the cursor is drawn: the scroll keeps a margin above"
        );
        assert!(
            shows(&lines, &probe(below)),
            "the row below the cursor is drawn: the scroll keeps a margin below"
        );
    }
}

/// The reason the offset is derived rather than stored, stated as a test: a
/// reveal inserts rows *above* the cursor between one frame and the next. A
/// stored offset would still be pointing where it was and would leave the
/// cursor off screen; a derived one cannot.
/// @spec TUI-RENDER-009
#[test]
fn a_reveal_that_inserts_rows_above_the_cursor_cannot_hide_it() {
    let last = *StructureId::ALL.last().expect("structures exist");
    let name = probe(last);
    let view = looking_at(structure_row(last));

    // Before: only a few structures revealed, so the focused row is the fourth
    // row of the list and the viewport is at the very top.
    let before = meadow(
        &[StructureId::ALL[0], StructureId::ALL[1], last],
        ResourceId::ALL.len(),
    );
    // After: fifty rows have been revealed above the very same focused row.
    let after = long_meadow();

    let inserted =
        row_index(&after, &structure_row(last)) - row_index(&before, &structure_row(last));
    assert!(
        inserted > 30,
        "fixture: the reveal should insert far more rows than a viewport holds"
    );

    let mut term = terminal(WIDE, TALL);
    let first = draw(&mut term, &before, &view, None);
    assert!(
        shows(&list_lines(&first), &name),
        "the cursor starts visible"
    );

    let second = draw(&mut term, &after, &view, Some(&first.layout));
    let lines = list_lines(&second);
    assert!(
        shows(&lines, &name),
        "{inserted} rows appeared above the cursor and the cursor's row went off screen; \
         a viewport derived from the cursor cannot disagree with it. Pane read as:\n{}",
        lines.join("\n")
    );
}

/// The other reason: a resize changes the body's height under a cursor that has
/// not moved. Every supported height must still show it.
/// @spec TUI-RENDER-009
#[test]
fn a_resize_that_changes_the_body_height_cannot_hide_the_cursor() {
    // A deep row with a short name: this is the one test that draws at the
    // 80-column floor, where the list pane is at its narrowest and a long name
    // is the likeliest thing to meet TUI-RENDER-014's ellipsis.
    let state = long_meadow();
    let b = StructureId::ALL[50];
    let name = probe(b);
    let view = looking_at(structure_row(b));

    let mut term = terminal(WIDE, VERY_TALL);
    let mut prev = Some(draw(&mut term, &state, &view, None).layout);

    for (w, h) in [
        (WIDE, TALL),
        (FLOOR_W, FLOOR_H),
        (WIDE, VERY_TALL),
        (FLOOR_W, 30),
    ] {
        term.backend_mut().resize(w, h);
        term.clear().expect("TestBackend is infallible");
        let d = draw(&mut term, &state, &view, prev.as_ref());
        let lines = list_lines(&d);
        assert!(
            shows(&lines, &name),
            "{w}x{h}: the body's height changed and the cursor's row is no longer drawn. \
             Pane read as:\n{}",
            lines.join("\n")
        );
        prev = Some(d.layout);
    }
}

/// Derived means derived from the cursor *alone*: how the cursor got there is
/// not an input. A viewport reached by walking the list is the viewport a cold
/// frame draws for the same cursor — which is only true if no offset was kept.
/// @spec TUI-RENDER-009
#[test]
fn the_scroll_position_depends_on_the_cursor_and_not_on_how_it_was_reached() {
    let state = long_meadow();
    let settled = structure_row(StructureId::ALL[33]);
    let reference = cold(&state, &looking_at(settled.clone()), WIDE, TALL);

    let mut term = terminal(WIDE, TALL);
    let mut prev: Option<Layout> = None;
    let walk = [0usize, 52, 4, 47, 20, 33];
    for ix in walk {
        let view = looking_at(structure_row(StructureId::ALL[ix]));
        prev = Some(draw(&mut term, &state, &view, prev.as_ref()).layout);
    }
    let arrived = draw(&mut term, &state, &looking_at(settled), prev.as_ref());

    assert_eq!(
        arrived.buffer, reference.buffer,
        "the same cursor must give the same viewport however it was reached"
    );
}

/// The viewport is a window that moves with the cursor: the rows far from it
/// are not merely off the bottom of the pane, they are not drawn anywhere.
/// @spec TUI-RENDER-009, TUI-RENDER-015
#[test]
fn only_the_rows_around_the_cursor_are_drawn() {
    let state = long_meadow();
    let deep = StructureId::ALL[52];
    let d = cold(&state, &looking_at(structure_row(deep)), WIDE, TALL);

    assert!(
        shows(&list_lines(&d), &probe(deep)),
        "the cursor's row is drawn"
    );

    for ix in 0..8 {
        let far = StructureId::ALL[ix];
        assert!(
            !drawn_anywhere(&d, &probe(far)),
            "row {ix} is far above the cursor's window and must not appear anywhere in the frame"
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-RENDER-010 — group headers scroll with the list
// ---------------------------------------------------------------------------

/// A group header is a line of the list, so it leaves the screen when the list
/// scrolls past it. Pinning it would cost a permanent row of the body for
/// something `[` and `]` already give the player.
///
/// The header's rendered text is not guessed: the list is drawn unscrolled and
/// the header is read off the line at its own index in the row list.
/// @spec TUI-RENDER-010
#[test]
fn a_group_header_scrolls_out_of_view_like_any_other_row() {
    // A short run of structures, then the group, then every held resource: the
    // header sits near the top of a list far longer than the viewport.
    let state = meadow(&StructureId::ALL[..3], ResourceId::ALL.len());
    let g = group_index(&state);
    let anchor = probe(StructureId::ALL[0]);
    let anchor_row = row_index(&state, &structure_row(StructureId::ALL[0]));

    let top = cold(
        &state,
        &looking_at(RowKey::Fixed("GatherCatnip")),
        WIDE,
        TALL,
    );
    let top_lines = list_lines(&top);
    let offset = row_offset(&top_lines, &anchor, anchor_row, 0);

    let header = top_lines[g + offset].trim().to_string();
    assert!(
        !header.is_empty(),
        "the group header should draw something on its line; read the pane as:\n{}",
        top_lines.join("\n")
    );
    assert_eq!(
        top_lines.iter().filter(|l| l.trim() == header).count(),
        1,
        "fixture: the header's line must be distinct enough to be recognised once it moves"
    );

    // Now put the cursor on the last row of the list. The header is dozens of
    // rows above the window.
    let rows = meadow_rows(&state);
    let last = rows.last().expect("the list has rows").key.clone();
    let bottom = cold(&state, &looking_at(last), WIDE, TALL);
    let bottom_lines = list_lines(&bottom);

    assert!(
        !bottom_lines.iter().any(|l| l.trim() == header),
        "the group header stayed on screen while the list scrolled {} rows past it; \
         headers scroll, they are not pinned. Pane read as:\n{}",
        rows.len() - bottom_lines.len(),
        bottom_lines.join("\n")
    );
}

/// Nothing at all is pinned inside the list. Between a viewport at the top of a
/// long list and one at its bottom the two windows share no rows, so no line of
/// the pane may survive in place.
/// @spec TUI-RENDER-010
#[test]
fn no_line_of_the_list_survives_a_scroll_in_place() {
    let state = long_meadow();
    let rows = meadow_rows(&state);
    let first = rows.first().expect("the list has rows").key.clone();
    let last = rows.last().expect("the list has rows").key.clone();

    let top = list_lines(&cold(&state, &looking_at(first), WIDE, TALL));
    let bottom = list_lines(&cold(&state, &looking_at(last), WIDE, TALL));
    assert_eq!(top.len(), bottom.len(), "the same pane, drawn twice");
    assert!(
        rows.len() > 2 * top.len(),
        "fixture: the two windows must not be able to overlap"
    );

    for (i, (a, b)) in top.iter().zip(bottom.iter()).enumerate() {
        if a.trim().is_empty() {
            continue;
        }
        assert_ne!(
            a.trim(),
            b.trim(),
            "line {i} of the list pane is the same at the top of a {}-row list as at its \
             bottom, so something is pinned there rather than scrolling",
            rows.len()
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-100, TUI-RENDER-023 — a group header's words, and its line
//
// A header is found without guessing its text: the cursor is put on the row
// directly beneath it in the row list (the first row of the group it heads,
// TUI-NAV-099), and the header is the pane line directly above the cursor's.
// The pane is read raw — `region_lines`, not `content_lines` — so nothing about
// a rule-shaped line is normalised away.
// ---------------------------------------------------------------------------

/// Every group of every section listed at once: every structure and idea, every
/// resource held, a roster, every rite the ceremonies depend on, the arch that
/// opens the jar and the promise, every colony, a visiting whale, and a meadow
/// that has Moved once.
fn every_group_listed() -> GameState {
    use clowder_core::content::MEADOW_MOVE_MIN_KITTENS;
    use clowder_core::{ArchId, ColonyId, CraftId, IdeaId, JobId, RiteId};

    let mut s = GameState::new(7);
    for r in ResourceId::ALL {
        s.resources[r.ix()] = 1.0e12;
    }
    for b in StructureId::ALL {
        s.structures[b.ix()] = 5;
    }
    s.ideas = IdeaId::ALL.iter().copied().collect::<BTreeSet<_>>();
    s.revealed = StructureId::ALL
        .iter()
        .map(|b| ContentId::Structure(*b))
        .chain(IdeaId::ALL.iter().map(|i| ContentId::Idea(*i)))
        .collect::<BTreeSet<_>>();
    let mut kittens = roster(MEADOW_MOVE_MIN_KITTENS + 2);
    for (i, k) in kittens.iter_mut().enumerate() {
        k.name = format!("kitten-{i}");
    }
    kittens[0].job = Some(JobId::Tinkerer);
    kittens[0].tinkering = Some(CraftId::ALL[0]);
    s.kittens = kittens;
    s.elder = Some(0);
    s.peak_devotion = 1.0e12;
    s.devotion = 1.0e6;
    s.serenity = 1.0e12;
    s.rites = [RiteId::TheReverie, RiteId::Enlightenment]
        .into_iter()
        .collect::<BTreeSet<_>>();
    s.arches_bought = [ArchId::EverdreamArch].into_iter().collect::<BTreeSet<_>>();
    s.discovered = ColonyId::ALL.iter().copied().collect::<BTreeSet<_>>();
    s.sites_reached = BTreeSet::new();
    s.whale_visit_years = 5;
    s.whale_contentment = 1;
    s.generation = 1;
    s.elder_wisdom = 1.0e9;
    s.warm_memories = 1.0e3;
    s
}

/// The words the design gives the header of the group `row` stands in: the
/// plain plural of what the row is about, `stores` for resources, and
/// `things to do` for a section's own verbs.
fn header_words_for(row: &Row) -> &'static str {
    match &row.key {
        RowKey::Fixed(_) => "things to do",
        RowKey::Content(ContentId::Structure(_), _) => "structures",
        RowKey::Content(ContentId::Job(_), _) => "jobs",
        RowKey::Content(ContentId::Idea(_), _) => "ideas",
        RowKey::Content(ContentId::Refinement(_), _) => "refinements",
        RowKey::Content(ContentId::Rule(_), _) => "rules",
        RowKey::Content(ContentId::Rite(_), _) => "rites",
        RowKey::Content(ContentId::Arch(_), _) => "arches",
        RowKey::Content(ContentId::Tradition(_), _) => "traditions",
        RowKey::Content(ContentId::DreamLoom(_), _) => "loom upgrades",
        RowKey::Content(ContentId::Craft(_), _) => "crafts",
        RowKey::Bench => "tinkerers",
        RowKey::Energy | RowKey::Mess => "energy & mess",
        RowKey::Resource(_) => "stores",
        RowKey::Kitten(..) => "kittens",
        RowKey::Colony(..) => "colonies",
        RowKey::Site(_) => "sites",
        RowKey::Adventure(_) => "adventures",
    }
}

/// Every (section, header words) the design gives a several-group section.
fn designed_headers() -> BTreeSet<(usize, &'static str)> {
    let table: [(Section, &[&str]); 10] = [
        (
            Section::Meadow,
            &["things to do", "structures", "energy & mess", "stores"],
        ),
        (Section::Clowder, &["jobs", "things to do", "kittens"]),
        (Section::Ideas, &["ideas", "refinements", "rules"]),
        (Section::Crafts, &["tinkerers", "crafts"]),
        (Section::Sunbeams, &["things to do", "rites"]),
        (Section::Wonders, &["things to do", "arches"]),
        (Section::Friends, &["colonies", "things to do"]),
        (Section::Sky, &["sites", "things to do"]),
        (Section::Dreams, &["things to do", "loom upgrades"]),
        (
            Section::Legacy,
            &["things to do", "traditions", "adventures"],
        ),
    ];
    table
        .iter()
        .flat_map(|(s, ws)| ws.iter().map(move |w| (s.index(), *w)))
        .collect()
}

/// One drawn group header: the pane line, cell by cell, with the width of the
/// list it was drawn in.
struct HeaderLine {
    text: String,
    cells: Vec<ratatui::buffer::Cell>,
}

/// Draw `section` with the cursor on `beneath` and return the pane line
/// directly above the cursor's — the header of the group `beneath` opens.
fn header_above(
    state: &GameState,
    section: Section,
    beneath: &RowKey,
    w: u16,
    h: u16,
) -> HeaderLine {
    let mut view = ViewState {
        section: Some(section),
        ..Default::default()
    };
    view.focus[section.index()] = Some(beneath.clone());
    // The rows beneath the Ideas section's done header are drawn only while
    // its group is open (TUI-NAV-113).
    unfold_focus(state, &mut view);
    let d = cold(state, &view, w, h);
    let rect = list_rect(&d.layout);
    let lines = region_lines(&d.buffer, rect);
    let at = lines
        .iter()
        .position(|l| l.starts_with('\u{25B8}'))
        .unwrap_or_else(|| {
            panic!(
                "the cursor's row is not drawn in {}'s pane:\n{}",
                section.title(),
                lines.join("\n")
            )
        });
    assert!(
        at > 0,
        "the cursor's row is the pane's first line; no header can stand above it"
    );
    let y = rect.y + (at - 1) as u16;
    let cells = (rect.x..rect.right())
        .map(|x| d.buffer.cell((x, y)).expect("inside the buffer").clone())
        .collect();
    HeaderLine {
        text: lines[at - 1].clone(),
        cells,
    }
}

/// Every header the row list carries for `state`, as (section, the row
/// directly beneath it).
fn headers_of(state: &GameState) -> Vec<(Section, Row)> {
    let mut out = Vec::new();
    for section in Section::ALL {
        let list = rows(state, section);
        for (i, row) in list.iter().enumerate() {
            if matches!(row.kind, RowKind::Group) {
                let next = list.get(i + 1).unwrap_or_else(|| {
                    panic!(
                        "{}'s header at line {i} stands above nothing",
                        section.title()
                    )
                });
                out.push((section, next.clone()));
            }
        }
    }
    out
}

/// The words a drawn header line carries: what stands between its leading
/// `── ` and the blank before its closing rule.
fn words_of(line: &str) -> String {
    line.strip_prefix("\u{2500}\u{2500} ")
        .unwrap_or(line)
        .trim_end()
        .trim_end_matches('\u{2500}')
        .trim_end()
        .to_string()
}

/// Each header is worded as the design names its group's kind: `things to do`
/// over a section's own verbs in every section that has them, `stores` over
/// resources, and the plain plural for every other kind — lowercase, carrying
/// no figure. Quantified over every header of every several-group section, in
/// meadows that between them list every group the design names, which the
/// test checks — the done header aside, which TUI-NAV-111 words.
///
/// @spec TUI-NAV-100
#[test]
fn every_group_header_is_worded_as_the_plain_plural_of_its_kind() {
    let mut seen: BTreeSet<(usize, &'static str)> = BTreeSet::new();

    // The every-group meadow has researched every idea, so its ideas group is
    // gathered into the done group; the part-done meadow lists it.
    for state in [every_group_listed(), ideas_part_done()] {
        for (section, beneath) in headers_of(&state) {
            // The Ideas section's done header is worded as TUI-NAV-111 has it,
            // and tested there; it is the one header above finished rows.
            if section == Section::Ideas && is_done(&state, &beneath) {
                continue;
            }
            let expected = header_words_for(&beneath);
            let line = header_above(&state, section, &beneath.key, WIDE, TALL);
            let words = words_of(&line.text);
            assert_eq!(
                words,
                expected,
                "{}'s header over {:?} reads {:?}; the design words it {expected:?}. Line: {:?}",
                section.title(),
                beneath.key,
                words,
                line.text
            );
            assert!(
                !words
                    .chars()
                    .any(|c| c.is_ascii_digit() || c.is_uppercase()),
                "{}'s header {words:?} carries a figure or a capital",
                section.title()
            );
            seen.insert((section.index(), expected));
        }
    }

    let missing: Vec<String> = designed_headers()
        .difference(&seen)
        .map(|(ix, w)| format!("{} — {w}", Section::ALL[*ix].title()))
        .collect();
    assert!(
        missing.is_empty(),
        "with every group listed, these designed headers are not drawn:\n  {}",
        missing.join("\n  ")
    );
}

/// A header is one line across the list's whole width — from its left edge
/// two `─`, a blank, the words, a blank, and `─` to its right edge — with no
/// cursor glyph, marker, count or cost, and every cell of it, the words
/// included, in `dim`. Quantified over every header of every section, at
/// widths spanning both the narrow and the wide layout and both sides of the
/// list's cap. At every width the frame can draw a list at, the list holds the
/// leading rule, the longest header's words, and a blank (checked below), so
/// the words are drawn whole at each of them; the cut TUI-RENDER-014 gives a
/// narrower list is not reachable through a frame.
///
/// @spec TUI-RENDER-023
#[test]
fn a_group_header_is_a_dim_labelled_rule_spanning_the_list() {
    let state = every_group_listed();
    let headers = headers_of(&state);
    let dim = clowder_tui::theme::Role::Dim.color();
    let longest = designed_headers()
        .iter()
        .map(|(_, w)| w.chars().count())
        .max()
        .unwrap_or(0);

    for w in [FLOOR_W, 85, 99, 100, 104, 119, 120, 121, WIDE, 160, 200] {
        for (section, beneath) in &headers {
            let line = header_above(&state, *section, &beneath.key, w, TALL);
            let width = line.cells.len();
            assert!(
                width > 3 + longest,
                "fixture: at {w} columns the list is {width} wide, too narrow to hold every header whole"
            );
            let words = words_of(&line.text);
            assert!(
                !words.is_empty(),
                "{}'s header at {w} columns carries no words: {:?}",
                section.title(),
                line.text
            );
            let n = words.chars().count();
            let expected = format!(
                "\u{2500}\u{2500} {words} {}",
                "\u{2500}".repeat(width.saturating_sub(3 + n + 1))
            );
            assert_eq!(
                line.text, expected,
                "{}'s header over {:?} at {w} columns ({width}-column list) is not a rule spanning \
                 the list with its words set after two rule glyphs",
                section.title(),
                beneath.key
            );
            for (x, cell) in line.cells.iter().enumerate() {
                assert_eq!(
                    cell.fg,
                    dim.expect("dim is a colour"),
                    "{}'s header at {w} columns: cell {x} ({:?}) is not drawn in dim",
                    section.title(),
                    cell.symbol()
                );
                assert!(
                    !cell.modifier.contains(ratatui::style::Modifier::REVERSED),
                    "{}'s header at {w} columns: cell {x} is reversed, as a focused row is",
                    section.title()
                );
            }
        }
    }
    assert!(
        headers.len() >= designed_headers().len(),
        "fixture: with every group listed, only {} header line(s) are listed; the design names {}, \
         so the claim is not quantified over every header",
        headers.len(),
        designed_headers().len()
    );
}

// ---------------------------------------------------------------------------
// TUI-RENDER-011 — say there is more, without spending a column
// ---------------------------------------------------------------------------

/// The word the design uses for the overflow note is "more" ("a short *n more
/// below*"). The spec says only that the frame indicates rows exist beyond the
/// visible range, so this is the one lexical assumption in the file.
fn overflow_note(lines: &[String]) -> Option<String> {
    lines
        .iter()
        .find(|l| l.to_lowercase().contains("more"))
        .map(|l| l.trim().to_string())
}

fn first_number(text: &str) -> Option<u64> {
    let digits: String = text
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

/// A list longer than its viewport says so, and one that fits says nothing —
/// a note that is always there is not a signal.
/// @spec TUI-RENDER-011
#[test]
fn a_list_longer_than_its_viewport_says_that_there_is_more() {
    let state = long_meadow();
    let view = looking_at(RowKey::Fixed("GatherCatnip"));

    let overflowing = list_lines(&cold(&state, &view, WIDE, TALL));
    assert!(
        meadow_rows(&state).len() > overflowing.len(),
        "fixture: this list must not fit"
    );
    assert!(
        overflow_note(&overflowing).is_some(),
        "a list of {} rows in a viewport of {} must say that more rows exist. Pane read as:\n{}",
        meadow_rows(&state).len(),
        overflowing.len(),
        overflowing.join("\n")
    );

    let small = short_meadow();
    let fitting = list_lines(&cold(&small, &view, WIDE, TALL));
    assert!(
        meadow_rows(&small).len() < fitting.len(),
        "fixture: this list must fit"
    );
    assert_eq!(
        overflow_note(&fitting),
        None,
        "a list that fits has nothing beyond the visible range to announce. Pane read as:\n{}",
        fitting.join("\n")
    );
}

/// The note counts. Two lists at the same size, one much longer than the other,
/// must not say the same thing — the point of "n more below" over a bare mark
/// is that the player learns how much list is left.
/// @spec TUI-RENDER-011
#[test]
fn the_note_says_how_many_rows_are_beyond_the_view() {
    let view = looking_at(RowKey::Fixed("GatherCatnip"));
    let longer = long_meadow();
    let shorter = meadow(&StructureId::ALL, 1);
    assert!(
        meadow_rows(&longer).len() > meadow_rows(&shorter).len(),
        "fixture: one list is longer"
    );

    let long_note = overflow_note(&list_lines(&cold(&longer, &view, WIDE, TALL)))
        .expect("a long list announces the rows beyond the view");
    let short_note = overflow_note(&list_lines(&cold(&shorter, &view, WIDE, TALL)))
        .expect("a list that overflows by less still announces it");

    let long_n = first_number(&long_note)
        .unwrap_or_else(|| panic!("the note should carry a count; it read {long_note:?}"));
    let short_n = first_number(&short_note)
        .unwrap_or_else(|| panic!("the note should carry a count; it read {short_note:?}"));

    assert!(
        long_n > short_n,
        "a list with {} rows hidden announced {long_n:?} and one with fewer announced \
         {short_n:?}; the count must track what is actually beyond the view",
        meadow_rows(&longer).len() - meadow_rows(&shorter).len()
    );
}

/// The indication costs a row, not a column. A column of scrollbar is harder to
/// read at a glance and more expensive in a narrow pane, so the rows must be
/// drawn across exactly the same columns whether or not the list overflows.
/// @spec TUI-RENDER-011
#[test]
fn the_indication_costs_a_row_and_not_a_column() {
    // One state, one width, one cursor: the only difference is whether the
    // viewport is tall enough to hold the list.
    let state = meadow(&StructureId::ALL, 1);
    let view = looking_at(RowKey::Fixed("GatherCatnip"));

    let overflows = cold(&state, &view, WIDE, TALL);
    let fits = cold(&state, &view, WIDE, VERY_TALL);
    let over_lines = list_lines(&overflows);
    let fit_lines = list_lines(&fits);

    assert!(
        meadow_rows(&state).len() > over_lines.len(),
        "fixture: this list overflows at {TALL}"
    );
    assert!(
        meadow_rows(&state).len() < fit_lines.len(),
        "fixture: it fits at {VERY_TALL}"
    );
    assert_eq!(
        list_rect(&overflows.layout).width,
        list_rect(&fits.layout).width,
        "fixture: the same width, so the same pane"
    );

    for i in 0..5 {
        assert_eq!(
            over_lines[i], fit_lines[i],
            "line {i} of the list is drawn differently once the list overflows; \
             the overflow indication must not take a column away from the rows"
        );
    }

    // The same comparison over the *raw* pane, borders and all: a scrollbar
    // drawn on the pane's edge would show up here as a track that only the
    // overflowing frame has, or as a thumb in a different place.
    let over_raw = region_lines(&overflows.buffer, list_rect(&overflows.layout));
    let fit_raw = region_lines(&fits.buffer, list_rect(&fits.layout));
    for i in 0..8 {
        assert_eq!(
            over_raw[i], fit_raw[i],
            "line {i} of the list pane differs between a list that overflows and one that fits, \
             once its borders are counted; a column has been spent on the overflow"
        );
    }

    assert_no_scrollbar_column(&overflows);
}

/// Glyphs a vertical rule or a scrollbar thumb could be drawn from.
const BAR_GLYPHS: &str = "│┃║▌▐█▓▒░▊▋";

/// A scrollbar is a column of bar glyphs running the height of the pane with a
/// thumb somewhere along it. A pane divider is the same glyph all the way down;
/// a scrollbar is not, and a scrollbar in the pane's interior is not a divider
/// at all.
fn assert_no_scrollbar_column(d: &Drawn) {
    let rect = list_rect(&d.layout);
    let lines = region_lines(&d.buffer, rect);
    let height = lines.len();
    assert!(
        height > 4,
        "fixture: a pane tall enough for the question to mean anything"
    );

    for x in 0..rect.width as usize {
        let column: Vec<char> = lines.iter().filter_map(|l| l.chars().nth(x)).collect();
        let bars = column.iter().filter(|c| BAR_GLYPHS.contains(**c)).count();
        if bars * 10 < height * 9 {
            continue;
        }
        let distinct: BTreeSet<char> = column
            .iter()
            .copied()
            .filter(|c| BAR_GLYPHS.contains(*c))
            .collect();
        assert!(
            distinct.len() < 2,
            "column {x} of the list pane is a run of {distinct:?} down its height — that is a \
             scrollbar with a thumb, and the design spends a row on a note rather than a column \
             on a bar"
        );
        assert!(
            x == 0 || x + 1 == rect.width as usize,
            "column {x} inside the list pane is a full-height rule; only the pane's own edges \
             may carry one"
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-RENDER-015 — a frame costs what is visible, not what exists
// ---------------------------------------------------------------------------

/// The cheapest a frame can be measured: the fastest of many draws, which
/// discards scheduler noise rather than averaging it in.
fn frame_cost(state: &GameState, view: &ViewState, w: u16, h: u16) -> Duration {
    let mut term = terminal(w, h);
    let mut prev: Option<Layout> = None;
    for _ in 0..10 {
        prev = Some(draw(&mut term, state, view, prev.as_ref()).layout);
    }

    let mut best = Duration::MAX;
    for _ in 0..80 {
        let start = Instant::now();
        prev = Some(draw(&mut term, state, view, prev.as_ref()).layout);
        best = best.min(start.elapsed());
    }
    best
}

/// A row that exists but is not on screen must cost far less than a row that is
/// drawn. That is the whole of TUI-RENDER-015: the frame's work is bounded by
/// the viewport, so a list a thousand rows long in a viewport of twenty is a
/// twenty-row frame.
///
/// The comparison is self-calibrating rather than measured against a magic
/// number of milliseconds. Two frames over *the same* snapshot — so the header,
/// the sidebar and every core query cost exactly the same in both — differ only
/// in which section is focused, and so only in how many rows the list has. The
/// price of one *drawn* row is measured from the same section at two viewport
/// heights. A frame that walked its whole list would price a hidden row at
/// roughly a drawn one; a frame bounded by the viewport prices it near zero.
/// @spec TUI-RENDER-015
#[test]
fn a_row_that_exists_but_is_not_drawn_costs_far_less_than_one_that_is() {
    let mut state = GameState::new(11);
    state.kittens = roster(600);

    let short_list = ViewState {
        section: Some(Section::Meadow),
        ..Default::default()
    };
    let long_list = ViewState {
        section: Some(Section::Clowder),
        ..Default::default()
    };

    let short_rows = rows(&state, Section::Meadow).len();
    let long_rows = rows(&state, Section::Clowder).len();
    assert!(
        long_rows > short_rows + 500,
        "fixture: one section must have hundreds of rows more than the other ({long_rows} vs \
         {short_rows})"
    );

    let short_at_floor = frame_cost(&state, &short_list, FLOOR_W, FLOOR_H);
    let long_at_floor = frame_cost(&state, &long_list, FLOOR_W, FLOOR_H);
    let long_when_tall = frame_cost(&state, &long_list, FLOOR_W, VERY_TALL);

    let extra_visible_rows = (VERY_TALL - FLOOR_H) as u32;
    assert!(
        long_when_tall > long_at_floor,
        "fixture: drawing {extra_visible_rows} more rows should cost something measurable \
         ({long_at_floor:?} at {FLOOR_H} rows, {long_when_tall:?} at {VERY_TALL})"
    );

    let per_drawn_row =
        (long_when_tall - long_at_floor).as_secs_f64() / f64::from(extra_visible_rows);
    let hidden_rows = (long_rows - short_rows) as f64;
    let per_hidden_row = (long_at_floor.saturating_sub(short_at_floor)).as_secs_f64() / hidden_rows;

    assert!(
        per_hidden_row * 4.0 < per_drawn_row,
        "a row that exists but is never drawn cost {:.3}µs against {:.3}µs for one that is \
         drawn — the frame is walking the whole list rather than the viewport \
         ({hidden_rows} hidden rows: {short_at_floor:?} vs {long_at_floor:?})",
        per_hidden_row * 1e6,
        per_drawn_row * 1e6,
    );
}

/// The same claim without a clock: a frame over a list of hundreds of rows draws
/// only the window, so the rows outside it are not on screen in any pane.
/// @spec TUI-RENDER-015
#[test]
fn a_frame_draws_no_more_rows_than_the_viewport_holds() {
    let state = long_meadow();
    let d = cold(
        &state,
        &looking_at(structure_row(StructureId::ALL[25])),
        WIDE,
        TALL,
    );
    let viewport = list_lines(&d).len();
    let all = meadow_rows(&state).len();
    assert!(all > viewport * 2, "fixture: far more rows exist than fit");

    let drawn = StructureId::ALL
        .into_iter()
        .filter(|b| drawn_anywhere(&d, &needle(structure_def(*b).name)))
        .count();

    assert!(
        drawn <= viewport,
        "{drawn} of {all} rows were drawn into a viewport of {viewport} lines; a frame's work \
         is bounded by what is visible, not by what exists"
    );
}

// ---------------------------------------------------------------------------
// TUI-RENDER-020 — a list row's count and cost fields
// ---------------------------------------------------------------------------

use std::collections::HashMap;

use clowder_core::content::MEADOW_MOVE_MIN_KITTENS;
use clowder_core::{ArchId, ColonyId, CraftId, IdeaId, JobId, LockReason, RiteId};
use clowder_tui::render::fmt;
use clowder_tui::render::widgets::inspector;

/// The list's space in a drawn frame: the list's rectangle — or the narrow
/// body's single pane — with its pane divider(s) (TUI-RENDER-016) set aside.
fn list_space(l: &Layout) -> Rect {
    match l.body {
        Body::Narrow { main, .. } => Rect::new(main.x + 1, main.y, main.width - 1, main.height),
        Body::Wide { list, .. } => Rect::new(list.x + 1, list.y, list.width - 2, list.height),
        Body::TooSmall(r) => panic!("fixture: {r:?} is below the floor"),
    }
}

/// A rectangle's cells, one string of single-cell symbols per line.
fn cell_lines(buf: &Buffer, rect: Rect) -> Vec<Vec<String>> {
    (rect.y..rect.bottom())
        .map(|y| {
            (rect.x..rect.right())
                .map(|x| {
                    buf.cell((x, y))
                        .map(|c| c.symbol().to_string())
                        .unwrap_or_else(|| " ".into())
                })
                .collect()
        })
        .collect()
}

fn cells_text(cells: &[String], from: usize, to: usize) -> String {
    cells[from..to].concat()
}

/// The label a row draws in its name's place, where it is a later row of an
/// item that carries several (TUI-NAV-089): keyed by the row's own tag, and
/// only where an earlier row of the same item is listed before it.
fn later_row_label(state: &GameState, section: Section, key: &RowKey) -> Option<&'static str> {
    let label = match key {
        RowKey::Content(ContentId::Craft(_), "tinker") => "tinkerers",
        RowKey::Kitten(_, "elder") => "appoint Elder",
        RowKey::Colony(_, "penpal") => "send a pen-pal",
        _ => return None,
    };
    let same_item = |k: &RowKey| match (k, key) {
        (RowKey::Content(a, _), RowKey::Content(b, _)) => a == b,
        (RowKey::Kitten(a, _), RowKey::Kitten(b, _)) => a == b,
        (RowKey::Colony(a, _), RowKey::Colony(b, _)) => a == b,
        _ => false,
    };
    rows(state, section)
        .iter()
        .find(|r| same_item(&r.key))
        .filter(|first| &first.key != key)
        .map(|_| label)
}

/// What a row draws in its name's place, in full: on a later row of an item
/// that carries several, that row's label (TUI-NAV-089); on any other row, the
/// first line of the inspector's description of it (TUI-NAV-018), drawn wide
/// enough that nothing is cut.
fn inspector_name(state: &GameState, section: Section, key: &RowKey) -> String {
    if let Some(label) = later_row_label(state, section, key) {
        return label.to_string();
    }
    let (w, h) = (180u16, 6u16);
    let mut view = ViewState {
        section: Some(section),
        ..Default::default()
    };
    view.focus[section.index()] = Some(key.clone());
    unfold_focus(state, &mut view);
    let mut term = terminal(w, h);
    term.draw(|f| inspector::draw(f, area(w, h), state, &view))
        .expect("TestBackend is infallible");
    cell_lines(term.backend().buffer(), area(w, h))[0]
        .concat()
        .trim()
        .to_string()
}

/// Every resource's stores-row spelling, read from the inspector over its row
/// in the stores group.
fn store_names() -> HashMap<ResourceId, String> {
    let mut s = GameState::new(11);
    for r in ResourceId::ALL {
        s.resources[r.ix()] = 1.0;
    }
    ResourceId::ALL
        .into_iter()
        .map(|r| (r, inspector_name(&s, Section::Meadow, &RowKey::Resource(r))))
        .collect()
}

/// The cost field's width: six columns of figure, a space, the widest
/// stores-row name in the resource vocabulary, and three columns for `, …`.
fn cost_field_width(names: &HashMap<ResourceId, String>) -> usize {
    6 + 1
        + names
            .values()
            .map(|n| n.chars().count())
            .max()
            .expect("a vocabulary")
        + 3
}

/// Whether a section can list a row whose description states an owned count
/// (TUI-NAV-018): the copies of a structure or a rite, the kittens on a job's
/// count row, the tinkerers on a craft's tinkerer row. Judged by the section
/// alone, from what its rows are — checked against real lists below.
fn gives_count_field(section: Section) -> bool {
    matches!(
        section,
        Section::Meadow | Section::Clowder | Section::Crafts | Section::Sunbeams
    )
}

/// Whether a section can list a row whose description states a next cost
/// (TUI-NAV-078, TUI-NAV-073). Judged by the section alone.
fn gives_cost_field(section: Section) -> bool {
    !matches!(section, Section::Sky | Section::Legacy)
}

/// The owned count the inspector states for a row, if it states one.
fn stated_count(state: &GameState, row: &Row) -> Option<u64> {
    match &row.key {
        RowKey::Content(ContentId::Structure(b), _) => Some(state.structures[b.ix()] as u64),
        RowKey::Content(ContentId::Rite(r), _) => Some(state.rite_counts[r.ix()] as u64),
        RowKey::Content(ContentId::Job(j), _) if matches!(row.kind, RowKind::Count { .. }) => {
            Some(state.effective_workers(*j) as u64)
        }
        RowKey::Content(ContentId::Craft(c), "tinker") => Some(
            state
                .kittens
                .iter()
                .filter(|k| k.tinkering == Some(*c) && k.location == Location::Home)
                .count() as u64,
        ),
        _ => None,
    }
}

/// The next cost the inspector states for a row, if it states one: the core's
/// price for one more on a row that takes a content item — none on a
/// tinkerer row, where the item is already the player's, or where the price
/// names no component — and the one-unit price on a Pounce or Trade row.
fn stated_cost(state: &GameState, row: &Row) -> Option<Vec<(ResourceId, f64)>> {
    let cost = match (&row.key, &row.kind) {
        (RowKey::Content(_, "tinker"), _) => return None,
        (RowKey::Content(id, _), _) => {
            if state.lock_reason(*id) == Some(LockReason::AlreadyYours) {
                return None;
            }
            state.cost_of_next(*id, 1)
        }
        (_, RowKind::Act { command, .. }) => state.unit_price(command)?,
        _ => return None,
    };
    (!cost.is_empty()).then_some(cost)
}

/// The count field as TUI-RENDER-020 draws it: `×` joined to a compact
/// figure, right-aligned together in seven columns, blank for zero or where
/// nothing is stated.
fn count_field(count: Option<u64>) -> String {
    match count {
        Some(n) if n > 0 => format!("{:>7}", format!("×{}", fmt::compact(n as f64))),
        _ => " ".repeat(7),
    }
}

/// The cost field as TUI-RENDER-020 draws it, `width` columns: the first
/// figure right-aligned in six, then each component a compact figure, a space,
/// and its stores-row name, apart by `, ` — the longest leading run that fits,
/// with `, …` after it where any is left out.
fn cost_field(
    cost: Option<&[(ResourceId, f64)]>,
    names: &HashMap<ResourceId, String>,
    width: usize,
) -> (String, bool) {
    let Some(cost) = cost else {
        return (" ".repeat(width), false);
    };
    let parts: Vec<String> = cost
        .iter()
        .enumerate()
        .map(|(i, (r, a))| {
            if i == 0 {
                format!("{:>6} {}", fmt::compact(*a), names[r])
            } else {
                format!("{} {}", fmt::compact(*a), names[r])
            }
        })
        .collect();
    let mut text = String::new();
    let mut cut = false;
    for k in (1..=parts.len()).rev() {
        let run = parts[..k].join(", ");
        let tail = if k < parts.len() { ", …" } else { "" };
        if run.chars().count() + tail.chars().count() <= width {
            cut = k < parts.len();
            text = format!("{run}{tail}");
            break;
        }
    }
    assert!(
        !text.is_empty(),
        "the field holds the widest single component and `, …` after it"
    );
    (format!("{text:<width$}"), cut)
}

fn kitten_named(name: &str, job: Option<JobId>) -> Kitten {
    Kitten {
        name: name.to_string(),
        job,
        location: Location::Home,
        arrived_day: 0,
        job_seq: job.map(|_| 1),
        away_seq: None,
        skills: [0.0; 8],
        exp: 1.0e6,
        rank: 0,
        trait_id: KittenTrait::None,
        tinkering: None,
        profile: None,
    }
}

/// Every section's surfaces open, everything affordable, every idea held.
fn rich_meadow() -> GameState {
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
    s.roster_high_water = MEADOW_MOVE_MIN_KITTENS + 9;
    let mut roster: Vec<Kitten> = (0..MEADOW_MOVE_MIN_KITTENS + 9)
        .map(|i| kitten_named(&format!("kitten-{i}"), None))
        .collect();
    for k in roster.iter_mut().take(4) {
        k.job = Some(JobId::Tinkerer);
    }
    roster[0].tinkering = Some(CraftId::ALL[0]);
    s.kittens = roster;
    s.elder = Some(0);
    s.peak_devotion = 1.0e12;
    s.devotion = 1.0e6;
    s.serenity = 1.0e12;
    s.rites = [RiteId::TheReverie, RiteId::Enlightenment]
        .into_iter()
        .collect();
    s.rite_counts[RiteId::TheReverie.ix()] = 2;
    s.arches_bought = [ArchId::EverdreamArch].into_iter().collect();
    s.discovered = ColonyId::ALL.iter().copied().collect();
    s.whale_visit_years = 5;
    s.whale_contentment = 1;
    s.generation = 1;
    s.elder_wisdom = 1.0e9;
    s.warm_memories = 1.0e3;
    s
}

/// The same surfaces with nothing in the stores and no idea held: every price
/// stated, none met.
fn poor_meadow() -> GameState {
    let mut s = rich_meadow();
    for r in ResourceId::ALL {
        s.resources[r.ix()] = 0.0;
    }
    s.ideas.clear();
    s
}

/// Counts and stocks at every width the compact form takes: structures from
/// none to thousands, stores from a trickle to past the last marker.
fn ragged_meadow() -> GameState {
    let mut s = rich_meadow();
    let counts = [0u32, 1, 7, 42, 999, 1000, 12_345, 4_000_000_000];
    for (i, b) in StructureId::ALL.into_iter().enumerate() {
        s.structures[b.ix()] = counts[i % counts.len()];
    }
    let stocks = [0.0004, 3.0, 12.5, 1234.5, 1.0e7, 1.0e20];
    for (i, r) in ResourceId::ALL.into_iter().enumerate() {
        s.resources[r.ix()] = stocks[i % stocks.len()];
    }
    s
}

/// Every list row draws its owned count and next cost in fields of fixed
/// width at the row's right, after its name, a blank column before each: the
/// count as `×` joined to a compact figure right-aligned in seven columns,
/// blank for zero; the cost as its components in the core's order, compact
/// figure, space, stores-row name, apart by `, `, the first figure
/// right-aligned in the field's first six columns, and the longest leading
/// run of whole components that fits followed by `, …` where any is left
/// out. Each field draws exactly what the inspector's description states,
/// blank where it states nothing, and has width only where its section can
/// list a row stating that heading — judged by the section alone, so the
/// columns are the same whatever is listed, whatever the figures, and wherever
/// the list is scrolled; the name takes the width the fields leave.
/// Quantified over every section, over meadows rich, poor, ragged, and new,
/// at the floor, narrow, three-pane, capped, and wide, scrolled to the top
/// and the bottom.
/// @spec TUI-RENDER-020, TUI-RENDER-012
#[test]
fn every_list_row_draws_its_count_and_cost_in_fixed_fields_at_its_right() {
    let names = store_names();
    let c = cost_field_width(&names);
    let meadows = [
        rich_meadow(),
        poor_meadow(),
        ragged_meadow(),
        GameState::new(11),
    ];

    // The per-section table is the section's own: check it against what the
    // sections really list, over every meadow here.
    for section in Section::ALL {
        let lists_count = meadows.iter().any(|s| {
            rows(s, section)
                .iter()
                .any(|r| stated_count(s, r).is_some())
        });
        let lists_cost = meadows
            .iter()
            .any(|s| rows(s, section).iter().any(|r| stated_cost(s, r).is_some()));
        assert_eq!(
            lists_count,
            gives_count_field(section),
            "fixture: {section:?} and owned counts"
        );
        assert_eq!(
            lists_cost,
            gives_cost_field(section),
            "fixture: {section:?} and next costs"
        );
    }

    let mut seen = (0usize, 0usize, 0usize, 0usize); // rows, cut costs, marked counts, blank zero counts
    let mut name_cache: HashMap<(usize, Section, String), String> = HashMap::new();
    for (mi, state) in meadows.iter().enumerate() {
        for section in Section::ALL {
            let all = rows(state, section);
            let selectable: Vec<usize> = (0..all.len()).filter(|i| all[*i].selectable()).collect();
            let (Some(&first), Some(&last)) = (selectable.first(), selectable.last()) else {
                continue;
            };
            for (w, h) in [
                (FLOOR_W, FLOOR_H),
                (99, 30),
                (100, 30),
                (130, TALL),
                (200, 60),
            ] {
                for cursor in [first, last] {
                    let mut view = ViewState {
                        section: Some(section),
                        ..Default::default()
                    };
                    // Every listed row is drawn, in list order, only with the
                    // Ideas section's done group open (TUI-NAV-113).
                    if drawn_rows(state, section, &view) != all {
                        press_done_key(state, &mut view);
                    }
                    view.focus[section.index()] = Some(all[cursor].key.clone());
                    let d = cold(state, &view, w, h);
                    let space = list_space(&d.layout);
                    let width = space.width as usize;
                    let lines = cell_lines(&d.buffer, space);
                    let cy = lines
                        .iter()
                        .position(|l| l[0] == "▸")
                        .unwrap_or_else(|| panic!("fixture: the cursor is drawn in {section:?}"));

                    let counted = gives_count_field(section);
                    let costed = gives_cost_field(section);
                    let name_w =
                        width - 4 - if counted { 8 } else { 0 } - if costed { c + 1 } else { 0 };
                    let count_at = if costed { width - c - 8 } else { width - 7 };

                    for (y, line) in lines.iter().enumerate() {
                        if line[0].chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
                            continue; // an overflow note (TUI-RENDER-011)
                        }
                        let Some(ix) = (cursor + y).checked_sub(cy).filter(|ix| *ix < all.len())
                        else {
                            continue;
                        };
                        let row = &all[ix];
                        if matches!(row.kind, RowKind::Group) {
                            continue;
                        }
                        seen.0 += 1;
                        let key_text = format!("{:?}", row.key);
                        let full = name_cache
                            .entry((mi, section, key_text.clone()))
                            .or_insert_with(|| inspector_name(state, section, &row.key))
                            .clone();
                        let at = format!("{section:?} row {key_text} at {w}x{h}");
                        let drawn = line.concat();

                        // A kitten's row may carry, after its name where the
                        // column has room, the whole `(n: Elder)` hint
                        // (TUI-NAV-095) or, on the Elder's own row, the
                        // Elder's mark as far as the column holds it
                        // (TUI-NAV-104, TUI-NAV-106) — the dedicated sweeps
                        // over every width, gate state, and Elder are
                        // `every_kitten_row_but_the_elders_carries_the_whole_elder_hint_where_it_fits`
                        // and
                        // `the_elders_row_keeps_its_name_and_draws_the_first_step_of_the_mark_its_column_holds`;
                        // here they are only accounted for so this generic
                        // field check still holds over the name column they
                        // share.
                        let expected_column = match row.key {
                            RowKey::Kitten(kix, _) => {
                                assert_eq!(
                                    full, state.kittens[kix].name,
                                    "fixture: {at} is named by its kitten"
                                );
                                expected_name_column(state, kix, name_w)
                            }
                            _ => {
                                let name = fmt::fit(&full, name_w);
                                format!("{name:<name_w$}")
                            }
                        };
                        assert_eq!(
                            cells_text(line, 4, 4 + name_w),
                            expected_column,
                            "{at}: the name takes the {name_w} columns the fields leave, at one \
                             column on every row (TUI-RENDER-020).\n{drawn:?}"
                        );
                        if counted || costed {
                            assert_eq!(
                                line[4 + name_w],
                                " ",
                                "{at}: a blank column before the first field.\n{drawn:?}"
                            );
                        }
                        if matches!(
                            row.key,
                            RowKey::Kitten(..) | RowKey::Bench | RowKey::Energy | RowKey::Mess
                        ) {
                            // A kitten's row states neither heading; its
                            // fields' place carries the kitten's state
                            // instead, swept over every state, width, and
                            // cursor by
                            // `every_kitten_row_names_its_one_state_whole_in_its_fields_place`
                            // (TUI-RENDER-024). The bench's carries its
                            // seats, swept in `bench.rs` (TUI-RENDER-028).
                            continue;
                        }
                        if counted {
                            let count = stated_count(state, row);
                            if count == Some(0) {
                                seen.3 += 1;
                            }
                            if count.is_some_and(|n| n >= 1000) {
                                seen.2 += 1;
                            }
                            assert_eq!(
                                cells_text(line, count_at, count_at + 7),
                                count_field(count),
                                "{at}: the count field — what the inspector states as owned, `×` \
                                 joined to a compact figure right-aligned in seven columns, blank \
                                 for zero or for nothing stated (TUI-RENDER-020).\n{drawn:?}"
                            );
                            if costed {
                                assert_eq!(
                                    line[count_at + 7],
                                    " ",
                                    "{at}: a blank column before the cost field.\n{drawn:?}"
                                );
                            }
                        }
                        if costed {
                            let cost = stated_cost(state, row);
                            let (expected, cut) = cost_field(cost.as_deref(), &names, c);
                            if cut {
                                seen.1 += 1;
                            }
                            assert_eq!(
                                cells_text(line, width - c, width),
                                expected,
                                "{at}: the cost field — the inspector's next cost, compact figures \
                                 and stores-row names apart by `, `, the first figure right-aligned \
                                 in six columns, whole components then `, …` where any is left \
                                 out, in the {c}-column field at the row's right \
                                 (TUI-RENDER-020).\n{drawn:?}"
                            );
                        }
                    }
                }
            }
        }
    }
    assert!(seen.0 > 0, "fixture: rows were checked");
    assert!(
        seen.1 > 0,
        "fixture: some cost too long for its field was drawn"
    );
    assert!(seen.2 > 0, "fixture: some count past a thousand was drawn");
    assert!(seen.3 > 0, "fixture: some zero count was drawn");
}

// ---------------------------------------------------------------------------
// TUI-NAV-095 — the `(n: Elder)` hint on a kitten's row
// ---------------------------------------------------------------------------

/// The hint a kitten's row carries while the row-bound key is live there.
const ELDER_HINT: &str = "(n: Elder)";

/// `rich_meadow`'s open surfaces with a roster of exactly these names, nobody
/// yet the Elder.
fn clowder_named(names: &[String]) -> GameState {
    let mut s = rich_meadow();
    s.kittens = names.iter().map(|n| kitten_named(n, None)).collect();
    s.elder = None;
    s
}

/// A name of exactly `len` letters, headed by `head` so no two names made
/// here share a prefix a list could confuse.
fn name_of_len(head: char, len: usize) -> String {
    let mut n = String::from(head);
    n.push_str(&"o".repeat(len.saturating_sub(1)));
    n
}

/// Wherever the core reports the gate of AppointElder open for a listed
/// kitten who is not the one the core holds as the Elder, that kitten's row —
/// the cursor's and every other alike — carries `(n: Elder)` after the name,
/// within the name's column, in the dim role, of single-width characters, and
/// never in the count or cost field. The hint is drawn whole or not at all and
/// never costs the name a character: where the name's column holds the whole
/// name, a blank, and the whole hint, the hint is there; where it cannot hold
/// the whole name and the whole hint even set hard against each other, the row
/// carries no part of the hint and draws its name as any name is drawn
/// (TUI-RENDER-014); and in every case the name comes first, whole or
/// shortened as any name is, followed by nothing but blanks and at most the
/// whole hint. The row of the kitten the core holds as the Elder carries no
/// part of the hint, in its name's column or anywhere else on the row, however
/// much room its name leaves; what stands there instead is TUI-NAV-104's. A
/// row whose kitten's gate the core reports shut carries none (no reachable
/// meadow lists one: the gate opens with the first kitten). Quantified over
/// every kitten row, with nobody the Elder and with each kitten the Elder in
/// turn, with the cursor on each kitten row in turn and on no kitten at all,
/// at the floor, narrow, at the pane breakpoint, three-pane, and wide — with
/// names built at each width to sit just inside, on, and just past where the
/// hint fits, and names longer than the column.
///
/// @spec TUI-NAV-095
#[test]
fn every_kitten_row_but_the_elders_carries_the_whole_elder_hint_where_it_fits() {
    let dim = clowder_tui::theme::Role::Dim
        .color()
        .expect("the dim role is a colour");
    let store = store_names();
    let c = cost_field_width(&store);
    let hint_w = ELDER_HINT.chars().count();
    assert!(
        gives_count_field(Section::Clowder) && gives_cost_field(Section::Clowder),
        "fixture: the Clowder list gives both fields, so the name column is bounded by both"
    );
    assert!(
        ELDER_HINT.is_ascii(),
        "fixture: the hint as the spec words it is single-width"
    );

    let (mut drawn_with, mut drawn_without, mut focused_with) = (0u32, 0u32, 0u32);
    let (mut elder_rows, mut elder_rows_with_room, mut beside_an_elder) = (0u32, 0u32, 0u32);
    for (w, h) in [
        (FLOOR_W, 60u16),
        (90, 60),
        (99, 60),
        (100, 60),
        (130, 60),
        (200, 60),
    ] {
        // The name column's width, taken from the frame's own list space.
        let probe = clowder_named(&["Biscuit".to_string()]);
        let space = list_space(
            &cold(
                &probe,
                &ViewState {
                    section: Some(Section::Clowder),
                    ..Default::default()
                },
                w,
                h,
            )
            .layout,
        );
        let width = space.width as usize;
        let name_w = width - 4 - 8 - (c + 1);

        let mut names: Vec<String> = vec!["Biscuit".to_string(), name_of_len('W', name_w + 7)];
        for (head, len) in [
            ('C', name_w.checked_sub(hint_w + 1)),
            ('D', name_w.checked_sub(hint_w).map(|n| n + 1)),
            ('J', Some(name_w)),
            ('Q', name_w.checked_sub(hint_w)),
        ] {
            if let Some(len) = len.filter(|l| *l >= 2) {
                names.push(name_of_len(head, len));
            }
        }
        // Nobody the Elder, then each kitten in turn.
        let elders: Vec<Option<usize>> = std::iter::once(None)
            .chain((0..names.len()).map(Some))
            .collect();
        for elder in elders {
            let mut state = clowder_named(&names);
            state.elder = elder;
            let all = rows(&state, Section::Clowder);
            let kitten_ix: Vec<usize> = (0..all.len())
                .filter(|i| matches!(all[*i].key, RowKey::Kitten(..)))
                .collect();
            assert!(
                !kitten_ix.is_empty(),
                "fixture: kitten rows are listed at {w}x{h}"
            );

            let mut cursors: Vec<Option<RowKey>> = kitten_ix
                .iter()
                .map(|i| Some(all[*i].key.clone()))
                .collect();
            cursors.push(
                all.iter()
                    .find(|r| r.selectable() && !matches!(r.key, RowKey::Kitten(..)))
                    .map(|r| r.key.clone()),
            );

            for cursor_key in cursors {
                let mut view = ViewState {
                    section: Some(Section::Clowder),
                    ..Default::default()
                };
                view.focus[Section::Clowder.index()] = cursor_key.clone();
                let d = cold(&state, &view, w, h);
                let space = list_space(&d.layout);
                let lines = cell_lines(&d.buffer, space);
                let cursor = cursor_key
                    .as_ref()
                    .and_then(|k| all.iter().position(|r| &r.key == k))
                    .unwrap_or(0);
                let cy = lines.iter().position(|l| l[0] == "▸").unwrap_or_else(|| {
                    panic!("fixture: the cursor is drawn in the Clowder list at {w}x{h}")
                });

                for &ri in &kitten_ix {
                    let row = &all[ri];
                    let RowKey::Kitten(k, _) = row.key else {
                        unreachable!()
                    };
                    let y = (cy + ri)
                        .checked_sub(cursor)
                        .expect("fixture: the list is drawn unscrolled");
                    let line = &lines[y];
                    let name = &state.kittens[k].name;
                    let name_len = name.chars().count();
                    let focused = cursor_key.as_ref() == Some(&row.key);
                    let at = format!(
                        "at {w}x{h} with {}, kitten {k}'s row (a {name_len}-letter name in a \
                         {name_w}-column name column, {}) reads {:?}",
                        match elder {
                            Some(e) => format!("kitten {e} the Elder"),
                            None => "nobody the Elder".to_string(),
                        },
                        if focused {
                            "the cursor's"
                        } else {
                            "not the cursor's"
                        },
                        line.concat()
                    );
                    let column = cells_text(line, 4, 4 + name_w);
                    let fields = cells_text(line, 4 + name_w, width);

                    // Never in the count or cost field's place: that place carries
                    // the kitten's state, whose exact drawing is swept by
                    // `every_kitten_row_names_its_one_state_whole_in_its_fields_place`
                    // (TUI-RENDER-024); here only that no part of the hint is there.
                    assert!(
                        fields.starts_with(' ')
                            && !fields.contains(ELDER_HINT)
                            && !fields.contains("(n:"),
                        "{at}: the hint never stands in the count or cost field's place"
                    );

                    // The Elder's own row carries no hint, whole or in part,
                    // whatever room its name leaves. What it carries in the
                    // hint's place is swept by the TUI-NAV-104–106 tests.
                    if elder == Some(k) {
                        assert!(
                            !column.contains(ELDER_HINT)
                                && !column.contains("(n")
                                && !column.contains("n:"),
                            "{at}: the row of the kitten the core holds as the Elder carries no \
                             hint — the hint would offer what is already true"
                        );
                        elder_rows += 1;
                        if name_len + 1 + hint_w <= name_w {
                            elder_rows_with_room += 1;
                        }
                        continue;
                    }
                    if elder.is_some() {
                        beside_an_elder += 1;
                    }

                    // The name first, never cost a character; after it, blanks and
                    // at most the whole hint.
                    if name_len > name_w || !column.contains(ELDER_HINT) {
                        assert_eq!(
                            column,
                            format!("{:<name_w$}", fmt::fit(name, name_w)),
                            "{at}: without the whole hint, the name's column carries the name as any \
                             name is drawn and no part of the hint"
                        );
                    }
                    let hint_at = column.find(ELDER_HINT);
                    if let Some(off) = hint_at {
                        let head: String = column.chars().take(name_len).collect();
                        assert_eq!(&head, name, "{at}: the hint cost the name a character");
                        let between = &column[name.len()..off];
                        assert!(
                            between.chars().all(|ch| ch == ' '),
                            "{at}: something other than blanks between the name and the hint"
                        );
                        let after = &column[off + ELDER_HINT.len()..];
                        assert!(
                            after.chars().all(|ch| ch == ' '),
                            "{at}: something other than blanks after the hint"
                        );
                        let cell0 = 4 + column[..off].chars().count();
                        for (i, sym) in line[cell0..cell0 + hint_w].iter().enumerate() {
                            assert_eq!(
                                sym.chars().count(),
                                1,
                                "{at}: the hint's cell {i} is not one single-width character"
                            );
                            let fg = d
                                .buffer
                                .cell((space.x + (cell0 + i) as u16, space.y + y as u16))
                                .expect("in the buffer")
                                .fg;
                            assert_eq!(
                                fg, dim,
                                "{at}: the hint's cell {i} ({sym:?}) is not drawn in the dim role"
                            );
                        }
                    }

                    let gate = state.command_gate_open(&clowder_core::Command::AppointElder(k));
                    if !gate {
                        assert!(hint_at.is_none(), "{at}: the core reports this kitten's gate shut, so the row carries no hint");
                    } else if name_len + 1 + hint_w <= name_w {
                        assert!(
                            hint_at.is_some(),
                            "{at}: the gate is open, the kitten is not the Elder, and the name's \
                             column holds the whole name, a blank, and the whole `{ELDER_HINT}`, so \
                             the row carries the hint"
                        );
                        drawn_with += 1;
                        if focused {
                            focused_with += 1;
                        }
                    } else if name_len + hint_w > name_w {
                        assert!(
                            hint_at.is_none(),
                            "{at}: the name's column cannot hold the whole name and the whole hint, \
                             so the row carries no part of it"
                        );
                        drawn_without += 1;
                    }
                }
            }
        }
    }
    assert!(
        drawn_with > 0,
        "fixture: no row was wide enough to require the hint"
    );
    assert!(
        focused_with > 0,
        "fixture: the cursor's own row never required the hint"
    );
    assert!(
        drawn_without > 0,
        "fixture: no row was too narrow to hold the hint"
    );
    assert!(
        beside_an_elder > 0,
        "fixture: no other kitten's row was read while an Elder stood"
    );
    assert!(
        elder_rows > 0,
        "fixture: the Elder's own row was never read"
    );
    assert!(
        elder_rows_with_room > 0,
        "fixture: the Elder's own row never had room for the hint it must not carry"
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-104 / TUI-NAV-105 / TUI-NAV-106 — who the Elder is, on the roster
// ---------------------------------------------------------------------------

/// The Elder's mark, whole: the glyph, one blank column, and the word.
const ELDER_MARK: &str = "◆ Elder";
/// The mark's glyph alone — what stands where the word has no room.
const ELDER_GLYPH: &str = "◆";

/// The Clowder list as one frame drew it, unscrolled: the list's space, its
/// cells line by line, the width of the name's column the fields leave
/// (TUI-RENDER-020), and for each kitten's row its roster position and the
/// line of the list's space it is drawn on.
struct ClowderList {
    d: Drawn,
    space: Rect,
    lines: Vec<Vec<String>>,
    name_w: usize,
    kitten_lines: Vec<(usize, usize)>,
}

impl ClowderList {
    /// The line a kitten's row is drawn on.
    fn line_of_kitten(&self, k: usize) -> usize {
        self.kitten_lines
            .iter()
            .find(|(ix, _)| *ix == k)
            .map(|(_, y)| *y)
            .unwrap_or_else(|| panic!("fixture: kitten {k}'s row is listed"))
    }

    /// Every line of the list's space that draws the mark's glyph anywhere.
    fn lines_with_the_glyph(&self) -> Vec<usize> {
        (0..self.lines.len())
            .filter(|y| self.lines[*y].iter().any(|sym| sym == ELDER_GLYPH))
            .collect()
    }
}

/// Read the Clowder list out of a drawn frame whose cursor stands on
/// `cursor_key`, or where the list puts it when none is remembered. `c` is the
/// cost field's width.
fn clowder_list_in(
    d: Drawn,
    state: &GameState,
    cursor_key: Option<&RowKey>,
    c: usize,
) -> ClowderList {
    let space = list_space(&d.layout);
    let lines = cell_lines(&d.buffer, space);
    let all = rows(state, Section::Clowder);
    assert!(
        all.len() <= space.height as usize,
        "fixture: the Clowder list ({} rows) fits its {}-line pane unscrolled",
        all.len(),
        space.height
    );
    let cursor = cursor_key
        .and_then(|k| all.iter().position(|r| &r.key == k))
        .or_else(|| all.iter().position(|r| r.selectable()))
        .expect("fixture: the Clowder list has a row the cursor can stand on");
    let cy = lines
        .iter()
        .position(|l| l[0] == "▸")
        .expect("fixture: the cursor is drawn in the Clowder list");
    let kitten_lines = all
        .iter()
        .enumerate()
        .filter_map(|(ri, r)| match r.key {
            RowKey::Kitten(k, _) => Some((
                k,
                (cy + ri)
                    .checked_sub(cursor)
                    .expect("fixture: the list is drawn unscrolled"),
            )),
            _ => None,
        })
        .collect();
    let name_w = space.width as usize - 4 - 8 - (c + 1);
    ClowderList {
        d,
        space,
        lines,
        name_w,
        kitten_lines,
    }
}

/// The Clowder list drawn into a terminal that has never drawn anything.
fn clowder_list(
    state: &GameState,
    cursor_key: Option<&RowKey>,
    w: u16,
    h: u16,
    c: usize,
) -> ClowderList {
    let mut view = ViewState {
        section: Some(Section::Clowder),
        ..Default::default()
    };
    view.focus[Section::Clowder.index()] = cursor_key.cloned();
    clowder_list_in(cold(state, &view, w, h), state, cursor_key, c)
}

/// While the core holds a kitten as the Elder, the Elder's mark stands on that
/// kitten's row — the row for the roster position the core holds — and on no
/// other row of the roster, of the Clowder list, or of any other section's
/// list, whatever the kitten is doing: every job at home, every job away at
/// every away location, no job at home and away, a tinkerer bound and unbound.
/// Every other kitten's row is drawn as any kitten's row is. While the core
/// holds no Elder, no row carries the mark. The gate of AppointElder has no
/// say: no reachable meadow lists a kitten's row with that gate shut (the
/// gate opens with the first kitten), so the mark is read here under the only
/// answer the core gives a listed kitten. Quantified over nobody and every
/// kitten of a roster holding every state in turn as the Elder, at the floor,
/// narrow, at the pane breakpoint, three-pane, and wide.
///
/// @spec TUI-NAV-104
#[test]
fn the_elders_mark_stands_on_the_elders_row_and_on_no_other_whoever_the_elder_is() {
    let c = cost_field_width(&store_names());
    let base = every_state_roster(&[]);
    let everyone: BTreeSet<usize> = (0..base.kittens.len()).collect();
    let mut marked_whole: BTreeSet<usize> = BTreeSet::new();
    let (mut elder_rows, mut other_rows, mut elderless_frames) = (0u32, 0u32, 0u32);

    for (w, h) in [
        (FLOOR_W, 70u16),
        (90, 70),
        (99, 70),
        (100, 70),
        (130, 70),
        (200, 70),
    ] {
        let elders: Vec<Option<usize>> = std::iter::once(None)
            .chain(everyone.iter().copied().map(Some))
            .collect();
        for elder in elders {
            let mut state = base.clone();
            state.elder = elder;
            let list = clowder_list(&state, None, w, h, c);
            assert_eq!(
                list.kitten_lines.len(),
                state.kittens.len(),
                "fixture: every kitten is listed at {w}x{h}"
            );
            let whose = match elder {
                Some(e) => format!(
                    "kitten {e} ({:?} at {:?}) the Elder",
                    state.kittens[e].job, state.kittens[e].location
                ),
                None => "the core holding no Elder".to_string(),
            };

            for &(k, y) in &list.kitten_lines {
                let line = &list.lines[y];
                assert_eq!(
                    cells_text(line, 4, 4 + list.name_w),
                    expected_name_column(&state, k, list.name_w),
                    "at {w}x{h} with {whose}, kitten {k}'s row reads {:?}: the Elder's mark stands \
                     after the name on the Elder's own row, as far as its column holds it, and no \
                     other kitten's row carries it (TUI-NAV-104)",
                    line.concat()
                );
                if elder == Some(k) {
                    elder_rows += 1;
                    if kitten_aside(&state, k, list.name_w) == Some(ELDER_MARK) {
                        marked_whole.insert(k);
                    }
                } else {
                    other_rows += 1;
                }
            }

            // On no other row: nowhere in the list's space but the Elder's own line.
            let allowed: Vec<usize> = elder
                .filter(|e| kitten_aside(&state, *e, list.name_w).is_some())
                .map(|e| vec![list.line_of_kitten(e)])
                .unwrap_or_default();
            assert_eq!(
                list.lines_with_the_glyph(),
                allowed,
                "at {w}x{h} with {whose}, the lines of the Clowder list drawing `{ELDER_GLYPH}` are \
                 not exactly the Elder's own row (TUI-NAV-104)"
            );
            if elder.is_none() {
                elderless_frames += 1;
            }
        }
    }
    assert!(
        elder_rows > 0 && other_rows > 0 && elderless_frames > 0,
        "fixture: rows were read with and without an Elder"
    );
    assert_eq!(
        marked_whole, everyone,
        "fixture: every kitten of the roster, whatever it is doing, wore the whole mark as the \
         Elder at some width"
    );

    // No other section's list carries the mark on any row.
    let mut state = base.clone();
    state.elder = Some(0);
    for section in Section::ALL.into_iter().filter(|s| *s != Section::Clowder) {
        for (w, h) in [(FLOOR_W, 70u16), (130, 70)] {
            let d = cold(
                &state,
                &ViewState {
                    section: Some(section),
                    ..Default::default()
                },
                w,
                h,
            );
            let lines = cell_lines(&d.buffer, list_space(&d.layout));
            assert!(
                !lines.iter().any(|l| l.iter().any(|sym| sym == ELDER_GLYPH)),
                "at {w}x{h}, a row of the {section:?} list draws `{ELDER_GLYPH}`; the mark stands \
                 on the Elder's own row and on no other (TUI-NAV-104)"
            );
        }
    }
}

/// The mark is re-derived on every frame from the roster position the core
/// holds: on the very next frame drawn into the same terminal after an
/// appointment lands, the new Elder's row carries the mark and no other row
/// does — the row it left is drawn as any other kitten's row is, its hint
/// back — and it is the roster position that is marked, never a name: a later
/// namesake's appointment marks the namesake's row alone. The mark stays on
/// the Elder's row when the Elder wanders away or takes a job, and no row
/// carries it once the core holds no Elder.
///
/// @spec TUI-NAV-104
#[test]
fn the_elders_mark_follows_the_cores_elder_on_the_frame_an_appointment_lands() {
    use clowder_core::Command;

    let c = cost_field_width(&store_names());
    let names: Vec<String> = ["Biscuit", "Clover", "Biscuit", "Dumpling"]
        .iter()
        .map(|n| n.to_string())
        .collect();
    let mut state = clowder_named(&names);
    let view = ViewState {
        section: Some(Section::Clowder),
        ..Default::default()
    };
    let (w, h) = (130u16, 60u16);
    let mut term = terminal(w, h);
    let mut prev: Option<Layout> = None;

    let away = away_locations()[0];
    type MeadowStep<'a> = (&'a str, Box<dyn Fn(&mut GameState)>);
    let appoint = |k: usize| -> Box<dyn Fn(&mut GameState)> {
        Box::new(move |s: &mut GameState| {
            s.apply(Command::AppointElder(k))
                .expect("fixture: the core accepts the appointment");
        })
    };
    let steps: Vec<MeadowStep> = vec![
        (
            "nobody was yet the Elder",
            Box::new(|_s: &mut GameState| {}),
        ),
        ("kitten 0 was appointed", appoint(0)),
        (
            "kitten 0's later namesake, kitten 2, was appointed",
            appoint(2),
        ),
        ("kitten 2 was appointed again", appoint(2)),
        (
            "the Elder wandered away",
            Box::new(move |s: &mut GameState| {
                s.kittens[2].location = away;
                s.kittens[2].away_seq = Some(1);
            }),
        ),
        (
            "the Elder took a job",
            Box::new(|s: &mut GameState| s.kittens[2].job = Some(JobId::ALL[0])),
        ),
        ("kitten 1 was appointed", appoint(1)),
        ("kitten 3 was appointed", appoint(3)),
        (
            "the core held no Elder again",
            Box::new(|s: &mut GameState| s.elder = None),
        ),
    ];

    let (mut marked, mut left) = (0u32, 0u32);
    let mut before: Option<usize> = None;
    for (what, step) in steps {
        step(&mut state);
        let d = draw(&mut term, &state, &view, prev.as_ref());
        prev = Some(d.layout);
        let list = clowder_list_in(d, &state, None, c);
        for &(k, y) in &list.kitten_lines {
            let line = &list.lines[y];
            let column = cells_text(line, 4, 4 + list.name_w);
            let name = &state.kittens[k].name;
            let name_w = list.name_w;
            let (aside, why) = if state.elder == Some(k) {
                marked += 1;
                (
                    ELDER_MARK,
                    "is the kitten the core now holds as the Elder, so its row carries the mark",
                )
            } else {
                if before == Some(k) {
                    left += 1;
                }
                (ELDER_HINT, "is not the Elder, so its row is drawn as any other kitten's, its hint in place")
            };
            assert_eq!(
                column,
                format!("{:<name_w$}", format!("{name} {aside}")),
                "on the frame after {what}, kitten {k} {why} (TUI-NAV-104): {:?}",
                line.concat()
            );
        }
        let allowed: Vec<usize> = state
            .elder
            .map(|e| vec![list.line_of_kitten(e)])
            .unwrap_or_default();
        assert_eq!(
            list.lines_with_the_glyph(),
            allowed,
            "on the frame after {what}, the lines drawing `{ELDER_GLYPH}` are not exactly the row \
             of the Elder the core holds ({:?}) (TUI-NAV-104)",
            state.elder
        );
        before = state.elder;
    }
    assert!(marked > 0, "fixture: some frame was drawn with an Elder");
    assert!(
        left > 0,
        "fixture: some frame was drawn just after the mark left a row"
    );
}

/// The mark is `◆ Elder` — the glyph, one blank column, and the word, each
/// cell one single-width character — drawn after the kitten's name within the
/// name's column, one blank column on, exactly where the hint begins on every
/// other kitten's row. It wears the row's own style, cell for cell what the
/// name wears — the cursor's reverse with it under the cursor — and no colour
/// of its own. No part of it stands in the count and cost fields' place, where
/// the kitten's state is drawn as on every kitten's row. Quantified over an
/// Elder in each state a row names — idle, at work, away, a tinkerer — with
/// the cursor on each kitten's row in turn and on no kitten, at the floor,
/// narrow, at the pane breakpoint, three-pane, and wide.
///
/// @spec TUI-NAV-105
#[test]
fn the_elders_mark_is_its_glyph_a_blank_and_its_word_after_the_name_in_the_rows_own_style() {
    use ratatui::style::Modifier;

    let c = cost_field_width(&store_names());
    let place = 7 + 1 + c;
    let mark: Vec<String> = ELDER_MARK.chars().map(|ch| ch.to_string()).collect();
    assert_eq!(
        mark.len(),
        7,
        "fixture: the mark as the spec words it is the glyph, a blank, and a five-letter word"
    );
    assert_eq!(
        (mark[0].as_str(), mark[1].as_str()),
        (ELDER_GLYPH, " "),
        "fixture: the glyph, then one blank column"
    );

    let names: Vec<String> = ["Bo", "Fig", "Mo", "Zu"]
        .iter()
        .map(|n| n.to_string())
        .collect();
    let mut base = clowder_named(&names);
    base.kittens[1].job = Some(JobId::ALL[0]);
    base.kittens[1].job_seq = Some(1);
    base.kittens[2].location = away_locations()[0];
    base.kittens[2].away_seq = Some(1);
    base.kittens[3].job = Some(JobId::Tinkerer);
    base.kittens[3].job_seq = Some(2);
    base.kittens[3].tinkering = Some(CraftId::ALL[0]);

    let (mut whole, mut under_the_cursor, mut beside_a_hint) = (0u32, 0u32, 0u32);
    for (w, h) in [
        (FLOOR_W, 60u16),
        (90, 60),
        (99, 60),
        (100, 60),
        (130, 60),
        (200, 60),
    ] {
        let mut whole_here = 0u32;
        for elder in 0..base.kittens.len() {
            let mut state = base.clone();
            state.elder = Some(elder);
            let all = rows(&state, Section::Clowder);
            let mut cursors: Vec<Option<RowKey>> = all
                .iter()
                .filter(|r| matches!(r.key, RowKey::Kitten(..)))
                .map(|r| Some(r.key.clone()))
                .collect();
            cursors.push(
                all.iter()
                    .find(|r| r.selectable() && !matches!(r.key, RowKey::Kitten(..)))
                    .map(|r| r.key.clone()),
            );

            for cursor_key in cursors {
                let list = clowder_list(&state, cursor_key.as_ref(), w, h, c);
                let (name_w, width) = (list.name_w, list.space.width as usize);
                let y = list.line_of_kitten(elder);
                let line = &list.lines[y];
                let kitten = &state.kittens[elder];
                let name_len = kitten.name.chars().count();
                let focused = matches!(cursor_key, Some(RowKey::Kitten(k, _)) if k == elder);
                let at = format!(
                    "at {w}x{h}, the row of the Elder, kitten {elder} ({:?} at {:?}, {}), reads {:?}",
                    kitten.job,
                    kitten.location,
                    if focused { "the cursor's row" } else { "not the cursor's row" },
                    line.concat()
                );
                if name_len + 1 + mark.len() > name_w {
                    continue; // no room for the whole mark: TUI-NAV-106's steps
                }
                whole += 1;
                whole_here += 1;

                // The glyph, one blank column, and the word, after the name
                // and one blank column, and nothing else in the name's column.
                assert_eq!(
                    cells_text(line, 4, 4 + name_w),
                    format!("{:<name_w$}", format!("{} {ELDER_MARK}", kitten.name)),
                    "{at}: the name's column carries the name, one blank column, and the whole \
                     mark `{ELDER_MARK}` (TUI-NAV-105)"
                );
                let cell0 = 4 + name_len + 1;
                let cell_at = |x: usize| {
                    list.d
                        .buffer
                        .cell((list.space.x + x as u16, list.space.y + y as u16))
                        .expect("in the buffer")
                };
                let name_cell = cell_at(4);
                assert_eq!(
                    name_cell.modifier.contains(Modifier::REVERSED),
                    focused,
                    "fixture: {at}: the name is reversed exactly under the cursor"
                );
                for (i, expected) in mark.iter().enumerate() {
                    let sym = &line[cell0 + i];
                    assert_eq!(sym, expected, "{at}: the mark's cell {i} (TUI-NAV-105)");
                    assert_eq!(
                        sym.chars().count(),
                        1,
                        "{at}: the mark's cell {i} is not one single-width character"
                    );
                    let cell = cell_at(cell0 + i);
                    assert_eq!(
                        (cell.fg, cell.bg, cell.modifier),
                        (name_cell.fg, name_cell.bg, name_cell.modifier),
                        "{at}: the mark's cell {i} ({sym:?}) is not drawn in the row's own style, as \
                         the name is — it takes no colour of its own, and the cursor's reverse with \
                         the row (TUI-NAV-105)"
                    );
                }
                if focused {
                    under_the_cursor += 1;
                }

                // Placed as the hint is placed on every other kitten's row.
                for &(k, ky) in &list.kitten_lines {
                    if k == elder {
                        continue;
                    }
                    let other = cells_text(&list.lines[ky], 4, 4 + name_w);
                    if let Some(off) = other.find(ELDER_HINT) {
                        assert_eq!(
                            off - state.kittens[k].name.len(),
                            cell0 - 4 - name_len,
                            "{at}: the mark begins as far after its name as the hint does after \
                             kitten {k}'s — one blank column (TUI-NAV-105)"
                        );
                        beside_a_hint += 1;
                    }
                }

                // No part of it in the fields' place: the kitten's state
                // stands there as on every kitten's row (TUI-RENDER-024).
                assert_eq!(
                    cells_text(line, 4 + name_w, width),
                    format!(" {:<place$}", expected_state_word(kitten)),
                    "{at}: the count and cost fields' place carries the kitten's one state and no \
                     part of the mark (TUI-NAV-105, TUI-NAV-101)"
                );
            }
        }
        assert!(
            whole_here > 0,
            "fixture: at {w}x{h} no Elder's row had room for the whole mark"
        );
    }
    assert!(
        whole > 0 && under_the_cursor > 0,
        "fixture: the whole mark was read, under the cursor too"
    );
    assert!(
        beside_a_hint > 0,
        "fixture: no frame drew the mark and another row's hint together"
    );
}

/// The mark is its row's own and no part of the kitten's name: while the
/// Elder's row carries it, the palette's result for the Elder — and for every
/// other kitten — shows the kitten's name alone, a query for the mark, its
/// glyph, or its word finds no kitten whose name does not hold it, and the
/// palette draws no mark beside the Elder's name. Quantified over each kitten
/// of a roster as the Elder.
///
/// @spec TUI-NAV-105
#[test]
fn the_elders_mark_is_its_rows_own_and_no_part_of_the_kittens_name() {
    use clowder_tui::nav::palette::results;
    use clowder_tui::nav::Overlay;

    let c = cost_field_width(&store_names());
    let names: Vec<String> = ["Biscuit", "Clover", "Dumpling"]
        .iter()
        .map(|n| n.to_string())
        .collect();
    let (w, h) = (130u16, 60u16);
    for elder in 0..names.len() {
        let mut state = clowder_named(&names);
        state.elder = Some(elder);
        let elder_name = state.kittens[elder].name.clone();

        // The row carries the mark...
        let list = clowder_list(&state, None, w, h, c);
        let name_w = list.name_w;
        let line = &list.lines[list.line_of_kitten(elder)];
        assert_eq!(
            cells_text(line, 4, 4 + name_w),
            format!("{:<name_w$}", format!("{elder_name} {ELDER_MARK}")),
            "the row of the Elder, kitten {elder}, carries the mark after its name (TUI-NAV-105): {:?}",
            line.concat()
        );

        // ...and the palette's result for the kitten shows the name alone.
        for (k, kitten) in state.kittens.iter().enumerate() {
            let named: Vec<_> = results(&state, "")
                .into_iter()
                .filter(|r| matches!(r.key, RowKey::Kitten(ix, _) if ix == k))
                .collect();
            assert_eq!(named.len(), 1, "fixture: kitten {k} is one palette result");
            assert_eq!(
                named[0].name, kitten.name,
                "with kitten {elder} the Elder, kitten {k}'s palette result carries its name alone (TUI-NAV-105)"
            );
        }
        for query in [ELDER_MARK, ELDER_GLYPH, "Elder", "elder"] {
            for r in results(&state, query) {
                if let RowKey::Kitten(ix, _) = r.key {
                    assert!(
                        state.kittens[ix]
                            .name
                            .to_lowercase()
                            .contains(&query.to_lowercase()),
                        "a palette query for {query:?} found kitten {ix} by a mark its name does \
                         not hold (TUI-NAV-105)"
                    );
                }
            }
        }
        let view = ViewState {
            section: Some(Section::Clowder),
            overlay: Some(Overlay::Palette {
                query: elder_name.clone(),
                selected: 0,
            }),
            ..Default::default()
        };
        let d = cold(&state, &view, w, h);
        let drawn = region_lines(&d.buffer, d.buffer.area);
        assert!(
            drawn.iter().any(|l| l.trim_end().ends_with(&format!(" {elder_name}"))),
            "the palette shows the Elder, kitten {elder}, by its name alone, with nothing after it \
             (TUI-NAV-105):\n{}",
            drawn.join("\n")
        );
        assert!(
            !drawn.iter().any(|l| l.contains(ELDER_GLYPH)),
            "the palette draws the Elder's mark; the mark is its row's own and no part of the \
             kitten's name (TUI-NAV-105):\n{}",
            drawn.join("\n")
        );
    }
}

/// On the Elder's row the name is drawn exactly as on any kitten's row — whole
/// where its column holds it, shortened as any name is where it does not — and
/// after it stands the first of these that the column holds together with the
/// whole name and one blank column: the whole mark `◆ Elder`; the glyph `◆`
/// alone; no mark. The mark never costs the name a character, the word is
/// never drawn in part, and the row carries no mark only where its column
/// cannot hold the whole name, the blank, and the glyph. The steps are the
/// same at every width: what the row carries follows only from how long the
/// name is and how wide its column. Quantified over every width the list's
/// space takes from the floor up, narrow and three-pane alike, with names
/// built at each to sit just inside, on, and just past where the whole mark
/// fits and where the glyph fits, to fill the column, and to overrun it — with
/// the cursor on the Elder's row and off it.
///
/// @spec TUI-NAV-106
#[test]
fn the_elders_row_keeps_its_name_and_draws_the_first_step_of_the_mark_its_column_holds() {
    let c = cost_field_width(&store_names());
    let place = 7 + 1 + c;
    let mark_w = ELDER_MARK.chars().count();
    let glyph_w = ELDER_GLYPH.chars().count();
    let h = 60u16;

    // One terminal width for each distinct space the list takes.
    let mut sizes: Vec<u16> = Vec::new();
    let mut spaces: BTreeSet<(u16, bool)> = BTreeSet::new();
    for w in FLOOR_W..=240 {
        let layout = compute(area(w, h), None, false);
        let wide = matches!(layout.body, Body::Wide { .. });
        if spaces.insert((list_space(&layout).width, wide)) {
            sizes.push(w);
        }
    }
    assert!(
        sizes.len() > 6,
        "fixture: the list takes many widths between the floor and its cap"
    );

    let (mut step_whole, mut step_glyph, mut step_none, mut name_cut) = (0u32, 0u32, 0u32, 0u32);
    for w in sizes {
        let width = list_space(&compute(area(w, h), None, false)).width as usize;
        let name_w = width - 4 - 8 - (c + 1);

        // Name lengths either side of each step's edge.
        let lens: BTreeSet<usize> = [
            Some(1),
            Some(2),
            name_w.checked_sub(mark_w + 2),
            name_w.checked_sub(mark_w + 1), // the whole mark fits exactly
            name_w.checked_sub(mark_w),     // the whole mark is one column short
            name_w.checked_sub(glyph_w + 2),
            name_w.checked_sub(glyph_w + 1), // the glyph fits exactly
            name_w.checked_sub(glyph_w),     // the glyph is one column short
            Some(name_w),                    // the name fills its column
            Some(name_w + 1),                // the name is one column too long
            Some(name_w + 7),
        ]
        .into_iter()
        .flatten()
        .filter(|l| *l >= 1)
        .collect();
        let heads = ['B', 'C', 'D', 'F', 'G', 'H', 'J', 'K', 'L', 'M', 'N'];
        let names: Vec<String> = lens
            .iter()
            .zip(heads)
            .map(|(len, head)| name_of_len(head, *len))
            .collect();
        assert_eq!(names.len(), lens.len(), "fixture: a head for every name");
        let (mut whole_here, mut glyph_here, mut none_here) = (0u32, 0u32, 0u32);

        for elder in 0..names.len() {
            let mut state = clowder_named(&names);
            state.elder = Some(elder);
            let all = rows(&state, Section::Clowder);
            let on_the_elder = all
                .iter()
                .find(|r| matches!(r.key, RowKey::Kitten(k, _) if k == elder))
                .map(|r| r.key.clone())
                .expect("fixture: the Elder's row is listed");
            let off_the_roster = all
                .iter()
                .find(|r| r.selectable() && !matches!(r.key, RowKey::Kitten(..)))
                .map(|r| r.key.clone())
                .expect("fixture: a row that is no kitten's is listed");

            for (cursor_key, whose) in [
                (on_the_elder, "the cursor's row"),
                (off_the_roster, "not the cursor's row"),
            ] {
                let list = clowder_list(&state, Some(&cursor_key), w, h, c);
                assert_eq!(
                    list.name_w, name_w,
                    "fixture: the list's space at {w}x{h} is the layout's"
                );
                let line = &list.lines[list.line_of_kitten(elder)];
                let name = &state.kittens[elder].name;
                let len = name.chars().count();
                let at = format!(
                    "at {w}x{h}, the Elder's row ({whose}; a {len}-letter name in a {name_w}-column \
                     name column) reads {:?}",
                    line.concat()
                );

                let (expected, step) = if len + 1 + mark_w <= name_w {
                    whole_here += 1;
                    (format!("{name} {ELDER_MARK}"), "the whole name, a blank column, and the whole mark all fit, so the row carries the whole mark")
                } else if len + 1 + glyph_w <= name_w {
                    glyph_here += 1;
                    (format!("{name} {ELDER_GLYPH}"), "the whole mark does not fit beside the whole name but its glyph does, so the row carries the glyph alone and no part of the word")
                } else {
                    none_here += 1;
                    if len > name_w {
                        name_cut += 1;
                    }
                    (fmt::fit(name, name_w), "the column cannot hold the whole name, a blank column, and the glyph, so the row carries no mark and draws its name as any name is drawn")
                };
                assert_eq!(
                    cells_text(line, 4, 4 + name_w),
                    format!("{expected:<name_w$}"),
                    "{at}: {step} (TUI-NAV-106)"
                );
                // The name's column is all the room the mark has.
                assert_eq!(
                    cells_text(line, 4 + name_w, width),
                    format!(" {:<place$}", IDLE_WORD),
                    "{at}: nothing of the mark stands past the name's column (TUI-NAV-106)"
                );
            }
        }
        assert!(
            glyph_here > 0 && none_here > 0,
            "fixture: at {w}x{h} the glyph-alone and no-mark steps were both drawn"
        );
        step_whole += whole_here;
        step_glyph += glyph_here;
        step_none += none_here;
    }
    assert!(
        step_whole > 0 && step_glyph > 0 && step_none > 0,
        "fixture: every step was drawn"
    );
    assert!(
        name_cut > 0,
        "fixture: some Elder's name was too long for its column"
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-101 / TUI-RENDER-024 — what a kitten's row says it is doing
// ---------------------------------------------------------------------------

use clowder_core::content::job_def;

/// The word a kitten away from home reads, whatever job it holds.
const AWAY_WORD: &str = "visiting";
/// The word a kitten at home with no job reads — the idle chip's own word.
const IDLE_WORD: &str = "idle";

/// Every location that is not home. The match is exhaustive, so a location
/// added to the core lands here or fails to compile, and the sweeps below
/// stay quantified over every place a kitten can be away at.
fn away_locations() -> Vec<Location> {
    [Location::Home, Location::NeighborsYard]
        .into_iter()
        .filter(|l| match l {
            Location::Home => false,
            Location::NeighborsYard => true,
        })
        .collect()
}

/// The one state TUI-NAV-101 names for a kitten, read straight from its
/// location and job: away reads the away word whatever the job; at home with
/// a job, the job's name as its definition spells it; at home with none, the
/// idle word.
fn expected_state_word(k: &Kitten) -> String {
    if k.location != Location::Home {
        AWAY_WORD.to_string()
    } else if let Some(j) = k.job {
        job_def(j).name.to_string()
    } else {
        IDLE_WORD.to_string()
    }
}

/// A roster that holds every state a kitten's row can name, each reached
/// every way the rule distinguishes: every job at home and every job away at
/// every away location, no job at home and away, a tinkerer bound to a recipe
/// and one bound to none, at home and away, and the Elder among them — so the
/// word is checked for every job in the vocabulary, for both words, and on
/// the Elder's own row. `long` names are added at the front to push the name
/// column against the `(n: Elder)` hint's fit.
fn every_state_roster(long: &[String]) -> GameState {
    let mut s = rich_meadow();
    let mut roster: Vec<Kitten> = long.iter().map(|n| kitten_named(n, None)).collect();
    for j in JobId::ALL {
        roster.push(kitten_named(&format!("Home{j:?}"), Some(j)));
        for (li, loc) in away_locations().into_iter().enumerate() {
            let mut k = kitten_named(&format!("Away{li}{j:?}"), Some(j));
            k.location = loc;
            k.away_seq = Some(1);
            roster.push(k);
        }
    }
    roster.push(kitten_named("Lazybones", None));
    for (li, loc) in away_locations().into_iter().enumerate() {
        let mut k = kitten_named(&format!("Wanderer{li}"), None);
        k.location = loc;
        k.away_seq = Some(1);
        roster.push(k);
    }
    let mut bound = kitten_named("Boundtink", Some(JobId::Tinkerer));
    bound.tinkering = Some(CraftId::ALL[0]);
    roster.push(bound.clone());
    for (li, loc) in away_locations().into_iter().enumerate() {
        let mut k = bound.clone();
        k.name = format!("Boundaway{li}");
        k.location = loc;
        k.away_seq = Some(1);
        roster.push(k);
    }
    s.roster_high_water = s.roster_high_water.max(roster.len());
    s.kittens = roster;
    s
}

/// What a kitten's row carries after its name, one blank column on, in a
/// name's column `name_w` wide. On the row of the kitten the core holds as
/// the Elder: the first step of the Elder's mark the column holds together
/// with the whole name and that blank — the whole mark, else its glyph alone,
/// else nothing (TUI-NAV-104, TUI-NAV-106) — and never the hint. On any other
/// kitten's row: the whole hint, while the core reports the gate open and the
/// column holds it (TUI-NAV-095). Otherwise nothing.
fn kitten_aside(state: &GameState, k: usize, name_w: usize) -> Option<&'static str> {
    let name_len = state.kittens[k].name.chars().count();
    let fits = |aside: &str| name_len + 1 + aside.chars().count() <= name_w;
    if state.elder == Some(k) {
        [ELDER_MARK, ELDER_GLYPH]
            .into_iter()
            .find(|step| fits(step))
    } else if state.command_gate_open(&clowder_core::Command::AppointElder(k)) {
        Some(ELDER_HINT).filter(|hint| fits(hint))
    } else {
        None
    }
}

/// The name's column a kitten's row draws, exactly as TUI-RENDER-020,
/// TUI-NAV-095, and TUI-NAV-104–106 draw it with no state word in the
/// picture: the whole name, a blank, and what the row carries after its name
/// (`kitten_aside`) where it carries anything; otherwise the name as any name
/// is drawn.
fn expected_name_column(state: &GameState, k: usize, name_w: usize) -> String {
    let name = &state.kittens[k].name;
    match kitten_aside(state, k, name_w) {
        Some(aside) => format!("{:<name_w$}", format!("{name} {aside}")),
        None => format!("{:<name_w$}", fmt::fit(name, name_w)),
    }
}

/// Every listed kitten row names exactly one state in its count and cost
/// fields' place, read from the kitten's location and job as the core holds
/// them: the away word wherever it is anywhere but home, job or none; at home
/// with a job, that job's name as its definition spells it — the spelling its
/// count row in the jobs group draws — a tinkerer's included, bound to a
/// recipe or not; at home with no job, the idle word; and nothing of the
/// Elder. The word begins at the count field's left edge, one blank column
/// after the name's column, left-aligned so every kitten's word starts at one
/// column; it is drawn whole, never cut, in the row's own style (the cursor's
/// reverse included), with blanks after it to the cost field's end; and the
/// name's column keeps the width the fields leave, drawing the name and the
/// `(n: Elder)` hint exactly as it would with no word there. The place is
/// fixed by the resource vocabulary and is wider than every job name and both
/// words. Quantified over every job, every away location, both words, the
/// Elder's row, with the cursor on each kitten row in turn and on no kitten,
/// at the floor, narrow, at the pane breakpoint, three-pane, and wide — with
/// names built at each width to sit just inside, on, and just past where the
/// hint fits.
///
/// @spec TUI-NAV-101, TUI-RENDER-024
#[test]
fn every_kitten_row_names_its_one_state_whole_in_its_fields_place() {
    let store = store_names();
    let c = cost_field_width(&store);
    let place = 7 + 1 + c;
    let hint_w = ELDER_HINT.chars().count();
    assert!(
        gives_count_field(Section::Clowder) && gives_cost_field(Section::Clowder),
        "fixture: the Clowder list gives both fields, so a kitten's row has their place"
    );
    for word in JobId::ALL
        .iter()
        .map(|j| job_def(*j).name)
        .chain([AWAY_WORD, IDLE_WORD])
    {
        assert!(
            word.chars().count() < place,
            "the fields' place ({place} columns) is wider than every job name and both words; \
             {word:?} is not narrower (TUI-RENDER-024)"
        );
    }

    let (mut checked, mut focused_checked, mut elder_checked) = (0u32, 0u32, 0u32);
    let mut words_seen: BTreeSet<String> = BTreeSet::new();
    let mut job_rows_checked: BTreeSet<String> = BTreeSet::new();
    for (w, h) in [
        (FLOOR_W, 70u16),
        (90, 70),
        (99, 70),
        (100, 70),
        (130, 70),
        (200, 70),
    ] {
        let probe = every_state_roster(&[]);
        let space = list_space(
            &cold(
                &probe,
                &ViewState {
                    section: Some(Section::Clowder),
                    ..Default::default()
                },
                w,
                h,
            )
            .layout,
        );
        let width = space.width as usize;
        let name_w = width - 4 - 8 - (c + 1);
        let count_at = 4 + name_w + 1;
        assert_eq!(
            count_at + place,
            width,
            "fixture: the fields' place runs to the row's right edge"
        );

        let mut long: Vec<String> = vec![name_of_len('W', name_w + 7)];
        for (head, len) in [
            ('C', name_w.checked_sub(hint_w + 1)),
            ('D', name_w.checked_sub(hint_w).map(|n| n + 1)),
            ('X', Some(name_w)),
            ('Q', name_w.checked_sub(hint_w)),
        ] {
            if let Some(len) = len.filter(|l| *l >= 2) {
                long.push(name_of_len(head, len));
            }
        }
        let mut state = every_state_roster(&long);
        // The Elder is a kitten at home with a job, so its word is the job's
        // and says nothing of the appointment.
        let elder = state
            .kittens
            .iter()
            .position(|k| k.location == Location::Home && k.job.is_some())
            .expect("fixture");
        state.elder = Some(elder);

        let all = rows(&state, Section::Clowder);
        let kitten_ix: Vec<usize> = (0..all.len())
            .filter(|i| matches!(all[*i].key, RowKey::Kitten(..)))
            .collect();
        assert_eq!(
            kitten_ix.len(),
            state.kittens.len(),
            "fixture: every kitten is listed at {w}x{h}"
        );

        let mut cursors: Vec<Option<RowKey>> = kitten_ix
            .iter()
            .map(|i| Some(all[*i].key.clone()))
            .collect();
        cursors.push(
            all.iter()
                .find(|r| r.selectable() && !matches!(r.key, RowKey::Kitten(..)))
                .map(|r| r.key.clone()),
        );

        for cursor_key in cursors {
            let mut view = ViewState {
                section: Some(Section::Clowder),
                ..Default::default()
            };
            view.focus[Section::Clowder.index()] = cursor_key.clone();
            let d = cold(&state, &view, w, h);
            let space = list_space(&d.layout);
            assert_eq!(
                space.width as usize, width,
                "fixture: the list's width is the probe's at {w}x{h}"
            );
            let lines = cell_lines(&d.buffer, space);
            let cursor = cursor_key
                .as_ref()
                .and_then(|k| all.iter().position(|r| &r.key == k))
                .unwrap_or(0);
            let cy = lines.iter().position(|l| l[0] == "▸").unwrap_or_else(|| {
                panic!("fixture: the cursor is drawn in the Clowder list at {w}x{h}")
            });
            let line_of_row = |ri: usize| {
                (cy + ri)
                    .checked_sub(cursor)
                    .expect("fixture: the list is drawn unscrolled")
            };

            // The job's name as its count row in the jobs group draws it.
            for (ri, row) in all.iter().enumerate() {
                if let (RowKey::Content(ContentId::Job(j), _), RowKind::Count { .. }) =
                    (&row.key, &row.kind)
                {
                    let col = cells_text(&lines[line_of_row(ri)], 4, 4 + name_w);
                    let def = job_def(*j).name;
                    if def.chars().count() <= name_w {
                        assert_eq!(
                            col.trim_end(),
                            def,
                            "fixture: the job's count row draws its definition's name"
                        );
                        job_rows_checked.insert(def.to_string());
                    }
                }
            }

            for &ri in &kitten_ix {
                let row = &all[ri];
                let RowKey::Kitten(k, _) = row.key else {
                    unreachable!()
                };
                let y = line_of_row(ri);
                let line = &lines[y];
                let kitten = &state.kittens[k];
                let word = expected_state_word(kitten);
                let focused = cursor_key.as_ref() == Some(&row.key);
                let at = format!(
                    "at {w}x{h}, kitten {k} ({:?} at {:?}{}, {}) reads {:?}",
                    kitten.job,
                    kitten.location,
                    if Some(k) == state.elder {
                        ", the Elder"
                    } else {
                        ""
                    },
                    if focused {
                        "the cursor's row"
                    } else {
                        "not the cursor's row"
                    },
                    line.concat()
                );

                // The name's column keeps the width the fields leave and draws
                // exactly what it would with no word: the word shortens no
                // name and moves no hint.
                assert_eq!(
                    cells_text(line, 4, 4 + name_w),
                    expected_name_column(&state, k, name_w),
                    "{at}: the name's column keeps its {name_w} columns and draws the name and \
                     the hint — or, on the Elder's row, the Elder's mark — exactly as without \
                     the word (TUI-RENDER-024, TUI-NAV-095, TUI-NAV-104)"
                );
                assert_eq!(
                    line[4 + name_w],
                    " ",
                    "{at}: one blank column after the name's column"
                );

                // Exactly the one word, begun at the count field's left edge,
                // left-aligned, whole, blank after it to the cost field's end.
                assert_eq!(
                    cells_text(line, count_at, width),
                    format!("{word:<place$}"),
                    "{at}: the fields' place names exactly the one state {word:?}, begun at the \
                     count field's left edge, left-aligned and whole, blank after it to the cost \
                     field's end (TUI-NAV-101, TUI-RENDER-024)"
                );

                // In the row's own style: every cell of the place wears what
                // the row's marker wears, the cursor's reverse included.
                let marker_cell = d
                    .buffer
                    .cell((space.x + 2, space.y + y as u16))
                    .expect("in the buffer")
                    .clone();
                for x in count_at..count_at + word.chars().count() {
                    let cell = d
                        .buffer
                        .cell((space.x + x as u16, space.y + y as u16))
                        .expect("in the buffer");
                    assert_eq!(
                        (cell.fg, cell.bg, cell.modifier),
                        (marker_cell.fg, marker_cell.bg, marker_cell.modifier),
                        "{at}: the word's cell {} is not drawn in the row's own style (TUI-RENDER-024)",
                        x - count_at
                    );
                }

                checked += 1;
                words_seen.insert(word);
                if focused {
                    focused_checked += 1;
                }
                if Some(k) == state.elder {
                    elder_checked += 1;
                }
            }
        }
    }
    assert!(checked > 0, "fixture: kitten rows were checked");
    assert!(
        focused_checked > 0,
        "fixture: the cursor's own row was checked"
    );
    assert!(elder_checked > 0, "fixture: the Elder's row was checked");
    for word in JobId::ALL
        .iter()
        .map(|j| job_def(*j).name.to_string())
        .chain([AWAY_WORD.into(), IDLE_WORD.into()])
    {
        assert!(
            words_seen.contains(&word),
            "fixture: some kitten's row named {word:?}"
        );
    }
    assert!(
        !job_rows_checked.is_empty(),
        "fixture: some job's count row was read for its spelling"
    );
}

/// The word belongs to the row, not to the kitten's name: while every kitten's
/// row names its state, the palette's result for each kitten carries the
/// kitten's name alone — a query for any state word finds no kitten whose
/// name does not hold it — and the inspector names each kitten as before.
///
/// @spec TUI-NAV-101
#[test]
fn a_kittens_state_word_is_drawn_on_its_row_and_is_no_part_of_its_name() {
    use clowder_tui::nav::palette::results;

    let store = store_names();
    let c = cost_field_width(&store);
    let place = 7 + 1 + c;
    let state = every_state_roster(&[]);
    let (w, h) = (130u16, 70u16);
    let view = ViewState {
        section: Some(Section::Clowder),
        ..Default::default()
    };
    let d = cold(&state, &view, w, h);
    let space = list_space(&d.layout);
    let width = space.width as usize;
    let lines = cell_lines(&d.buffer, space);

    for (k, kitten) in state.kittens.iter().enumerate() {
        let word = expected_state_word(kitten);
        let line = lines
            .iter()
            .find(|l| cells_text(l, 4, width).starts_with(&format!("{} ", kitten.name)))
            .unwrap_or_else(|| panic!("fixture: kitten {k}'s row is drawn"));
        assert_eq!(
            cells_text(line, width - place, width),
            format!("{word:<place$}"),
            "kitten {k}'s row names its state {word:?} in its fields' place (TUI-NAV-101): {:?}",
            line.concat()
        );

        let named: Vec<_> = results(&state, "")
            .into_iter()
            .filter(|r| matches!(r.key, RowKey::Kitten(ix, _) if ix == k))
            .collect();
        assert_eq!(named.len(), 1, "fixture: kitten {k} is one palette result");
        assert_eq!(
            named[0].name, kitten.name,
            "kitten {k}'s palette result carries its name alone (TUI-NAV-101)"
        );

        assert_eq!(
            inspector_name(&state, Section::Clowder, &RowKey::Kitten(k, "")),
            kitten.name,
            "the inspector names kitten {k} by its name alone (TUI-NAV-101)"
        );
    }
    for word in JobId::ALL
        .iter()
        .map(|j| job_def(*j).name.to_string())
        .chain([AWAY_WORD.into(), IDLE_WORD.into()])
    {
        for r in results(&state, &word) {
            if let RowKey::Kitten(ix, _) = r.key {
                assert!(
                    state.kittens[ix]
                        .name
                        .to_lowercase()
                        .contains(&word.to_lowercase()),
                    "a palette query for {word:?} found kitten {ix} by a word its name does not \
                     hold (TUI-NAV-101)"
                );
            }
        }
    }
}

/// The word is re-derived on every frame: an assignment, a change of job, a
/// wandering away, a homecoming, and an unassignment each show on the very
/// next frame drawn into the same terminal, with the previous layout carried
/// as the loop carries it.
///
/// @spec TUI-NAV-101
#[test]
fn a_kittens_state_word_follows_the_core_on_the_next_frame() {
    let store = store_names();
    let c = cost_field_width(&store);
    let place = 7 + 1 + c;
    let mut state = clowder_named(&["Biscuit".to_string(), "Marmalade".to_string()]);
    let view = ViewState {
        section: Some(Section::Clowder),
        ..Default::default()
    };
    let (w, h) = (130u16, 60u16);
    let mut term = terminal(w, h);
    let mut prev: Option<Layout> = None;

    let away = away_locations();
    type KittenStep<'a> = (&'a str, Box<dyn Fn(&mut Kitten)>);
    let mut steps: Vec<KittenStep> = vec![
        ("idle at home", Box::new(|_k: &mut Kitten| {})),
        (
            "assigned",
            Box::new(|k: &mut Kitten| k.job = Some(JobId::ALL[0])),
        ),
        (
            "reassigned",
            Box::new(|k: &mut Kitten| k.job = Some(JobId::Tinkerer)),
        ),
    ];
    for loc in away {
        steps.push((
            "wandered away",
            Box::new(move |k: &mut Kitten| k.location = loc),
        ));
        steps.push((
            "came home",
            Box::new(|k: &mut Kitten| k.location = Location::Home),
        ));
        steps.push((
            "wandered away jobless",
            Box::new(move |k: &mut Kitten| {
                k.job = None;
                k.location = loc;
            }),
        ));
        steps.push((
            "came home jobless",
            Box::new(|k: &mut Kitten| k.location = Location::Home),
        ));
    }
    steps.push((
        "assigned again",
        Box::new(|k: &mut Kitten| k.job = Some(JobId::ALL[1])),
    ));
    steps.push(("unassigned", Box::new(|k: &mut Kitten| k.job = None)));

    for (what, step) in steps {
        step(&mut state.kittens[0]);
        let d = draw(&mut term, &state, &view, prev.as_ref());
        let space = list_space(&d.layout);
        prev = Some(d.layout);
        let width = space.width as usize;
        let lines = cell_lines(&d.buffer, space);
        for (k, kitten) in state.kittens.iter().enumerate() {
            let word = expected_state_word(kitten);
            let line = lines
                .iter()
                .find(|l| cells_text(l, 4, width).starts_with(&format!("{} ", kitten.name)))
                .unwrap_or_else(|| panic!("fixture: kitten {k}'s row is drawn after {what}"));
            assert_eq!(
                cells_text(line, width - place, width),
                format!("{word:<place$}"),
                "after kitten 0 {what}, kitten {k}'s row names {word:?} on the next frame \
                 (TUI-NAV-101): {:?}",
                line.concat()
            );
        }
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-111, TUI-NAV-113, TUI-NAV-120 — the done group (Ideas), drawn
// ---------------------------------------------------------------------------

/// Whether the core names `row`'s item already the player's.
fn is_done(state: &GameState, row: &Row) -> bool {
    match &row.key {
        RowKey::Content(id, _) => {
            state.lock_reason(*id) == Some(clowder_core::LockReason::AlreadyYours)
        }
        _ => false,
    }
}

/// `d` pressed as the player presses it, the repeat clock cleared after.
fn press_done_key(state: &GameState, view: &mut ViewState) {
    let d = nav::keys::resolve(
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('d'),
            crossterm::event::KeyModifiers::NONE,
        ),
        view,
    );
    nav::update(d, view, state);
    view.last_repeat = None;
}

/// Where `view`'s focused row is listed but folded into the done group, open
/// the group by its key, keeping the focus on that row.
fn unfold_focus(state: &GameState, view: &mut ViewState) {
    let Some(section) = view.section else { return };
    let Some(key) = view.focus[section.index()].clone() else {
        return;
    };
    if drawn_rows(state, section, view)
        .iter()
        .any(|r| r.key == key)
        || !rows(state, section).iter().any(|r| r.key == key)
    {
        return;
    }
    press_done_key(state, view);
    view.focus[section.index()] = Some(key);
}

/// The every-group meadow with part of its Ideas screen done: every other idea
/// no other idea needs left unresearched, every other refinement on offer
/// bought, and one rule adopted from each of two pairs.
fn ideas_part_done() -> GameState {
    use clowder_core::content::{idea_def, refinement_def, rule_def};
    use clowder_core::{IdeaId, RefinementId, RuleId};
    let mut s = every_group_listed();
    let leaves: Vec<IdeaId> = IdeaId::ALL
        .iter()
        .copied()
        .filter(|i| !IdeaId::ALL.iter().any(|j| idea_def(*j).prereqs.contains(i)))
        .collect();
    for (n, i) in leaves.iter().enumerate() {
        if n % 2 == 0 {
            s.ideas.remove(i);
        }
    }
    let mut n = 0;
    for r in RefinementId::ALL {
        if s.ideas.contains(&refinement_def(r).gate) {
            if n % 2 == 0 {
                s.refinements.insert(r);
            }
            n += 1;
        }
    }
    let mut paired: Vec<RuleId> = Vec::new();
    let mut pair = 0;
    for r in RuleId::ALL {
        if paired.contains(&r) {
            continue;
        }
        paired.push(r);
        paired.push(rule_def(r).blocks);
        if pair % 2 == 0 {
            s.rules.insert(r);
        }
        pair += 1;
    }
    s
}

/// The every-group meadow with every row of its Ideas screen done.
fn ideas_all_done() -> GameState {
    use clowder_core::content::rule_def;
    use clowder_core::{RefinementId, RuleId};
    let mut s = every_group_listed();
    s.refinements = RefinementId::ALL.iter().copied().collect();
    let mut paired: Vec<RuleId> = Vec::new();
    for r in RuleId::ALL {
        if !paired.contains(&r) {
            paired.push(r);
            paired.push(rule_def(r).blocks);
            s.rules.insert(r);
        }
    }
    s
}

/// The every-group meadow with nothing on its Ideas screen done.
fn ideas_none_done() -> GameState {
    let mut s = every_group_listed();
    s.ideas = BTreeSet::new();
    s.refinements = BTreeSet::new();
    s.rules = BTreeSet::new();
    s
}

/// The Ideas screen entered as a player enters it, its group open or folded,
/// and the cursor then put on `at` where one is given.
fn ideas_view(state: &GameState, open: bool, at: Option<RowKey>) -> ViewState {
    let mut v = ViewState::default();
    nav::opening(&mut v, state);
    nav::update(nav::Action::SectionJump(Section::Ideas), &mut v, state);
    if open {
        press_done_key(state, &mut v);
    }
    if let Some(k) = at {
        v.focus[Section::Ideas.index()] = Some(k);
    }
    v
}

/// The words of every drawn line of the list that reads as the done header.
fn done_headers_drawn(d: &Drawn) -> Vec<String> {
    list_lines(d)
        .iter()
        .filter(|l| l.starts_with("\u{2500}\u{2500} done"))
        .map(|l| words_of(l))
        .collect()
}

/// The done header's words are `done`, the number of rows its group holds in
/// parentheses, and ` · d to show` while folded or ` · d to fold` while open,
/// drawn as one header at the supported floor and wide; the count is the
/// group's rows, the same folded or open, and is re-derived on the frame after
/// another row finishes. With nothing finished there is no done header.
///
/// @spec TUI-NAV-111
#[test]
fn the_done_header_counts_its_rows_and_names_its_key_folded_and_open() {
    for (label, state) in [
        ("part done", ideas_part_done()),
        ("all done", ideas_all_done()),
    ] {
        let n = rows(&state, Section::Ideas)
            .iter()
            .filter(|r| is_done(&state, r))
            .count();
        assert!(n > 0, "fixture: {label} has finished Ideas rows");
        for (w, h) in [(FLOOR_W, FLOOR_H), (WIDE, TALL)] {
            for open in [false, true] {
                let want = format!(
                    "done ({n}) \u{b7} d to {}",
                    if open { "fold" } else { "show" }
                );
                // The cursor sits next to the header so it is in view: the
                // last row drawn above it while folded, its first row open.
                let probe = ideas_view(&state, open, None);
                let drawn = drawn_rows(&state, Section::Ideas, &probe);
                let header = drawn
                    .iter()
                    .rposition(|r| matches!(r.kind, RowKind::Group))
                    .expect("a header is drawn");
                let at = if open {
                    drawn.get(header + 1).map(|r| r.key.clone())
                } else {
                    drawn[..header]
                        .iter()
                        .rev()
                        .find(|r| r.selectable())
                        .map(|r| r.key.clone())
                };
                let view = ideas_view(&state, open, at);
                let d = cold(&state, &view, w, h);
                assert_eq!(
                    done_headers_drawn(&d),
                    vec![want.clone()],
                    "{label} at {w}x{h} (group open: {open}): the done header is not drawn once as \
                     {want:?}. List:\n{}",
                    list_lines(&d).join("\n")
                );
            }
        }
    }

    // Re-derived on every frame: one more finished row, one more in the count.
    let state = ideas_part_done();
    let view = ideas_view(&state, true, None);
    let n = rows(&state, Section::Ideas)
        .iter()
        .filter(|r| is_done(&state, r))
        .count();
    let mut more = state.clone();
    let next = rows(&state, Section::Ideas)
        .into_iter()
        .find_map(|r| match r.key {
            RowKey::Content(ContentId::Idea(i), _)
                if !is_done(&state, &r) && state.lock_reason(ContentId::Idea(i)).is_none() =>
            {
                Some(i)
            }
            _ => None,
        })
        .expect("fixture: an idea is ready to research");
    more.ideas.insert(next);
    let m = rows(&more, Section::Ideas)
        .iter()
        .filter(|r| is_done(&more, r))
        .count();
    assert!(m > n, "fixture: researching an idea finishes one more row");
    let mut v = view.clone();
    let drawn = drawn_rows(&more, Section::Ideas, &v);
    let header = drawn
        .iter()
        .rposition(|r| matches!(r.kind, RowKind::Group))
        .expect("a header is drawn");
    v.focus[Section::Ideas.index()] = drawn.get(header + 1).map(|r| r.key.clone());
    let d = cold(&more, &v, WIDE, VERY_TALL);
    assert_eq!(
        done_headers_drawn(&d),
        vec![format!("done ({m}) \u{b7} d to fold")],
        "after another row finished the done header did not count it"
    );

    let none = ideas_none_done();
    assert!(
        rows(&none, Section::Ideas).iter().any(|r| r.selectable()),
        "fixture: the Ideas screen lists rows"
    );
    for (w, h) in [(FLOOR_W, FLOOR_H), (WIDE, VERY_TALL)] {
        let d = cold(&none, &ideas_view(&none, false, None), w, h);
        assert!(
            done_headers_drawn(&d).is_empty(),
            "with nothing finished a done header is drawn at {w}x{h}"
        );
    }
}

/// Where the list cannot hold the leading rule, all of the done header's
/// words, and a space, the header leaves out its key clause whole and draws the
/// rest as any header's words are drawn; where it can, it draws them all.
/// Quantified over every list width up to past the longest header, both
/// clauses, and counts of one to four digits.
///
/// @spec TUI-NAV-111
#[test]
fn the_done_headers_key_clause_gives_way_whole_where_the_list_cannot_hold_it() {
    for count in [1usize, 9, 12, 100, 1234] {
        for open in [false, true] {
            let full = format!(
                "done ({count}) \u{b7} d to {}",
                if open { "fold" } else { "show" }
            );
            let short = format!("done ({count})");
            for width in 0..=60usize {
                let got = render::fmt::done_header_line(count, open, width);
                let want = if width > 3 + full.chars().count() {
                    render::fmt::group_header_line(&full, width)
                } else {
                    render::fmt::group_header_line(&short, width)
                };
                assert_eq!(
                    got, want,
                    "the done header holding {count} row(s) (open: {open}) at {width} columns reads \
                     {got:?}, not {want:?}"
                );
            }
        }
    }
}

/// While every row the Ideas section lists is folded, the list draws the done
/// header alone and no cursor, at the supported floor and wide.
///
/// @spec TUI-NAV-120
#[test]
fn with_every_row_folded_the_list_draws_the_done_header_alone_and_no_cursor() {
    let state = ideas_all_done();
    let view = ideas_view(&state, false, None);
    for (w, h) in [(FLOOR_W, FLOOR_H), (WIDE, TALL)] {
        let d = cold(&state, &view, w, h);
        let lines: Vec<String> = list_lines(&d)
            .into_iter()
            .filter(|l| !l.trim().is_empty())
            .collect();
        assert_eq!(
            lines.len(),
            1,
            "at {w}x{h} the all-folded list draws {} line(s):\n{}",
            lines.len(),
            lines.join("\n")
        );
        assert!(
            lines[0].starts_with("\u{2500}\u{2500} done"),
            "at {w}x{h} the one line drawn is not the done header: {:?}",
            lines[0]
        );
        assert!(
            !lines[0].contains('\u{25B8}'),
            "at {w}x{h} a cursor is drawn on the done header"
        );
    }
}

/// While the done group is open its rows are rows like any other: the cursor
/// is drawn on the open row it rests on, and the inspector describes that row,
/// naming its item.
///
/// @spec TUI-NAV-113
#[test]
fn an_open_done_row_is_drawn_under_the_cursor_and_described_by_the_inspector() {
    let state = ideas_part_done();
    let done: Vec<RowKey> = rows(&state, Section::Ideas)
        .into_iter()
        .filter(|r| is_done(&state, r))
        .map(|r| r.key)
        .collect();
    let names = nav::palette::results(&state, "");
    for key in &done {
        let view = ideas_view(&state, true, Some(key.clone()));
        let d = cold(&state, &view, WIDE, TALL);
        let name = &names
            .iter()
            .find(|r| &r.key == key)
            .expect("a finished row is a palette result")
            .name;
        let head: String = name.chars().take(8).collect();
        let cursor_line = list_lines(&d)
            .into_iter()
            .find(|l| l.contains('\u{25B8}'))
            .unwrap_or_else(|| panic!("no cursor is drawn with the open done row {key:?} focused"));
        assert!(
            cursor_line.contains(&head),
            "the cursor is drawn on {cursor_line:?}, not on {key:?}"
        );
        let inspector = match d.layout.body {
            Body::Wide { inspector, .. } => inspector,
            other => panic!("fixture: {WIDE}x{TALL} lays out the inspector pane, not {other:?}"),
        };
        let pane = region_lines(&d.buffer, inspector).join("\n");
        assert!(
            pane.contains(&head),
            "the inspector does not describe the open done row {key:?}:\n{pane}"
        );
    }
}

// ---------------------------------------------------------------------------
// Coats on list rows — TUI-THEME-019, TUI-THEME-020
// ---------------------------------------------------------------------------

fn fg_at(buf: &Buffer, x: u16, y: u16) -> ratatui::style::Color {
    buf.cell((x, y)).map(|c| c.fg).unwrap_or_default()
}

/// A resource's name as a cost field spells it — its stores-row name, the
/// variant spaced at each capital.
fn resource_name(r: ResourceId) -> String {
    let tag = format!("{r:?}");
    let mut out = String::new();
    for (i, c) in tag.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

/// An ungated structure priced in two or more resources, so its row can be
/// partly covered and its cost field holds more than one name — and a second
/// ungated structure for the cursor to rest on.
fn two_priced_structures() -> (StructureId, StructureId) {
    let ungated =
        |s: &StructureId| matches!(structure_def(*s).gate, clowder_core::StructureGate::Always);
    let a = StructureId::ALL
        .into_iter()
        .find(|s| ungated(s) && structure_def(*s).base_costs.len() >= 2)
        .expect("fixture: an ungated structure priced in two resources");
    let b = StructureId::ALL
        .into_iter()
        .find(|s| *s != a && ungated(s))
        .expect("fixture: a second ungated structure");
    (a, b)
}

/// A meadow with just `revealed` on its list and nothing in stock.
fn bare_meadow(revealed: &[StructureId]) -> GameState {
    let mut s = GameState::new(11);
    s.revealed = revealed.iter().map(|b| ContentId::Structure(*b)).collect();
    s
}

/// The Meadow drawn wide, `focus` under the cursor: the buffer and the list.
fn meadow_list(state: &GameState, focus: RowKey) -> (Buffer, Rect) {
    let mut view = ViewState {
        section: Some(Section::Meadow),
        ..Default::default()
    };
    view.focus[Section::Meadow.index()] = Some(focus);
    let drawn = cold(state, &view, 150, 60);
    let list = match drawn.layout.body {
        Body::Wide { list, .. } => list,
        other => panic!("fixture: 150x60 lays out a list pane, not {other:?}"),
    };
    (drawn.buffer, list)
}

/// The list line whose name column — five cells in, after the divider the
/// list's space carries on its edge, the cursor column, and the marker —
/// begins with `name`: its y, and its text.
fn line_named(buf: &Buffer, list: Rect, name: &str) -> (u16, String) {
    let lines = region_lines(buf, list);
    lines
        .iter()
        .enumerate()
        .find(|(_, l)| {
            let column: String = l.chars().skip(5).collect();
            // The name's column pads the name with blanks, so two blanks after
            // it tell `Catnip` from `Catnip Patch`.
            column.starts_with(&format!("{name}  "))
        })
        .map(|(i, l)| (list.y + i as u16, l.clone()))
        .unwrap_or_else(|| {
            panic!(
                "fixture: no list line is named {name:?}:\n{}",
                lines.join("\n")
            )
        })
}

/// The cell column at which `needle` last occurs in `line`.
fn last_at(line: &str, needle: &str) -> usize {
    let at = line
        .rfind(needle)
        .unwrap_or_else(|| panic!("{needle:?} is not on the line {line:?}"));
    line[..at].chars().count()
}

/// A priced structure's row wears its coats on the names in its cost field
/// while its line is `ready` — the figures and the separators keep the row's
/// colour — and wears none at all while its line is `dim`, which recedes
/// whole: a structure short of its price is Teased and dim whether it wears
/// `○` or, half covered, `◐`. (A `partial`-coloured row belongs to a kind
/// with no reveal ladder; the row tests cover its coats.)
///
/// @spec TUI-THEME-019, TUI-THEME-020
#[test]
fn a_priced_row_wears_coats_on_its_cost_names_while_its_state_lets_it() {
    use clowder_tui::theme::{coat, coat_colours, Role};
    use ratatui::style::Modifier;

    let (a, other) = two_priced_structures();
    let a_name = structure_def(a).name;
    let a_id = ContentId::Structure(a);
    let cost = bare_meadow(&[a, other]).cost_of_next(a_id, 1);
    assert!(
        cost.len() >= 2,
        "fixture: {a_name} is priced in two or more resources"
    );

    let mut ready = bare_meadow(&[a, other]);
    for (r, need) in &cost {
        ready.resources[r.ix()] = need * 2.0;
    }
    let mut partial = bare_meadow(&[a, other]);
    for (r, need) in &cost {
        partial.resources[r.ix()] = need * 0.5;
    }
    let dim = bare_meadow(&[a, other]);

    for (what, state, glyph, role) in [
        ("ready", &ready, "●", Role::Ready),
        ("teased, half covered", &partial, "◐", Role::Dim),
        ("not covered", &dim, "○", Role::Dim),
    ] {
        let (buf, list) = meadow_list(state, RowKey::Content(ContentId::Structure(other), ""));
        let (y, line) = line_named(&buf, list, a_name);
        let marker_x = list.x + 3;
        let marker = buf
            .cell((marker_x, y))
            .expect("a cell")
            .symbol()
            .to_string();
        assert_eq!(
            marker, glyph,
            "fixture: the {what} row wears {glyph}: {line:?}"
        );
        let role_colour = role.color().expect("a state colour");
        assert_eq!(
            fg_at(&buf, marker_x, y),
            role_colour,
            "fixture: the {what} marker wears its colour"
        );
        assert!(
            !buf.cell((marker_x, y))
                .expect("a cell")
                .modifier
                .contains(Modifier::REVERSED),
            "fixture: {a_name} is not the focused row"
        );

        for (r, _) in &cost {
            let name = resource_name(*r);
            let cx = last_at(&line, &format!(" {name}")) + 1;
            let expected = coat_colours(coat(*r), name.chars().count());
            for (k, want) in expected.iter().enumerate() {
                let got = fg_at(&buf, list.x + (cx + k) as u16, y);
                if role == Role::Dim {
                    assert_eq!(
                        got, role_colour,
                        "the {what} row's cost name {name}, character {k}, wears {got:?}; a dim row \
                         recedes whole: {line:?}"
                    );
                } else {
                    assert_eq!(
                        &got, want,
                        "the {what} row's cost name {name}, character {k}, wears {got:?}, not its \
                         coat's {want:?}: {line:?}"
                    );
                }
            }
            // The figure before the name, and the blank between, wear the row's colour.
            for x in [cx - 2, cx - 1] {
                let got = fg_at(&buf, list.x + x as u16, y);
                assert_eq!(
                    got, role_colour,
                    "on the {what} row, cell {x} before {name} wears {got:?}, not the row's colour: \
                     {line:?}"
                );
            }
        }
    }
}

/// The focused row reverses whole and wears no coat: focus is the terminal's
/// own pair, and a coat under a reversal is a block of colour the design never
/// chose.
///
/// @spec TUI-THEME-020
#[test]
fn the_focused_row_reverses_whole_and_wears_no_coat() {
    use clowder_tui::theme::Hue;
    use ratatui::style::Modifier;

    let (a, other) = two_priced_structures();
    let a_name = structure_def(a).name;
    let mut state = bare_meadow(&[a, other]);
    for (r, need) in state.cost_of_next(ContentId::Structure(a), 1) {
        state.resources[r.ix()] = need * 2.0;
    }
    let (buf, list) = meadow_list(&state, RowKey::Content(ContentId::Structure(a), ""));
    let (y, line) = line_named(&buf, list, a_name);
    assert_eq!(
        buf.cell((list.x + 1, y)).expect("a cell").symbol(),
        "▸",
        "fixture: {a_name} is focused"
    );
    assert!(
        buf.cell((list.x + 3, y))
            .expect("a cell")
            .modifier
            .contains(Modifier::REVERSED),
        "fixture: the focused row is reversed"
    );
    let ring: Vec<ratatui::style::Color> = Hue::RING.iter().map(|h| h.color()).collect();
    for x in list.x..list.right() {
        let got = fg_at(&buf, x, y);
        assert!(
            !ring.contains(&got),
            "the focused row wears a coat colour {got:?} at cell {} under its reversal: {line:?}",
            x - list.x
        );
    }
}

/// A stores row wears its coat on its own name, which is the name the sidebar
/// and every cost field spell.
///
/// @spec TUI-THEME-019
#[test]
fn a_stores_row_wears_its_coat_on_its_name() {
    use clowder_tui::theme::{coat, coat_colours};

    let (a, other) = two_priced_structures();
    let mut state = bare_meadow(&[a, other]);
    let (r, _) = state.cost_of_next(ContentId::Structure(a), 1)[0];
    state.resources[r.ix()] = 5.0;
    let (buf, list) = meadow_list(&state, RowKey::Content(ContentId::Structure(other), ""));
    let name = resource_name(r);
    let (y, line) = line_named(&buf, list, &name);
    let expected = coat_colours(coat(r), name.chars().count());
    for (k, want) in expected.iter().enumerate() {
        let got = fg_at(&buf, list.x + 5 + k as u16, y);
        assert_eq!(
            &got, want,
            "the stores row for {name}, character {k}, wears {got:?}, not its coat's {want:?}: {line:?}"
        );
    }
}
