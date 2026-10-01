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

use nav::rows::{rows, Row, RowKey, RowKind};
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
    s.revealed = structures.iter().map(|b| ContentId::Structure(*b)).collect::<BTreeSet<_>>();
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

    for (w, h) in [(FLOOR_W, FLOOR_H), (WIDE, TALL), (VERY_TALL + 20, VERY_TALL)] {
        let first = cold(&state, &view, w, h);
        let second = cold(&state, &view, w, h);
        assert_eq!(
            first.buffer, second.buffer,
            "{w}x{h}: two frames from identical inputs must be identical"
        );
        assert_eq!(first.layout, second.layout, "{w}x{h}: and so must the layouts they used");
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
    later.revealed = StructureId::ALL.iter().map(|b| ContentId::Structure(*b)).collect();

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
    assert_eq!(again.buffer, a.buffer, "the snapshot is the input; nothing else moved");
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

    assert_eq!(save_string(&state, 0), state_before, "the frame must not touch the snapshot");
    assert_eq!(format!("{view:?}"), view_before, "the frame must not touch the view state");
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

    for (w, h) in [(FLOOR_W, FLOOR_H), (99, 30), (100, 30), (119, 30), (WIDE, TALL), (200, 60)] {
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
        let (above, at, below) =
            (StructureId::ALL[ix - 1], StructureId::ALL[ix], StructureId::ALL[ix + 1]);
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
    let before = meadow(&[StructureId::ALL[0], StructureId::ALL[1], last], ResourceId::ALL.len());
    // After: fifty rows have been revealed above the very same focused row.
    let after = long_meadow();

    let inserted = row_index(&after, &structure_row(last)) - row_index(&before, &structure_row(last));
    assert!(inserted > 30, "fixture: the reveal should insert far more rows than a viewport holds");

    let mut term = terminal(WIDE, TALL);
    let first = draw(&mut term, &before, &view, None);
    assert!(shows(&list_lines(&first), &name), "the cursor starts visible");

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

    for (w, h) in [(WIDE, TALL), (FLOOR_W, FLOOR_H), (WIDE, VERY_TALL), (FLOOR_W, 30)] {
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

    assert!(shows(&list_lines(&d), &probe(deep)), "the cursor's row is drawn");

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

    let top = cold(&state, &looking_at(RowKey::Fixed("GatherCatnip")), WIDE, TALL);
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
    s.rites = [RiteId::TheReverie, RiteId::Enlightenment].into_iter().collect::<BTreeSet<_>>();
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
        RowKey::Content(ContentId::Craft(_), _) => panic!("the one-group section's rows wear no header"),
        RowKey::Resource(_) => "stores",
        RowKey::Kitten(..) => "kittens",
        RowKey::Colony(..) => "colonies",
        RowKey::Site(_) => "sites",
        RowKey::Adventure(_) => "adventures",
    }
}

/// Every (section, header words) the design gives a several-group section.
fn designed_headers() -> BTreeSet<(usize, &'static str)> {
    let table: [(Section, &[&str]); 9] = [
        (Section::Meadow, &["things to do", "structures", "stores"]),
        (Section::Clowder, &["jobs", "kittens", "things to do"]),
        (Section::Ideas, &["ideas", "refinements", "rules"]),
        (Section::Sunbeams, &["things to do", "rites"]),
        (Section::Wonders, &["things to do", "arches"]),
        (Section::Friends, &["colonies", "things to do"]),
        (Section::Sky, &["sites", "things to do"]),
        (Section::Dreams, &["things to do", "loom upgrades"]),
        (Section::Legacy, &["things to do", "traditions", "adventures"]),
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
fn header_above(state: &GameState, section: Section, beneath: &RowKey, w: u16, h: u16) -> HeaderLine {
    let mut view = ViewState { section: Some(section), ..Default::default() };
    view.focus[section.index()] = Some(beneath.clone());
    let d = cold(state, &view, w, h);
    let rect = list_rect(&d.layout);
    let lines = region_lines(&d.buffer, rect);
    let at = lines
        .iter()
        .position(|l| l.starts_with('\u{25B8}'))
        .unwrap_or_else(|| panic!("the cursor's row is not drawn in {}'s pane:\n{}", section.title(), lines.join("\n")));
    assert!(at > 0, "the cursor's row is the pane's first line; no header can stand above it");
    let y = rect.y + (at - 1) as u16;
    let cells = (rect.x..rect.right())
        .map(|x| d.buffer.cell((x, y)).expect("inside the buffer").clone())
        .collect();
    HeaderLine { text: lines[at - 1].clone(), cells }
}

/// Every header the row list carries for `state`, as (section, the row
/// directly beneath it).
fn headers_of(state: &GameState) -> Vec<(Section, Row)> {
    let mut out = Vec::new();
    for section in Section::ALL {
        let list = rows(state, section);
        for (i, row) in list.iter().enumerate() {
            if matches!(row.kind, RowKind::Group) {
                let next = list
                    .get(i + 1)
                    .unwrap_or_else(|| panic!("{}'s header at line {i} stands above nothing", section.title()));
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
/// a meadow that lists every group the design names, which the test checks.
///
/// @spec TUI-NAV-100
#[test]
fn every_group_header_is_worded_as_the_plain_plural_of_its_kind() {
    let state = every_group_listed();
    let mut seen: BTreeSet<(usize, &'static str)> = BTreeSet::new();

    for (section, beneath) in headers_of(&state) {
        let expected = header_words_for(&beneath);
        let line = header_above(&state, section, &beneath.key, WIDE, TALL);
        let words = words_of(&line.text);
        assert_eq!(
            words, expected,
            "{}'s header over {:?} reads {:?}; the design words it {expected:?}. Line: {:?}",
            section.title(),
            beneath.key,
            words,
            line.text
        );
        assert!(
            !words.chars().any(|c| c.is_ascii_digit() || c.is_uppercase()),
            "{}'s header {words:?} carries a figure or a capital",
            section.title()
        );
        seen.insert((section.index(), expected));
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
    let longest = designed_headers().iter().map(|(_, w)| w.chars().count()).max().unwrap_or(0);

    for w in [FLOOR_W, 85, 99, 100, 104, 119, 120, 121, WIDE, 160, 200] {
        for (section, beneath) in &headers {
            let line = header_above(&state, *section, &beneath.key, w, TALL);
            let width = line.cells.len();
            assert!(
                width > 3 + longest,
                "fixture: at {w} columns the list is {width} wide, too narrow to hold every header whole"
            );
            let words = words_of(&line.text);
            assert!(!words.is_empty(), "{}'s header at {w} columns carries no words: {:?}", section.title(), line.text);
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
    assert!(meadow_rows(&small).len() < fitting.len(), "fixture: this list must fit");
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

    assert!(meadow_rows(&state).len() > over_lines.len(), "fixture: this list overflows at {TALL}");
    assert!(meadow_rows(&state).len() < fit_lines.len(), "fixture: it fits at {VERY_TALL}");
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
    assert!(height > 4, "fixture: a pane tall enough for the question to mean anything");

    for x in 0..rect.width as usize {
        let column: Vec<char> = lines.iter().filter_map(|l| l.chars().nth(x)).collect();
        let bars = column.iter().filter(|c| BAR_GLYPHS.contains(**c)).count();
        if bars * 10 < height * 9 {
            continue;
        }
        let distinct: BTreeSet<char> =
            column.iter().copied().filter(|c| BAR_GLYPHS.contains(*c)).collect();
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

    let per_drawn_row = (long_when_tall - long_at_floor).as_secs_f64() / f64::from(extra_visible_rows);
    let hidden_rows = (long_rows - short_rows) as f64;
    let per_hidden_row =
        (long_at_floor.saturating_sub(short_at_floor)).as_secs_f64() / hidden_rows;

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
    let d = cold(&state, &looking_at(structure_row(StructureId::ALL[25])), WIDE, TALL);
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
                .map(|x| buf.cell((x, y)).map(|c| c.symbol().to_string()).unwrap_or_else(|| " ".into()))
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
    rows(state, section).iter().find(|r| same_item(&r.key)).filter(|first| &first.key != key).map(|_| label)
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
    let mut view = ViewState { section: Some(section), ..Default::default() };
    view.focus[section.index()] = Some(key.clone());
    let mut term = terminal(w, h);
    term.draw(|f| inspector::draw(f, area(w, h), state, &view)).expect("TestBackend is infallible");
    cell_lines(term.backend().buffer(), area(w, h))[0].concat().trim().to_string()
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
    6 + 1 + names.values().map(|n| n.chars().count()).max().expect("a vocabulary") + 3
}

/// Whether a section can list a row whose description states an owned count
/// (TUI-NAV-018): the copies of a structure or a rite, the kittens on a job's
/// count row, the tinkerers on a craft's tinkerer row. Judged by the section
/// alone, from what its rows are — checked against real lists below.
fn gives_count_field(section: Section) -> bool {
    matches!(section, Section::Meadow | Section::Clowder | Section::Crafts | Section::Sunbeams)
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
    let Some(cost) = cost else { return (" ".repeat(width), false) };
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
    assert!(!text.is_empty(), "the field holds the widest single component and `, …` after it");
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
    let mut roster: Vec<Kitten> =
        (0..MEADOW_MOVE_MIN_KITTENS + 9).map(|i| kitten_named(&format!("kitten-{i}"), None)).collect();
    for k in roster.iter_mut().take(4) {
        k.job = Some(JobId::Tinkerer);
    }
    roster[0].tinkering = Some(CraftId::ALL[0]);
    s.kittens = roster;
    s.elder = Some(0);
    s.peak_devotion = 1.0e12;
    s.devotion = 1.0e6;
    s.serenity = 1.0e12;
    s.rites = [RiteId::TheReverie, RiteId::Enlightenment].into_iter().collect();
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
    let meadows = [rich_meadow(), poor_meadow(), ragged_meadow(), GameState::new(11)];

    // The per-section table is the section's own: check it against what the
    // sections really list, over every meadow here.
    for section in Section::ALL {
        let lists_count = meadows.iter().any(|s| rows(s, section).iter().any(|r| stated_count(s, r).is_some()));
        let lists_cost = meadows.iter().any(|s| rows(s, section).iter().any(|r| stated_cost(s, r).is_some()));
        assert_eq!(lists_count, gives_count_field(section), "fixture: {section:?} and owned counts");
        assert_eq!(lists_cost, gives_cost_field(section), "fixture: {section:?} and next costs");
    }

    let mut seen = (0usize, 0usize, 0usize, 0usize); // rows, cut costs, marked counts, blank zero counts
    let mut name_cache: HashMap<(usize, Section, String), String> = HashMap::new();
    for (mi, state) in meadows.iter().enumerate() {
        for section in Section::ALL {
            let all = rows(state, section);
            let selectable: Vec<usize> = (0..all.len()).filter(|i| all[*i].selectable()).collect();
            let (Some(&first), Some(&last)) = (selectable.first(), selectable.last()) else { continue };
            for (w, h) in [(FLOOR_W, FLOOR_H), (99, 30), (100, 30), (130, TALL), (200, 60)] {
                for cursor in [first, last] {
                    let mut view = ViewState { section: Some(section), ..Default::default() };
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
                    let name_w = width - 4 - if counted { 8 } else { 0 } - if costed { c + 1 } else { 0 };
                    let count_at = if costed { width - c - 8 } else { width - 7 };

                    for (y, line) in lines.iter().enumerate() {
                        if line[0].chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
                            continue; // an overflow note (TUI-RENDER-011)
                        }
                        let Some(ix) = (cursor + y).checked_sub(cy).filter(|ix| *ix < all.len()) else {
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

                        // A kitten's row may carry the whole `(n: Elder)` hint
                        // after its name where the column has room, drawn there
                        // and nowhere else (TUI-NAV-095) — the dedicated sweep
                        // over every width and gate state is
                        // `every_kitten_row_whose_gate_is_open_carries_the_whole_elder_hint_where_it_fits`;
                        // here it is only accounted for so this generic field
                        // check still holds over the name column it shares.
                        let hint = match row.key {
                            RowKey::Kitten(kix, _)
                                if state.command_gate_open(&clowder_core::Command::AppointElder(kix)) =>
                            {
                                Some("(n: Elder)")
                            }
                            _ => None,
                        };
                        let name_len = full.chars().count();
                        let expected_column = match hint.filter(|h| name_len + 1 + h.chars().count() <= name_w) {
                            Some(h) => {
                                let trailing = name_w - name_len - 1 - h.chars().count();
                                format!("{full} {h}{}", " ".repeat(trailing))
                            }
                            None => {
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
                            assert_eq!(line[4 + name_w], " ", "{at}: a blank column before the first field.\n{drawn:?}");
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
                                assert_eq!(line[count_at + 7], " ", "{at}: a blank column before the cost field.\n{drawn:?}");
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
    assert!(seen.1 > 0, "fixture: some cost too long for its field was drawn");
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
/// kitten, that kitten's row — the cursor's and every other alike — carries
/// `(n: Elder)` after the name, within the name's column, in the dim role, of
/// single-width characters, and never in the count or cost field. The hint is
/// drawn whole or not at all and never costs the name a character: where the
/// name's column holds the whole name, a blank, and the whole hint, the hint
/// is there; where it cannot hold the whole name and the whole hint even set
/// hard against each other, the row carries no part of the hint and draws its
/// name as any name is drawn (TUI-RENDER-014); and in every case the name
/// comes first, whole or shortened as any name is, followed by nothing but
/// blanks and at most the whole hint. A row whose kitten's gate the core
/// reports shut carries none (no reachable meadow lists one: the gate opens
/// with the first kitten). Quantified over every kitten row, with the cursor
/// on each kitten row in turn and on no kitten at all, at the floor, narrow,
/// at the pane breakpoint, three-pane, and wide — with names built at each
/// width to sit just inside, on, and just past where the hint fits, and names
/// longer than the column.
///
/// @spec TUI-NAV-095
#[test]
fn every_kitten_row_whose_gate_is_open_carries_the_whole_elder_hint_where_it_fits() {
    let dim = clowder_tui::theme::Role::Dim.color().expect("the dim role is a colour");
    let store = store_names();
    let c = cost_field_width(&store);
    let hint_w = ELDER_HINT.chars().count();
    assert!(
        gives_count_field(Section::Clowder) && gives_cost_field(Section::Clowder),
        "fixture: the Clowder list gives both fields, so the name column is bounded by both"
    );
    assert!(ELDER_HINT.is_ascii(), "fixture: the hint as the spec words it is single-width");

    let (mut drawn_with, mut drawn_without, mut focused_with) = (0u32, 0u32, 0u32);
    for (w, h) in [(FLOOR_W, 60u16), (90, 60), (99, 60), (100, 60), (130, 60), (200, 60)] {
        // The name column's width, taken from the frame's own list space.
        let probe = clowder_named(&["Biscuit".to_string()]);
        let space = list_space(&cold(&probe, &ViewState { section: Some(Section::Clowder), ..Default::default() }, w, h).layout);
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
        let state = clowder_named(&names);
        let all = rows(&state, Section::Clowder);
        let kitten_ix: Vec<usize> = (0..all.len()).filter(|i| matches!(all[*i].key, RowKey::Kitten(..))).collect();
        assert!(!kitten_ix.is_empty(), "fixture: kitten rows are listed at {w}x{h}");

        let mut cursors: Vec<Option<RowKey>> = kitten_ix.iter().map(|i| Some(all[*i].key.clone())).collect();
        cursors.push(all.iter().find(|r| r.selectable() && !matches!(r.key, RowKey::Kitten(..))).map(|r| r.key.clone()));

        for cursor_key in cursors {
            let mut view = ViewState { section: Some(Section::Clowder), ..Default::default() };
            view.focus[Section::Clowder.index()] = cursor_key.clone();
            let d = cold(&state, &view, w, h);
            let space = list_space(&d.layout);
            let lines = cell_lines(&d.buffer, space);
            let cursor = cursor_key.as_ref().and_then(|k| all.iter().position(|r| &r.key == k)).unwrap_or(0);
            let cy = lines
                .iter()
                .position(|l| l[0] == "▸")
                .unwrap_or_else(|| panic!("fixture: the cursor is drawn in the Clowder list at {w}x{h}"));

            for &ri in &kitten_ix {
                let row = &all[ri];
                let RowKey::Kitten(k, _) = row.key else { unreachable!() };
                let y = (cy + ri).checked_sub(cursor).expect("fixture: the list is drawn unscrolled");
                let line = &lines[y];
                let name = &state.kittens[k].name;
                let name_len = name.chars().count();
                let focused = cursor_key.as_ref() == Some(&row.key);
                let at = format!(
                    "at {w}x{h}, kitten {k}'s row (a {name_len}-letter name in a {name_w}-column name \
                     column, {}) reads {:?}",
                    if focused { "the cursor's" } else { "not the cursor's" },
                    line.concat()
                );
                let column = cells_text(line, 4, 4 + name_w);
                let fields = cells_text(line, 4 + name_w, width);

                // Never in the count or cost field: those draw what the
                // inspector states, and nothing of the hint.
                let (cost_text, _) = cost_field(stated_cost(&state, row).as_deref(), &store, c);
                assert_eq!(
                    fields,
                    format!(" {} {cost_text}", count_field(stated_count(&state, row))),
                    "{at}: the count and cost fields draw what the inspector states there, and \
                     the hint never stands in either"
                );

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
                    assert!(between.chars().all(|ch| ch == ' '), "{at}: something other than blanks between the name and the hint");
                    let after = &column[off + ELDER_HINT.len()..];
                    assert!(after.chars().all(|ch| ch == ' '), "{at}: something other than blanks after the hint");
                    let cell0 = 4 + column[..off].chars().count();
                    for (i, sym) in line[cell0..cell0 + hint_w].iter().enumerate() {
                        assert_eq!(sym.chars().count(), 1, "{at}: the hint's cell {i} is not one single-width character");
                        let fg = d.buffer.cell((space.x + (cell0 + i) as u16, space.y + y as u16)).expect("in the buffer").fg;
                        assert_eq!(fg, dim, "{at}: the hint's cell {i} ({sym:?}) is not drawn in the dim role");
                    }
                }

                let gate = state.command_gate_open(&clowder_core::Command::AppointElder(k));
                if !gate {
                    assert!(hint_at.is_none(), "{at}: the core reports this kitten's gate shut, so the row carries no hint");
                } else if name_len + 1 + hint_w <= name_w {
                    assert!(
                        hint_at.is_some(),
                        "{at}: the gate is open and the name's column holds the whole name, a \
                         blank, and the whole `{ELDER_HINT}`, so the row carries the hint"
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
    assert!(drawn_with > 0, "fixture: no row was wide enough to require the hint");
    assert!(focused_with > 0, "fixture: the cursor's own row never required the hint");
    assert!(drawn_without > 0, "fixture: no row was too narrow to hold the hint");
}
