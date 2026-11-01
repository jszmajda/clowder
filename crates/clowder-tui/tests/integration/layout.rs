//! Where things go: the chrome algebra, the width breakpoints, and the
//! dead-band that keeps a wobbling pane from thrashing between layouts.
//!
//! The layout is pure geometry — `compute(area, prev)` takes a size and the
//! previous frame's layout and returns rectangles — so every spec in this file
//! is testable at exact sizes with no terminal involved. That testability is
//! the stated reason the frame is a pure function at all (render design,
//! "Context and Design Philosophy"), so these tests exercise the design's
//! reasons, not just its return values:
//!
//! * the regions are asserted to *sum* to the terminal's height at many sizes,
//!   because the design states the body as a formula (`height − 3 − T`)
//!   precisely so a table of per-size numbers cannot drift out of step;
//! * the wide layout is asserted to hand every surplus column to the inspector,
//!   because a list row stretched across 150 columns puts its name and its cost
//!   a saccade apart;
//! * the dead-band is asserted to be *asymmetric* — the same width gives a
//!   different layout depending on which side it was approached from — because
//!   a symmetric threshold is exactly what flickers when a drag-resize
//!   overshoots and settles.
//!
//! `clowder-tui` is a binary with no library target, so the module under test
//! is included by path — the real implementation file, not a copy.

use clowder_tui::render::layout;

use layout::{compute, Body, Layout};
use ratatui::layout::Rect;

// ---------------------------------------------------------------------------
// Constants
//
// The floor and the two breakpoints are stated by the specs and the design.
// The sidebar's exact width and the list's cap are explicitly deferred to
// measurement (render design, Deferred 1), so nothing below pins them to a
// number: they are bounded by what could not possibly be right, and every
// substantive claim about them is relational (constant across widths, capped
// beyond 120, surplus to the inspector).
// ---------------------------------------------------------------------------

/// TUI-RENDER-004: the supported floor.
const FLOOR_W: u16 = 80;
const FLOOR_H: u16 = 24;

/// TUI-RENDER-005/006: where the inspector stops being a swap and becomes a pane.
const PANE_BREAKPOINT: u16 = 100;

/// TUI-RENDER-007: where the list stops growing.
const CAP_BREAKPOINT: u16 = 120;

/// TUI-RENDER-008: the design says a layout is left "a few columns below" the
/// boundary it was entered at. The exact width is deferred, but it cannot be
/// wide enough to leave the 80–99 range reachable as a three-pane layout —
/// TUI-RENDER-005 governs that range — so the band is bounded here.
const MAX_DEAD_BAND: u16 = 8;

/// Sanity bounds on the fixed sidebar. Under this it cannot hold a resource
/// name beside a compact number; over it, the 80-column floor has nothing left
/// for a list.
const SIDEBAR_MIN: u16 = 12;
const SIDEBAR_MAX: u16 = 32;

/// Sanity bounds on the list's cap. Under this a row cannot hold a name, its
/// markers and its cost; over it — wider than an entire floor-sized terminal —
/// the cap has stopped meaning "readable in one glance", which is the whole
/// reason TUI-RENDER-007 exists.
const LIST_CAP_MIN: u16 = 32;
const LIST_CAP_MAX: u16 = 80;

/// Chrome rows that are not the ticker: header, tab bar, hint bar.
const FIXED_CHROME_ROWS: u16 = 3;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn area(width: u16, height: u16) -> Rect {
    Rect::new(0, 0, width, height)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    TooSmall,
    Narrow,
    Wide,
}

fn mode(l: &Layout) -> Mode {
    match l.body {
        Body::TooSmall(_) => Mode::TooSmall,
        Body::Narrow { .. } => Mode::Narrow,
        Body::Wide { .. } => Mode::Wide,
    }
}

fn narrow_parts(l: &Layout, at: u16) -> (Rect, Rect) {
    match l.body {
        Body::Narrow { sidebar, main } => (sidebar, main),
        other => panic!("expected a sidebar+list body at {at} columns, got {other:?}"),
    }
}

fn wide_parts(l: &Layout, at: u16) -> (Rect, Rect, Rect) {
    match l.body {
        Body::Wide {
            sidebar,
            list,
            inspector,
        } => (sidebar, list, inspector),
        other => panic!("expected a three-pane body at {at} columns, got {other:?}"),
    }
}

/// The body's outer rectangle, asserting on the way that its columns actually
/// tile it: same top and same height, left to right, no gap and no overlap.
fn body_rect(l: &Layout, what: &str) -> Rect {
    match l.body {
        Body::TooSmall(card) => card,
        Body::Narrow { sidebar, main } => {
            assert_columns_tile(&[sidebar, main], what);
            Rect::new(
                sidebar.x,
                sidebar.y,
                sidebar.width + main.width,
                sidebar.height,
            )
        }
        Body::Wide {
            sidebar,
            list,
            inspector,
        } => {
            assert_columns_tile(&[sidebar, list, inspector], what);
            Rect::new(
                sidebar.x,
                sidebar.y,
                sidebar.width + list.width + inspector.width,
                sidebar.height,
            )
        }
    }
}

fn assert_columns_tile(cols: &[Rect], what: &str) {
    for pair in cols.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        assert_eq!(a.y, b.y, "{what}: body columns must share a top edge");
        assert_eq!(
            a.height, b.height,
            "{what}: body columns must share a height"
        );
        assert_eq!(
            a.right(),
            b.x,
            "{what}: body columns must be contiguous — no gap, no overlap ({a:?} then {b:?})"
        );
    }
}

/// Every stated invariant of the vertical frame, at one size.
///
/// The rows must abut in order, span the full width, and sum to exactly the
/// terminal's height — which is the whole point of stating the body as a
/// formula rather than a table.
/// @spec TUI-RENDER-002, TUI-RENDER-003
fn assert_frame_tiles(l: &Layout, a: Rect) {
    let what = format!("{}x{}", a.width, a.height);
    let body = body_rect(l, &what);

    let mut rows: Vec<(&str, Rect)> = vec![("header", l.header), ("tabs", l.tabs)];
    if let Some(banner) = l.banner {
        rows.push(("banner", banner));
    }
    rows.push(("body", body));
    rows.push(("ticker", l.ticker));
    rows.push(("hints", l.hints));

    let mut y = a.y;
    let mut total = 0u16;
    for (name, r) in &rows {
        assert_eq!(
            r.x, a.x,
            "{what}: the {name} row must start at the frame's left edge"
        );
        assert_eq!(
            r.width, a.width,
            "{what}: the {name} row must span the frame's width"
        );
        assert_eq!(
            r.y, y,
            "{what}: the {name} row must abut the region above it"
        );
        y = y.saturating_add(r.height);
        total += r.height;
    }

    assert_eq!(
        total, a.height,
        "{what}: the regions must sum to the terminal's height exactly — {rows:?}"
    );
    assert_eq!(
        y,
        a.y + a.height,
        "{what}: the last row must reach the frame's bottom edge"
    );
}

/// A representative spread of supported sizes: both breakpoints, both sides of
/// each, the floor, and sizes far past anything the layout was tuned at.
fn supported_sizes() -> Vec<Rect> {
    let mut out = Vec::new();
    for w in [
        80u16, 81, 90, 99, 100, 101, 110, 119, 120, 121, 150, 200, 300,
    ] {
        for h in [24u16, 25, 30, 40, 60, 100] {
            out.push(area(w, h));
        }
    }
    out
}

/// Thread `prev` through a sequence of widths, as the loop does frame to frame.
fn walk(widths: &[u16], height: u16) -> Vec<(u16, Layout)> {
    let mut prev: Option<Layout> = None;
    let mut out = Vec::new();
    for &w in widths {
        let l = compute(area(w, height), prev.as_ref(), false);
        prev = Some(l);
        out.push((w, l));
    }
    out
}

// ---------------------------------------------------------------------------
// TUI-RENDER-002 — the chrome algebra
// ---------------------------------------------------------------------------

/// The body is whatever the chrome leaves. Stated as a formula, tested as a
/// sum: at every supported size the regions abut in order and add up to the
/// terminal's height, with nothing lost or double-counted at the seams.
/// @spec TUI-RENDER-002
#[test]
fn the_regions_always_sum_to_the_terminal_height() {
    for a in supported_sizes() {
        let l = compute(a, None, false);
        assert_frame_tiles(&l, a);
    }
}

/// The chrome is constant furniture: one header row, one tab-bar row, one hint
/// row, and a ticker of at least one row. Everything else is body.
/// @spec TUI-RENDER-002
#[test]
fn the_chrome_is_a_header_a_tab_bar_a_ticker_of_one_or_more_rows_and_a_hint_bar() {
    for a in supported_sizes() {
        let what = format!("{}x{}", a.width, a.height);
        let l = compute(a, None, false);
        let body = body_rect(&l, &what);

        assert_eq!(l.header.height, 1, "{what}: the header is one row");
        assert_eq!(l.tabs.height, 1, "{what}: the tab bar is one row");
        assert_eq!(l.hints.height, 1, "{what}: the hint bar is one row");
        assert!(
            l.ticker.height >= 1,
            "{what}: the ticker is one row at the floor and never none"
        );

        let banner_rows = l.banner.map_or(0, |b| b.height);
        assert_eq!(
            body.height + FIXED_CHROME_ROWS + l.ticker.height + banner_rows,
            a.height,
            "{what}: the body is height - 3 - ticker rows, not a per-size constant"
        );
    }
}

/// The floor's arithmetic, stated outright by the design: at 80x24 the ticker
/// is one row and the body is 20.
/// @spec TUI-RENDER-002
#[test]
fn at_the_eighty_by_twenty_four_floor_the_body_is_twenty_rows() {
    let a = area(FLOOR_W, FLOOR_H);
    let l = compute(a, None, false);
    assert_frame_tiles(&l, a);

    assert_eq!(
        l.ticker.height, 1,
        "at the floor the ticker collapses to a single row"
    );
    assert_eq!(
        body_rect(&l, "80x24").height,
        20,
        "header 1 + tabs 1 + body 20 + ticker 1 + hints 1 = 24"
    );
}

/// A row added to the terminal is a row added below the tab bar, never to the
/// fixed chrome. (The ticker may earn rows of its own where height allows —
/// that is deferred — so the claim under test is that the header, tab bar and
/// hint bar never absorb the growth.)
/// @spec TUI-RENDER-002
#[test]
fn extra_terminal_rows_go_to_the_body_and_ticker_not_to_the_fixed_chrome() {
    for w in [FLOOR_W, 100u16, 150] {
        for h in FLOOR_H..FLOOR_H + 20 {
            let short = compute(area(w, h), None, false);
            let tall = compute(area(w, h + 1), None, false);

            let short_flexible = body_rect(&short, "short").height + short.ticker.height;
            let tall_flexible = body_rect(&tall, "tall").height + tall.ticker.height;

            assert_eq!(
                tall_flexible,
                short_flexible + 1,
                "at {w} columns, growing from {h} to {} rows must add the row below the tab bar",
                h + 1
            );
            assert_eq!(
                short.header.height + short.tabs.height + short.hints.height,
                3
            );
            assert_eq!(tall.header.height + tall.tabs.height + tall.hints.height, 3);
        }
    }
}

// ---------------------------------------------------------------------------
// TUI-RENDER-003 — the offline banner
// ---------------------------------------------------------------------------

/// The banner takes its row from the body rather than adding a row to the
/// frame: with a banner present the frame still sums to the terminal's height,
/// and the row it occupies has come out of the body.
///
/// NOTE: `compute(area, prev)` is given no way to know a banner is present, so
/// this test can only assert the invariant for whatever `compute` returns — it
/// cannot drive the condition. See openQuestions.
/// @spec TUI-RENDER-003
#[test]
fn an_offline_banner_takes_its_row_from_the_body_not_from_the_frame() {
    for a in supported_sizes() {
        let what = format!("{}x{}", a.width, a.height);
        let l = compute(a, None, false);
        assert_frame_tiles(&l, a);

        if let Some(banner) = l.banner {
            assert_eq!(banner.height, 1, "{what}: the banner is one row");
            assert_eq!(
                banner.y,
                l.tabs.y + l.tabs.height,
                "{what}: the banner sits under the tab bar"
            );

            // The frame already sums to the terminal's height (above), and the
            // rest of the chrome is fixed at one row each with the ticker at
            // its floor — so the banner's row can only have come out of the
            // body.
            assert_eq!(l.header.height + l.tabs.height + l.hints.height, 3);
            assert!(l.ticker.height >= 1);
            assert_eq!(
                body_rect(&l, &what).height + FIXED_CHROME_ROWS + l.ticker.height + 1,
                a.height,
                "{what}: the banner's row comes out of the body, never out of the frame"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// TUI-RENDER-004 — the too-small card
// ---------------------------------------------------------------------------

/// Too-small is *both* dimensions: under 80 columns or under 24 rows, however
/// generous the other one is.
/// @spec TUI-RENDER-004
#[test]
fn under_eighty_columns_or_twenty_four_rows_only_the_too_small_card_is_drawn() {
    let too_small = [
        (79u16, 24u16),
        (79, 100),
        (40, 24),
        (1, 1),
        (80, 23),
        (300, 23),
        (200, 1),
        (79, 23),
    ];

    for (w, h) in too_small {
        let a = area(w, h);
        let l = compute(a, None, false);
        match l.body {
            Body::TooSmall(card) => assert_eq!(
                card, a,
                "{w}x{h}: the too-small card is the whole frame — nothing else is drawn"
            ),
            other => panic!("{w}x{h} is below the 80x24 floor but laid out as {other:?}"),
        }
    }
}

/// At exactly the floor the full frame appears: 80x24 is supported, not
/// too small.
/// @spec TUI-RENDER-004
#[test]
fn eighty_by_twenty_four_exactly_is_supported() {
    for (w, h) in [
        (FLOOR_W, FLOOR_H),
        (FLOOR_W, FLOOR_H + 1),
        (FLOOR_W + 1, FLOOR_H),
    ] {
        let l = compute(area(w, h), None, false);
        assert_ne!(
            mode(&l),
            Mode::TooSmall,
            "{w}x{h} is at or above the floor and must draw the full frame"
        );
    }
}

/// "Nothing else is drawn" is a claim about the layout, which is the plan of
/// what gets drawn: below the floor no chrome region may claim a row.
/// @spec TUI-RENDER-004
#[test]
fn the_too_small_card_leaves_no_room_for_chrome() {
    let l = compute(area(60, 20), None, false);
    assert_eq!(l.header.height, 0, "no header is drawn below the floor");
    assert_eq!(l.tabs.height, 0, "no tab bar is drawn below the floor");
    assert_eq!(l.ticker.height, 0, "no ticker is drawn below the floor");
    assert_eq!(l.hints.height, 0, "no hint bar is drawn below the floor");
    assert_eq!(l.banner, None, "no banner is drawn below the floor");
}

/// The floor is a hard threshold, not a breakpoint: the dead-band applies at
/// the two width breakpoints (100 and 120), and a sticky floor would plan rows
/// a terminal genuinely cannot hold.
/// @spec TUI-RENDER-004
#[test]
fn the_floor_carries_no_dead_band() {
    let full = compute(area(120, 40), None, false);
    assert_eq!(mode(&full), Mode::Wide, "fixture: a comfortable terminal");

    for (w, h) in [(79u16, 40u16), (120, 23), (79, 23)] {
        let shrunk = compute(area(w, h), Some(&full), false);
        assert_eq!(
            mode(&shrunk),
            Mode::TooSmall,
            "{w}x{h}: shrinking below the floor shows the card immediately, whatever came before"
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-RENDER-005 — 80 to 99 columns
// ---------------------------------------------------------------------------

/// Between 80 and 99 columns the body is a sidebar and one other region, and
/// the two tile it.
/// @spec TUI-RENDER-005
#[test]
fn between_eighty_and_ninety_nine_columns_the_body_is_a_sidebar_and_a_list() {
    for w in FLOOR_W..PANE_BREAKPOINT {
        let l = compute(area(w, FLOOR_H), None, false);
        let (sidebar, main) = narrow_parts(&l, w);

        assert!(
            (SIDEBAR_MIN..=SIDEBAR_MAX).contains(&sidebar.width),
            "{w} columns: sidebar width {} is outside anything a resource sidebar could want",
            sidebar.width
        );
        assert!(main.width > 0, "{w} columns: the list must have room");
        assert_eq!(
            sidebar.width + main.width,
            w,
            "{w} columns: the sidebar and the list divide the whole body"
        );
    }
}

/// There is no third column below 100 — which is precisely why the inspector
/// can only appear *in place of* the list here.
/// @spec TUI-RENDER-005
#[test]
fn below_a_hundred_columns_there_is_no_pane_for_the_inspector_to_occupy() {
    for w in FLOOR_W..PANE_BREAKPOINT {
        let l = compute(area(w, FLOOR_H), None, false);
        assert_eq!(
            mode(&l),
            Mode::Narrow,
            "{w} columns: a third pane here would leave the list unreadable"
        );

        // The list already occupies every column the sidebar does not, so an
        // inspector shown at this width has nowhere to go but over it.
        let (sidebar, main) = narrow_parts(&l, w);
        assert_eq!(
            main.x,
            sidebar.right(),
            "{w} columns: the list abuts the sidebar"
        );
        assert_eq!(
            sidebar.width + main.width,
            w,
            "{w} columns: no column is held back for a third pane"
        );
    }
}

/// The sidebar is fixed; the list absorbs the width instead.
/// @spec TUI-RENDER-005
#[test]
fn the_sidebar_is_a_fixed_width_and_the_list_absorbs_the_rest() {
    let base = compute(area(FLOOR_W, FLOOR_H), None, false);
    let (base_sidebar, base_main) = narrow_parts(&base, FLOOR_W);

    for w in FLOOR_W + 1..PANE_BREAKPOINT {
        let l = compute(area(w, FLOOR_H), None, false);
        let (sidebar, main) = narrow_parts(&l, w);
        assert_eq!(
            sidebar.width, base_sidebar.width,
            "{w} columns: the sidebar is a fixed width"
        );
        assert_eq!(
            main.width,
            base_main.width + (w - FLOOR_W),
            "{w} columns: every extra column goes to the list"
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-RENDER-006 — 100 columns and wider
// ---------------------------------------------------------------------------

/// One hundred columns is where the inspector stops being a swap and becomes
/// a pane of its own.
/// @spec TUI-RENDER-006
#[test]
fn at_a_hundred_columns_the_inspector_becomes_a_pane_of_its_own() {
    let below = compute(area(PANE_BREAKPOINT - 1, FLOOR_H), None, false);
    assert_eq!(
        mode(&below),
        Mode::Narrow,
        "99 columns is still sidebar and list"
    );

    let at = compute(area(PANE_BREAKPOINT, FLOOR_H), None, false);
    let (sidebar, list, inspector) = wide_parts(&at, PANE_BREAKPOINT);
    assert!(
        sidebar.width > 0 && list.width > 0 && inspector.width > 0,
        "all three are drawn"
    );
    assert_eq!(
        sidebar.width + list.width + inspector.width,
        PANE_BREAKPOINT,
        "the three panes divide the whole body"
    );
}

/// Side by side, in order, with the inspector taking whatever the sidebar and
/// list leave.
/// @spec TUI-RENDER-006
#[test]
fn the_inspector_takes_the_width_the_sidebar_and_list_leave() {
    for w in PANE_BREAKPOINT..CAP_BREAKPOINT {
        let a = area(w, FLOOR_H);
        let l = compute(a, None, false);
        assert_frame_tiles(&l, a);

        let (sidebar, list, inspector) = wide_parts(&l, w);
        assert!(
            sidebar.x < list.x && list.x < inspector.x,
            "{w} columns: sidebar, list, inspector"
        );
        assert_eq!(
            inspector.width,
            w - sidebar.width - list.width,
            "{w} columns: the inspector is the remainder"
        );
        assert!(
            inspector.width > 0,
            "{w} columns: a pane with no width is not a pane"
        );
    }
}

/// The sidebar keeps its fixed width across the whole wide range, and neither
/// the list nor the inspector ever gives width back as the terminal grows.
/// @spec TUI-RENDER-006, TUI-RENDER-007
#[test]
fn growing_the_terminal_never_shrinks_a_wide_pane() {
    let mut prev: Option<(u16, Rect, Rect, Rect)> = None;
    let fixed_sidebar = wide_parts(
        &compute(area(PANE_BREAKPOINT, FLOOR_H), None, false),
        PANE_BREAKPOINT,
    )
    .0;

    for w in PANE_BREAKPOINT..=240 {
        let l = compute(area(w, FLOOR_H), None, false);
        let (sidebar, list, inspector) = wide_parts(&l, w);

        assert_eq!(
            sidebar.width, fixed_sidebar.width,
            "{w} columns: the sidebar is a fixed width"
        );

        if let Some((pw, _, plist, pinspector)) = prev {
            assert!(
                list.width >= plist.width,
                "{w} columns: the list narrowed as the terminal grew (was {} at {pw})",
                plist.width
            );
            assert!(
                inspector.width >= pinspector.width,
                "{w} columns: the inspector narrowed as the terminal grew (was {} at {pw})",
                pinspector.width
            );
        }
        prev = Some((w, sidebar, list, inspector));
    }
}

// ---------------------------------------------------------------------------
// TUI-RENDER-007 — the list's cap
// ---------------------------------------------------------------------------

/// At 120 and wider the list stops growing, so a row's name and its cost never
/// separate further than they can be read together.
/// @spec TUI-RENDER-007
#[test]
fn at_a_hundred_and_twenty_columns_and_wider_the_list_stops_growing() {
    let capped = wide_parts(
        &compute(area(CAP_BREAKPOINT, FLOOR_H), None, false),
        CAP_BREAKPOINT,
    )
    .1;

    for w in [CAP_BREAKPOINT + 1, 130, 150, 200, 300, 400] {
        let list = wide_parts(&compute(area(w, FLOOR_H), None, false), w).1;
        assert_eq!(
            list.width, capped.width,
            "{w} columns: the list is capped at its width at {CAP_BREAKPOINT}, not stretched"
        );
    }
}

/// Every surplus column goes to the inspector, which has genuinely more to say.
/// @spec TUI-RENDER-007
#[test]
fn every_column_past_the_cap_goes_to_the_inspector() {
    let (base_sidebar, _, base_inspector) = wide_parts(
        &compute(area(CAP_BREAKPOINT, FLOOR_H), None, false),
        CAP_BREAKPOINT,
    );

    for w in [CAP_BREAKPOINT + 1, 130, 150, 200, 300] {
        let (sidebar, _, inspector) = wide_parts(&compute(area(w, FLOOR_H), None, false), w);
        assert_eq!(
            sidebar.width, base_sidebar.width,
            "{w} columns: the sidebar stays fixed"
        );
        assert_eq!(
            inspector.width,
            base_inspector.width + (w - CAP_BREAKPOINT),
            "{w} columns: the surplus over {CAP_BREAKPOINT} belongs entirely to the inspector"
        );
    }
}

/// The cap's value is deferred to measurement, but its *purpose* is not: a list
/// wider than an entire floor-sized terminal is no longer scannable in one
/// glance, and one too narrow to hold a name beside a cost has overshot.
/// @spec TUI-RENDER-007
#[test]
fn the_capped_list_stays_readable_in_one_glance() {
    let list = wide_parts(&compute(area(300, FLOOR_H), None, false), 300).1;
    assert!(
        (LIST_CAP_MIN..=LIST_CAP_MAX).contains(&list.width),
        "a 300-column terminal gave the list {} columns; the cap exists so a row's name \
         and cost stay one saccade apart",
        list.width
    );

    let inspector = wide_parts(&compute(area(300, FLOOR_H), None, false), 300).2;
    assert!(
        inspector.width > list.width,
        "at 300 columns the surplus belongs to the inspector, so it must be the wider pane"
    );
}

// ---------------------------------------------------------------------------
// TUI-RENDER-008 — the asymmetric dead-band
// ---------------------------------------------------------------------------

/// A layout is entered *at* its boundary: growing from 99 to 100 gives the
/// three-pane layout immediately.
/// @spec TUI-RENDER-008
#[test]
fn a_layout_is_entered_at_its_boundary() {
    let steps = walk(&[95, 96, 97, 98, 99, 100], FLOOR_H);
    for (w, l) in &steps {
        let expected = if *w < PANE_BREAKPOINT {
            Mode::Narrow
        } else {
            Mode::Wide
        };
        assert_eq!(
            mode(l),
            expected,
            "growing to {w} columns: entry happens at the boundary"
        );
    }
}

/// ...and left some columns *below* it. The same width gives a different layout
/// depending on which side it was approached from — that asymmetry is the whole
/// mechanism, and a symmetric threshold would have none of it.
/// @spec TUI-RENDER-008
#[test]
fn a_layout_is_left_below_the_boundary_it_was_entered_at() {
    let entered = compute(area(PANE_BREAKPOINT, FLOOR_H), None, false);
    assert_eq!(
        mode(&entered),
        Mode::Wide,
        "fixture: entered the three-pane layout at 100"
    );

    let wobbled_down = compute(area(PANE_BREAKPOINT - 1, FLOOR_H), Some(&entered), false);
    assert_eq!(
        mode(&wobbled_down),
        Mode::Wide,
        "99 columns approached from above stays three-pane — one column of wobble may not flip it"
    );

    let fresh = compute(area(PANE_BREAKPOINT - 1, FLOOR_H), None, false);
    assert_eq!(
        mode(&fresh),
        Mode::Narrow,
        "99 columns approached fresh is sidebar and list"
    );
    assert_ne!(
        mode(&wobbled_down),
        mode(&fresh),
        "the dead-band must be asymmetric: the same width, two answers, decided by where it came from"
    );
}

/// The band is a few columns, not an open licence: TUI-RENDER-005 owns the
/// 80–99 range, so a shrinking terminal must be back to sidebar-and-list well
/// before the floor.
/// @spec TUI-RENDER-008
#[test]
fn the_dead_band_is_a_few_columns_wide_not_the_whole_range() {
    let widths: Vec<u16> = (85..=130).chain((85..130).rev()).collect();
    let steps = walk(&widths, FLOOR_H);

    let down = &steps[steps.len() - 45..];
    let left_at = down
        .iter()
        .find(|(_, l)| mode(l) == Mode::Narrow)
        .map(|(w, _)| *w)
        .expect("a shrinking terminal must eventually leave the three-pane layout");

    assert!(
        left_at < PANE_BREAKPOINT,
        "the layout must be left below the boundary it was entered at, not at it"
    );
    assert!(
        left_at >= PANE_BREAKPOINT - MAX_DEAD_BAND,
        "a dead-band reaching down to {left_at} would hold the three-pane layout across width \
         TUI-RENDER-005 governs"
    );
}

/// The behaviour the dead-band exists for: a drag-resize that overshoots the
/// boundary and settles on it must not flip the layout back and forth.
/// @spec TUI-RENDER-008
#[test]
fn a_resize_that_overshoots_and_settles_does_not_thrash() {
    let wobble = [96u16, 98, 99, 100, 101, 100, 99, 100, 99, 100, 101, 99, 100];
    let steps = walk(&wobble, FLOOR_H);

    let flips = steps
        .windows(2)
        .filter(|p| mode(&p[0].1) != mode(&p[1].1))
        .count();
    assert_eq!(
        flips,
        1,
        "a wobble across the boundary entered the three-pane layout once and should never have \
         left it; the layout changed {flips} times: {:?}",
        steps.iter().map(|(w, l)| (*w, mode(l))).collect::<Vec<_>>()
    );
    assert_eq!(
        mode(&steps.last().unwrap().1),
        Mode::Wide,
        "it settles three-pane"
    );
}

/// A full crossing in each direction changes the layout exactly once each way,
/// and the downward crossing happens at a strictly lower width than the upward
/// one.
/// @spec TUI-RENDER-008
#[test]
fn crossing_up_and_back_down_changes_the_layout_once_in_each_direction() {
    let up: Vec<u16> = (90..=130).collect();
    let down: Vec<u16> = (85..130).rev().collect();
    let widths: Vec<u16> = up.iter().chain(down.iter()).copied().collect();
    let steps = walk(&widths, FLOOR_H);

    let crossings: Vec<(u16, Mode)> = steps
        .windows(2)
        .filter(|p| mode(&p[0].1) != mode(&p[1].1))
        .map(|p| (p[1].0, mode(&p[1].1)))
        .collect();

    assert_eq!(
        crossings.len(),
        2,
        "one crossing up and one back down, no thrash in between: {crossings:?}"
    );
    assert_eq!(
        crossings[0],
        (PANE_BREAKPOINT, Mode::Wide),
        "entry is at the boundary"
    );
    assert_eq!(
        crossings[1].1,
        Mode::Narrow,
        "the second crossing is the way back"
    );
    assert!(
        crossings[1].0 < crossings[0].0,
        "the layout is left at a lower width ({}) than it was entered at ({})",
        crossings[1].0,
        crossings[0].0
    );
}

/// The dead-band applies at *each* breakpoint, not only the pane one: dropping
/// one column below 120 must not un-cap the list and re-flow both panes.
/// @spec TUI-RENDER-008, TUI-RENDER-007
#[test]
fn the_cap_breakpoint_carries_a_dead_band_too() {
    let at_cap = compute(area(CAP_BREAKPOINT, FLOOR_H), None, false);
    let capped_list = wide_parts(&at_cap, CAP_BREAKPOINT).1;

    let wobble = [
        CAP_BREAKPOINT - 1,
        CAP_BREAKPOINT,
        CAP_BREAKPOINT - 1,
        CAP_BREAKPOINT,
    ];
    let mut prev = at_cap;
    for w in wobble {
        let l = compute(area(w, FLOOR_H), Some(&prev), false);
        let (sidebar, list, inspector) = wide_parts(&l, w);
        assert_eq!(
            list.width, capped_list.width,
            "{w} columns: a one-column wobble at the cap breakpoint must not re-flow the list"
        );
        assert_eq!(
            sidebar.width + list.width + inspector.width,
            w,
            "{w} columns: the inspector absorbs the wobble"
        );
        prev = l;
    }
}

/// The previous layout is the only state the dead-band may carry: nothing is
/// remembered between calls, so a fresh call at a width always gives that
/// width's steady-state layout however wild the history before it.
/// @spec TUI-RENDER-008
#[test]
fn the_previous_layout_is_the_only_state_carried_between_calls() {
    let a = area(PANE_BREAKPOINT - 1, FLOOR_H);
    let first = compute(a, None, false);
    assert_eq!(first, compute(a, None, false), "same inputs, same layout");

    // Drag well past the boundary and back, then ask again from nothing.
    let _ = walk(&[99, 110, 130, 120, 99, 90], FLOOR_H);
    assert_eq!(
        compute(a, None, false),
        first,
        "a fresh call must not inherit a previous frame's side of the boundary"
    );
}
