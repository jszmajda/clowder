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
    let mut v = ViewState::default();
    v.section = Some(Section::Meadow);
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
    let bordered = |l: &String| l.chars().next().is_some_and(is_box);
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
    state.kittens = roster(400);

    let short_list = {
        let mut v = ViewState::default();
        v.section = Some(Section::Meadow);
        v
    };
    let long_list = {
        let mut v = ViewState::default();
        v.section = Some(Section::Clowder);
        v
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

