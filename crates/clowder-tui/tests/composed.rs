//! The composition — `render::frame`, once every widget exists.
//!
//! The parts of a frame are drawn and tested on their own: the chrome, the
//! inspector, the transients. This file is about the one thing none of them
//! can be asked about alone — whether the frame *routes* each region to the
//! widget that owns it. `render/widgets/mod.rs` states the division outright:
//! "one module per region so each can be drawn, and tested, on its own —
//! `mod.rs` composes them and owns nothing but the composition." A header the
//! header widget draws beautifully and the frame never calls is a header the
//! player never sees, and every chrome spec (TUI-NAV-019) is unmet no matter
//! how good the widget is.
//!
//! **How routing is asserted.** A region is compared against the *same widget
//! drawn alone into the same rectangle of a terminal of the same size*. That
//! is the sharpest form the claim has: not "something was drawn here" and not
//! "this text appears somewhere", but "these cells are that widget's". It
//! cannot be satisfied by a frame that re-implements a region inline, and it
//! needs no lexical guess about a widget's wording — the widget is the oracle.
//! Every such assertion is guarded against being vacuous: a widget that reads
//! the view state is first required to draw *differently* for a different
//! view, so "the region equals the widget" cannot pass by both sides ignoring
//! their inputs.
//!
//! **Symbols, not styles.** Comparisons read the buffer's symbols and ignore
//! its styles, as the sibling files do, because colour is the second channel
//! everywhere in this design (TUI-NAV-022) and the theme leaf owns it. A
//! composition that routed correctly but tinted differently is a theme
//! question, not a routing one.
//!
//! **Which rectangles are pinned, and which are not.** Every region but one
//! comes from `render::layout`, which owns the algebra and is tested on its
//! own: header, tab bar, banner row, sidebar, list, inspector pane, ticker,
//! hint bar. The exception is a full-screen overlay: no spec says whether the
//! Journal, help or palette takes the whole body (the sidebar included) or
//! only the list's space. The design pins only that it does not block — "the
//! sim keeps ticking and the chrome keeps rendering behind" — so the
//! assertions here require that *some* plausible region of the body is
//! exactly the widget's drawing, and separately that the chrome around it is
//! untouched. The choice between those regions is reported as an open
//! question rather than invented here.
//!
//! **Why the floor matters.** Every claim is made at the 80×24 floor as well
//! as above both breakpoints. The floor is where the regions are tightest and
//! where an off-by-one collision shows up as a neighbour's first or last line
//! being overwritten — which a per-region comparison against the widget's own
//! drawing catches exactly.
//!
//! **What is not tested here.** TUI-NAV-035 has an input half — that ↑/↓ and
//! j/k still reach the underlying cursor while the inspector is swapped in,
//! and that Space or Esc puts the list back. Those are key-to-view-state
//! moves, and they live in `nav::update`, which this chunk does not own and
//! which does not yet handle either action. What composition owes the spec is
//! the drawing half, and that is what is asserted below: the swapped
//! inspector describes whatever row the cursor is on, and the moment the
//! player is no longer asking for it the list is back — at the row the cursor
//! moved to, not the row it started on.

use std::collections::BTreeSet;

use clowder_core::content::structure_def;
use clowder_core::{
    content, ContentId, Event, GameState, JournalItem, Kitten, KittenTrait, Location,
    OfflineReport, ResourceId, StructureId,
};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::{Frame, Terminal};

use clowder_tui::nav::rows::rows;
use clowder_tui::nav::{update, Action, Overlay, PendingReport, RowKey, Section, SeenMark, ViewState};
use clowder_tui::render;
use clowder_tui::render::layout::{Body, Layout};
use clowder_tui::render::widgets::inspector::Placement;
use clowder_tui::render::widgets::{chrome, inspector, overlays};

// ---------------------------------------------------------------------------
// Sizes
//
// The floor and the two breakpoints come from the specs (TUI-RENDER-004..007).
// Every size below is reached cold, so the dead-band never enters into it: a
// frame drawn with no previous layout lays out from its width alone.
// ---------------------------------------------------------------------------

const FLOOR_W: u16 = 80;
const FLOOR_H: u16 = 24;

/// The top of the swap range: still one column short of the pane breakpoint.
const NARROW_W: u16 = 99;

/// Sizes every claim in this file is made at: the floor, the top of the narrow
/// range, the first width with a pane, the width where the list caps, and one
/// comfortably past both breakpoints.
const SIZES: [(u16, u16); 5] =
    [(FLOOR_W, FLOOR_H), (NARROW_W, FLOOR_H), (100, 30), (120, 40), (150, 50)];

/// The sizes at or above the first breakpoint — where the inspector is a pane
/// of its own rather than a swap.
const WIDE_SIZES: [(u16, u16); 3] = [(100, 30), (120, 40), (150, 50)];

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn kitten(job: Option<clowder_core::JobId>, location: Location) -> Kitten {
    Kitten {
        name: String::new(),
        job,
        location,
        arrived_day: 0,
        job_seq: None,
        away_seq: None,
        skills: [0.0; 8],
        exp: 0.0,
        rank: 0,
        trait_id: KittenTrait::None,
        tinkering: None,
    }
}

/// An absence long enough for the core to pin it, with something that grew and
/// someone who arrived — so the banner and the Journal both have something to
/// say and neither draws blank.
fn away_report() -> OfflineReport {
    let mut report =
        OfflineReport { elapsed_seconds: 2 * 86_400 + 4 * 3_600, ..OfflineReport::default() };
    report.resource_deltas[ResourceId::Catnip.ix()] = 12_400.0;
    report.arrived = vec![("Pebble".to_string(), 1)];
    report.wandered = vec![("Moss".to_string(), 1)];
    report
}

/// A meadow far enough along that every widget has something of its own to
/// say: a list far longer than any viewport, stores for the sidebar, kittens
/// the idle chip counts, an open sky window, a journal with occurrences in it,
/// and one pinned away-report.
///
/// The fixture checks each of those, because a routing assertion against a
/// widget that had nothing to draw would pass for the wrong reason.
fn settled() -> GameState {
    let mut s = GameState::new(11);
    s.ticks = 362 * content::TICKS_PER_DAY;
    s.speed = 2.0;
    s.revealed = StructureId::ALL.iter().map(|b| ContentId::Structure(*b)).collect::<BTreeSet<_>>();
    for r in ResourceId::ALL {
        s.resources[r.ix()] = 1_234.0;
    }
    for b in StructureId::ALL.iter().take(6) {
        s.structures[b.ix()] = 2;
    }
    s.kittens = (0..3)
        .map(|_| kitten(None, Location::Home))
        .chain((0..2).map(|_| kitten(Some(clowder_core::JobId::Gardener), Location::Home)))
        .collect();
    s.pending_sky_event = Some(s.ticks + 1_000);

    for (i, b) in StructureId::ALL.iter().take(4).enumerate() {
        s.journal.record(100 + i as u64, &Event::StructureBuilt { id: *b, count: 2 });
    }
    s.journal.record_away(200, &away_report());

    assert!(
        rows(&s, Section::Meadow).len() > FLOOR_H as usize * 2,
        "fixture: the Meadow's list must be far longer than any viewport here"
    );
    assert!(
        s.journal.iter_chronological().len() > 4,
        "fixture: the journal must hold enough to scroll"
    );
    assert!(
        s.journal.kept.iter().any(|e| matches!(e.item, JournalItem::Away(_))),
        "fixture: the away-report must be long enough an absence for the core to pin it"
    );
    s
}

/// Where the pinned away-report sits in `state.journal.kept` — what a
/// `PendingReport` indexes: a position in the permanent, append-only list,
/// never in the merged-with-`recent` view, whose order an ordinary event
/// stamped earlier can still shift. @spec TUI-NAV-027
fn away_entry(state: &GameState) -> usize {
    state
        .journal
        .kept
        .iter()
        .position(|e| matches!(e.item, JournalItem::Away(_)))
        .expect("fixture: the journal pins an away-report")
}

fn structure_row(b: StructureId) -> RowKey {
    RowKey::Content(ContentId::Structure(b), "")
}

/// Two rows far apart in the Meadow's list: far enough that the viewport that
/// shows one cannot also show the other, so "the cursor moved" is visible in
/// the drawing rather than inferred.
fn near_row() -> RowKey {
    structure_row(StructureId::ALL[4])
}

fn far_row() -> RowKey {
    structure_row(StructureId::ALL[44])
}

/// The view a settled player is in: focused on the Meadow with the cursor on a
/// row, the Meadow visited and everything else not — so the tab bar carries
/// badges and the header a `★N new` chip, and both are things only the *view*
/// can tell a widget.
///
/// No report is pending here: an unread one takes a row from the body
/// (TUI-RENDER-003), and the banner has its own tests below.
fn looking_at(state: &GameState, key: RowKey) -> ViewState {
    let mut v = ViewState { section: Some(Section::Meadow), ..Default::default() };
    v.focus[Section::Meadow.index()] = Some(key);
    v.seen[Section::Meadow.index()] =
        SeenMark { visited: true, revealed_at_last_visit: rows(state, Section::Meadow).len() };
    v
}

/// The same view, with the inspector asked for.
fn inspecting(state: &GameState, key: RowKey) -> ViewState {
    ViewState { inspector_wanted: true, ..looking_at(state, key) }
}

/// The view a returning player lands in: one unread report, so the layout
/// gives the frame a banner row.
fn returning(state: &GameState) -> ViewState {
    ViewState {
        reports: vec![PendingReport { entry: away_entry(state), dismissed: false }],
        ..looking_at(state, near_row())
    }
}

/// A prefix of a structure's name, long enough to identify it and short enough
/// to survive TUI-RENDER-014's ellipsis where a column is tight.
fn probe(b: StructureId) -> String {
    let name: String = structure_def(b).name.chars().take(10).collect();
    let clashes = StructureId::ALL
        .into_iter()
        .filter(|o| *o != b && structure_def(*o).name.contains(&name))
        .count();
    assert_eq!(clashes, 0, "fixture: {name:?} does not identify one structure");
    name
}

// ---------------------------------------------------------------------------
// Drawing
// ---------------------------------------------------------------------------

/// One drawn frame: the cells, and the layout the frame reported using.
struct Drawn {
    buffer: Buffer,
    layout: Layout,
}

fn terminal(w: u16, h: u16) -> Terminal<TestBackend> {
    Terminal::new(TestBackend::new(w, h)).expect("TestBackend is infallible")
}

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

/// Draw one frame into a terminal that has never drawn anything — so the
/// layout comes from the width alone and no dead-band is in play.
fn cold(state: &GameState, view: &ViewState, w: u16, h: u16) -> Drawn {
    let mut term = terminal(w, h);
    draw(&mut term, state, view, None)
}

/// Draw one widget, alone, into a terminal of the same size — the oracle every
/// routing assertion compares a region against.
fn widget_alone(w: u16, h: u16, render: impl FnOnce(&mut Frame)) -> Buffer {
    let mut term = terminal(w, h);
    term.draw(|f| render(f)).expect("TestBackend is infallible");
    term.backend().buffer().clone()
}

// ---------------------------------------------------------------------------
// Reading a buffer
// ---------------------------------------------------------------------------

fn region_lines(buf: &Buffer, rect: Rect) -> Vec<String> {
    (rect.y..rect.bottom())
        .map(|y| {
            (rect.x..rect.right())
                .map(|x| buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "))
                .collect::<String>()
        })
        .collect()
}

fn region_text(buf: &Buffer, rect: Rect) -> String {
    region_lines(buf, rect).join("\n")
}

fn has_content(buf: &Buffer, rect: Rect) -> bool {
    region_lines(buf, rect).iter().any(|l| !l.trim().is_empty())
}

fn drawn_anywhere(d: &Drawn, text: &str) -> bool {
    region_lines(&d.buffer, d.buffer.area).iter().any(|l| l.contains(text))
}

// ---------------------------------------------------------------------------
// Regions
// ---------------------------------------------------------------------------

fn sidebar_rect(l: &Layout) -> Rect {
    match l.body {
        Body::Narrow { sidebar, .. } | Body::Wide { sidebar, .. } => sidebar,
        Body::TooSmall(r) => panic!("this claim needs a supported size, not {r:?}"),
    }
}

/// The pane the list occupies — and the pane the inspector swaps into where
/// the terminal is narrow.
fn main_rect(l: &Layout) -> Rect {
    match l.body {
        Body::Narrow { main, .. } => main,
        Body::Wide { list, .. } => list,
        Body::TooSmall(r) => panic!("this claim needs a supported size, not {r:?}"),
    }
}

/// The inspector's own pane, where the width affords one.
fn pane_rect(l: &Layout) -> Option<Rect> {
    match l.body {
        Body::Wide { inspector, .. } => Some(inspector),
        _ => None,
    }
}

/// The whole body, full width: the largest region an overlay could be given.
fn body_rect(l: &Layout, w: u16) -> Rect {
    let any = sidebar_rect(l);
    Rect::new(0, any.y, w, any.height)
}

/// Every region a full-screen overlay could plausibly have been handed. No
/// spec chooses between them — see this file's header.
fn overlay_candidates(l: &Layout, w: u16) -> Vec<(&'static str, Rect)> {
    let mut out = vec![("the whole body", body_rect(l, w)), ("the list's space", main_rect(l))];
    if let Body::Wide { list, inspector, .. } = l.body {
        out.push((
            "the list and the inspector together",
            Rect::new(list.x, list.y, list.width + inspector.width, list.height),
        ));
    }
    out
}

// ---------------------------------------------------------------------------
// Routing assertions
// ---------------------------------------------------------------------------

/// A region of the composed frame is exactly what a widget draws into the same
/// rectangle.
#[track_caller]
fn assert_region_is(
    d: &Drawn,
    rect: Rect,
    size: (u16, u16),
    what: &str,
    widget: &str,
    render: impl FnOnce(&mut Frame),
) {
    let (w, h) = size;
    let alone = widget_alone(w, h, render);
    assert_eq!(
        region_lines(&d.buffer, rect),
        region_lines(&alone, rect),
        "at {w}x{h}, the {what} is not what `{widget}` draws into the same rectangle \
         ({rect:?}). The frame composes the widgets and owns nothing but the composition — \
         every region belongs to the widget that owns it.\n composed:\n{}\n {widget} alone:\n{}",
        region_text(&d.buffer, rect),
        region_text(&alone, rect),
    );
}

/// A guard against a vacuous comparison: a widget that reads the view state
/// must draw differently for a different view, or "the region equals the
/// widget" would hold of an implementation in which both ignored their inputs.
#[track_caller]
fn assert_view_sensitive(
    size: (u16, u16),
    rect: Rect,
    widget: &str,
    a: impl FnOnce(&mut Frame),
    b: impl FnOnce(&mut Frame),
) {
    let (w, h) = size;
    assert_ne!(
        region_text(&widget_alone(w, h, a), rect),
        region_text(&widget_alone(w, h, b), rect),
        "fixture: at {w}x{h}, `{widget}` draws the same thing for two different view states, \
         so comparing a region against it would prove nothing about what the frame handed it"
    );
}

/// Some plausible region of the body is exactly the widget's drawing.
#[track_caller]
fn assert_some_region_is(
    d: &Drawn,
    size: (u16, u16),
    what: &str,
    widget: &str,
    render: impl Fn(&mut Frame, Rect),
) {
    let (w, h) = size;
    let candidates = overlay_candidates(&d.layout, w);
    let matched = candidates.iter().find(|(_, rect)| {
        let rect = *rect;
        let alone = widget_alone(w, h, |f| render(f, rect));
        // A blank region matching a blank widget would prove nothing, so the
        // widget is required to have had something to say.
        has_content(&alone, rect) && region_lines(&d.buffer, rect) == region_lines(&alone, rect)
    });
    if matched.is_some() {
        return;
    }

    let (name, rect) = candidates[0];
    let alone = widget_alone(w, h, |f| render(f, rect));
    panic!(
        "at {w}x{h}, no region of the body is what `{widget}` draws: the {what} was not routed \
         to its widget. Tried {:?}.\n composed, over {name} ({rect:?}):\n{}\n {widget} alone \
         there:\n{}",
        candidates.iter().map(|(n, r)| (*n, *r)).collect::<Vec<_>>(),
        region_text(&d.buffer, rect),
        region_text(&alone, rect),
    );
}

/// The chrome around the body is untouched between two frames — what "nothing
/// here blocks; the chrome keeps rendering behind every one of them" means
/// cell by cell.
///
/// The hint bar is asked only to still be there. It is the one piece of chrome
/// the design says *adapts* to context (TUI-NAV-023), so a transient changing
/// it is a hint-bar decision rather than a broken composition; a transient
/// blanking it would not be.
#[track_caller]
fn assert_chrome_unchanged(a: &Drawn, b: &Drawn, what: &str) {
    for (name, rect) in
        [("header", a.layout.header), ("tab bar", a.layout.tabs), ("ticker", a.layout.ticker)]
    {
        assert_eq!(
            region_lines(&a.buffer, rect),
            region_lines(&b.buffer, rect),
            "the {name} changed when {what}. The chrome keeps rendering behind a transient: \
             nothing in this game stops the world.\n before:\n{}\n after:\n{}",
            region_text(&a.buffer, rect),
            region_text(&b.buffer, rect),
        );
    }
    assert!(
        has_content(&b.buffer, b.layout.hints),
        "the hint bar went blank when {what}; it adapts to context but it is never absent"
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-019 — the chrome is on screen, and it is the chrome widget's
// ---------------------------------------------------------------------------

/// Header, tab bar, sidebar, ticker and hint bar are all on screen at every
/// supported size, and each is exactly what its widget draws into its own
/// rectangle. The frame is where those five become simultaneous — the glance
/// contract is a property of the composition, not of any one part.
/// @spec TUI-NAV-019
#[test]
fn every_piece_of_the_chrome_is_the_widget_that_owns_it() {
    let state = settled();
    let view = looking_at(&state, near_row());
    let other = ViewState::default();

    for size in SIZES {
        let (w, h) = size;
        let d = cold(&state, &view, w, h);
        let l = d.layout;

        // Each of the three view-reading pieces really does read it, so the
        // comparisons below cannot pass by both sides ignoring the view.
        assert_view_sensitive(
            size,
            l.header,
            "chrome::header",
            |f| chrome::header(f, l.header, &state, &view),
            |f| chrome::header(f, l.header, &state, &other),
        );
        assert_view_sensitive(
            size,
            l.tabs,
            "chrome::tabs",
            |f| chrome::tabs(f, l.tabs, &state, &view),
            |f| chrome::tabs(f, l.tabs, &state, &other),
        );
        assert_view_sensitive(
            size,
            l.hints,
            "chrome::hints",
            |f| chrome::hints(f, l.hints, &state, &view),
            |f| chrome::hints(f, l.hints, &state, &other),
        );

        assert_region_is(&d, l.header, size, "header row", "chrome::header", |f| {
            chrome::header(f, l.header, &state, &view)
        });
        assert_region_is(&d, l.tabs, size, "tab bar", "chrome::tabs", |f| {
            chrome::tabs(f, l.tabs, &state, &view)
        });
        assert_region_is(&d, sidebar_rect(&l), size, "resource sidebar", "chrome::sidebar", |f| {
            chrome::sidebar(f, sidebar_rect(&l), &state)
        });
        assert_region_is(&d, l.ticker, size, "event ticker", "overlays::ticker", |f| {
            overlays::ticker(f, l.ticker, &state)
        });
        assert_region_is(&d, l.hints, size, "hint bar", "chrome::hints", |f| {
            chrome::hints(f, l.hints, &state, &view)
        });
    }
}

/// None of the five is blank. A region routed to a widget that had nothing to
/// draw would satisfy every comparison above and leave the player with an
/// empty line where the design promises a fact.
/// @spec TUI-NAV-019
#[test]
fn no_piece_of_the_chrome_is_left_blank_at_any_supported_size() {
    let state = settled();
    let view = looking_at(&state, near_row());

    for (w, h) in SIZES {
        let d = cold(&state, &view, w, h);
        for (name, rect) in [
            ("header", d.layout.header),
            ("tab bar", d.layout.tabs),
            ("resource sidebar", sidebar_rect(&d.layout)),
            ("event ticker", d.layout.ticker),
            ("hint bar", d.layout.hints),
        ] {
            assert!(
                has_content(&d.buffer, rect),
                "at {w}x{h} the {name} drew nothing at all. The chrome is what the two-second \
                 glance is made of, and it is on screen at every supported size"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-017 — the inspector's pane, and the frame agreeing with its widget
// ---------------------------------------------------------------------------

/// Where the width affords a pane, the pane is the inspector widget's drawing
/// — and it tracks the focused row without being asked, which is the whole
/// point of a persistent pane: truth at zero keypresses.
/// @spec TUI-NAV-017
#[test]
fn the_inspector_pane_is_the_inspector_widget_following_the_cursor() {
    let state = settled();

    for size in WIDE_SIZES {
        let (w, h) = size;
        for key in [near_row(), far_row()] {
            let view = looking_at(&state, key.clone());
            let d = cold(&state, &view, w, h);
            let pane = pane_rect(&d.layout)
                .unwrap_or_else(|| panic!("at {w}x{h} the layout must give the inspector a pane"));

            assert_region_is(&d, pane, size, "inspector pane", "inspector::draw", |f| {
                inspector::draw(f, pane, &state, &view)
            });
        }

        // Two cursors, one width: the pane must say different things, or it is
        // not following the cursor at all.
        let near = cold(&state, &looking_at(&state, near_row()), w, h);
        let far = cold(&state, &looking_at(&state, far_row()), w, h);
        let pane = pane_rect(&near.layout).expect("a pane at this width");
        assert_ne!(
            region_text(&near.buffer, pane),
            region_text(&far.buffer, pane),
            "at {w}x{h} the inspector pane says the same thing about two different rows; \
             a persistent pane live-updates for the focused row"
        );
    }
}

/// Above the breakpoint the inspector is already there, so asking for it
/// changes nothing — the swap is a narrow terminal's answer to the same need,
/// not a second mode a wide one can enter.
/// @spec TUI-NAV-017, TUI-NAV-035
#[test]
fn asking_for_the_inspector_where_a_pane_exists_changes_nothing() {
    let state = settled();

    for (w, h) in WIDE_SIZES {
        let unasked = cold(&state, &looking_at(&state, near_row()), w, h);
        let asked = cold(&state, &inspecting(&state, near_row()), w, h);
        assert_eq!(
            region_lines(&asked.buffer, asked.buffer.area),
            region_lines(&unasked.buffer, unasked.buffer.area),
            "at {w}x{h} asking for the inspector redrew the frame, but the inspector is already \
             a pane of its own here; the list must not be swapped away at a width that has room \
             for both"
        );
    }
}

/// The frame and the inspector's own `placement` are the same decision, taken
/// once. Whatever `placement` says at a width — a pane, a swap over the list,
/// or nothing at all — is what the frame draws.
///
/// Only cold frames are asked, because `placement` takes a width and nothing
/// else while `layout::compute` carries a dead-band across the same boundary
/// (TUI-RENDER-008, TUI-NAV-047); inside the band the two cannot both be
/// right, and that disagreement is reported rather than settled here.
/// @spec TUI-NAV-017, TUI-NAV-035
#[test]
fn the_frame_places_the_inspector_where_its_widget_says_it_goes() {
    let state = settled();

    for size in SIZES {
        let (w, h) = size;
        for wanted in [false, true] {
            let view = if wanted {
                inspecting(&state, near_row())
            } else {
                looking_at(&state, near_row())
            };
            let d = cold(&state, &view, w, h);
            let main = main_rect(&d.layout);

            match inspector::placement(w, wanted) {
                Placement::Pane => {
                    let pane = pane_rect(&d.layout).unwrap_or_else(|| {
                        panic!("at {w}x{h} placement says Pane, so the body must have one")
                    });
                    assert_region_is(&d, pane, size, "inspector pane", "inspector::draw", |f| {
                        inspector::draw(f, pane, &state, &view)
                    });
                }
                Placement::Swapped => {
                    // The swap draws into the space the pane divider
                    // (TUI-RENDER-016) leaves, not the full list rect.
                    let space = left_by_dividers(&d.layout);
                    assert_region_is(
                        &d,
                        space,
                        size,
                        "inspector swapped in over the list",
                        "inspector::draw",
                        |f| inspector::draw(f, space, &state, &view),
                    );
                }
                Placement::Absent => {
                    let alone = widget_alone(w, h, |f| inspector::draw(f, main, &state, &view));
                    assert_ne!(
                        region_text(&d.buffer, main),
                        region_text(&alone, main),
                        "at {w}x{h} nobody asked for the inspector and the width has no room for \
                         a pane, yet the list's space is showing the inspector"
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-035 — the narrow-mode swap
// ---------------------------------------------------------------------------

/// At the floor, Space puts the inspector *in the list's space* — and nowhere
/// else. Not a popup: the sidebar, the header, the tab bar, the ticker and the
/// hint bar all keep drawing exactly what they drew, because the chrome and
/// the sim stay visibly alive behind it.
/// @spec TUI-NAV-035, TUI-NAV-017
#[test]
fn the_swapped_inspector_takes_the_lists_space_and_nothing_else() {
    let state = settled();
    let listing = cold(&state, &looking_at(&state, near_row()), FLOOR_W, FLOOR_H);
    let swapped = cold(&state, &inspecting(&state, near_row()), FLOOR_W, FLOOR_H);
    let size = (FLOOR_W, FLOOR_H);
    let main = main_rect(&swapped.layout);
    let view = inspecting(&state, near_row());

    // The swap draws into the space the pane divider (TUI-RENDER-016) leaves.
    let space = left_by_dividers(&swapped.layout);
    assert_region_is(&swapped, space, size, "swapped inspector", "inspector::draw", |f| {
        inspector::draw(f, space, &state, &view)
    });

    assert_ne!(
        region_text(&swapped.buffer, main),
        region_text(&listing.buffer, main),
        "the inspector swapped in over the list and drew what the list was already drawing"
    );

    assert_chrome_unchanged(&listing, &swapped, "the inspector swapped in over the list");
    assert_eq!(
        region_lines(&swapped.buffer, sidebar_rect(&swapped.layout)),
        region_lines(&listing.buffer, sidebar_rect(&listing.layout)),
        "the resource sidebar changed when the inspector swapped in. The swap takes the list's \
         space; a popup that covered the chrome would read as something that stopped the world"
    );

    // A row that the list was showing, and that the inspector has no reason to
    // mention, is gone from the frame entirely — the list really was replaced.
    let neighbour = probe(StructureId::ALL[5]);
    assert!(
        drawn_anywhere(&listing, &neighbour),
        "fixture: the list should be showing the row next to the cursor"
    );
    assert!(
        !drawn_anywhere(&swapped, &neighbour),
        "the row beside the cursor is still drawn while the inspector occupies the list's \
         space; the inspector swaps in place of the list, it does not share it"
    );
}

/// While the inspector holds the list's space, the cursor underneath is still
/// live: move it and the inspector follows. That is what makes the swap a way
/// to *browse* inspected items rather than a dead end.
/// @spec TUI-NAV-035
#[test]
fn the_swapped_inspector_follows_the_underlying_cursor() {
    let state = settled();
    let size = (FLOOR_W, FLOOR_H);
    let near_view = inspecting(&state, near_row());
    let far_view = inspecting(&state, far_row());

    let near = cold(&state, &near_view, FLOOR_W, FLOOR_H);
    let far = cold(&state, &far_view, FLOOR_W, FLOOR_H);
    let main = main_rect(&near.layout);
    // The swap draws into the space the pane divider (TUI-RENDER-016) leaves.
    let space = left_by_dividers(&near.layout);

    assert_region_is(&near, space, size, "swapped inspector", "inspector::draw", |f| {
        inspector::draw(f, space, &state, &near_view)
    });
    assert_region_is(&far, space, size, "swapped inspector", "inspector::draw", |f| {
        inspector::draw(f, space, &state, &far_view)
    });
    assert_ne!(
        region_text(&near.buffer, main),
        region_text(&far.buffer, main),
        "the cursor moved under the swapped inspector and the inspector said the same thing; \
         it follows the cursor, so the player can browse inspected items"
    );
}

/// A palette jump moves the cursor, not the layout. Taken from inside the
/// narrow-mode swap, it leaves the swap in place — and the swapped inspector,
/// following the cursor as it follows every other move, shows the row the jump
/// landed on. The player chose the swap; a jump that quietly returned them to
/// the list would be undoing a choice they did not ask to undo.
///
/// @spec TUI-NAV-065
#[test]
fn a_palette_jump_inside_the_narrow_swap_keeps_the_swap_on_the_landed_row() {
    let state = settled();
    let target = StructureId::ALL[44];

    let mut view = inspecting(&state, near_row());
    update(Action::OpenPalette, &mut view, &state);
    for c in structure_def(target).name.chars() {
        update(Action::Type(c), &mut view, &state);
    }
    update(Action::Activate, &mut view, &state);

    assert!(view.overlay.is_none(), "the palette's Enter did not jump (TUI-NAV-038)");
    assert_eq!(
        view.focus[Section::Meadow.index()],
        Some(far_row()),
        "fixture: the structure's own name aims the palette at its row"
    );
    assert!(
        view.inspector_wanted,
        "the palette's jump dropped the inspector the player had swapped in; it moves the \
         cursor, not the layout"
    );

    let jumped = cold(&state, &view, FLOOR_W, FLOOR_H);
    let main = main_rect(&jumped.layout);
    let expected = cold(&state, &inspecting(&state, far_row()), FLOOR_W, FLOOR_H);
    let listing = cold(&state, &looking_at(&state, far_row()), FLOOR_W, FLOOR_H);
    assert_eq!(
        region_text(&jumped.buffer, main),
        region_text(&expected.buffer, main_rect(&expected.layout)),
        "after a palette jump inside the swap, the list's space does not show the swapped \
         inspector on the landed row"
    );
    assert_ne!(
        region_text(&jumped.buffer, main),
        region_text(&listing.buffer, main_rect(&listing.layout)),
        "after a palette jump inside the swap, the list came back; the swap stays"
    );
}

/// Space or Esc returns to the list — and to the row the cursor moved to while
/// the inspector was up, not the row it started on. The cursor the inspector
/// was following is the list's own cursor; there is only one.
///
/// The keys themselves are `nav::update`'s (see this file's header); what is
/// asserted here is the drawing either side of them.
/// @spec TUI-NAV-035
#[test]
fn returning_from_the_swap_shows_the_list_where_the_cursor_moved_to() {
    let state = settled();
    let started = probe(StructureId::ALL[4]);
    let arrived = probe(StructureId::ALL[44]);

    let mut term = terminal(FLOOR_W, FLOOR_H);
    // Listing, cursor on the near row.
    let opened = draw(&mut term, &state, &looking_at(&state, near_row()), None);
    assert!(
        drawn_anywhere(&opened, &started),
        "fixture: the list should open on the focused row"
    );

    // Space: the inspector takes the list's space.
    let inspecting_near =
        draw(&mut term, &state, &inspecting(&state, near_row()), Some(&opened.layout));
    // The cursor moves while the inspector is up.
    let inspecting_far =
        draw(&mut term, &state, &inspecting(&state, far_row()), Some(&inspecting_near.layout));
    assert_ne!(
        region_text(&inspecting_far.buffer, main_rect(&inspecting_far.layout)),
        region_text(&inspecting_near.buffer, main_rect(&inspecting_near.layout)),
        "the inspector must follow the cursor while it holds the list's space"
    );

    // Space (or Esc) again: the list is back.
    let returned =
        draw(&mut term, &state, &looking_at(&state, far_row()), Some(&inspecting_far.layout));

    assert!(
        drawn_anywhere(&returned, &arrived),
        "returning to the list must show the row the cursor moved to while the inspector was up"
    );
    assert!(
        !drawn_anywhere(&returned, &started),
        "the list came back at the row the cursor started on; the cursor keys moved the \
         underlying cursor, so the viewport that returns is the one that cursor derives"
    );
    let never_swapped = cold(&state, &looking_at(&state, far_row()), FLOOR_W, FLOOR_H);
    assert_eq!(
        region_lines(&returned.buffer, returned.buffer.area),
        region_lines(&never_swapped.buffer, never_swapped.buffer.area),
        "the frame after a swap and back is not the frame that cursor always draws; nothing may \
         survive the swap but the view state itself"
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-026, TUI-NAV-025, TUI-NAV-038 — the overlays
// ---------------------------------------------------------------------------

/// An open Journal is the Journal widget's drawing, over the body, with the
/// chrome still rendering around it. Nothing blocks: `e` opens scrollback, it
/// does not stop the world.
/// @spec TUI-NAV-026
#[test]
fn an_open_journal_is_the_journal_widgets_drawing() {
    let state = settled();

    for size in SIZES {
        let (w, h) = size;
        let bare = cold(&state, &looking_at(&state, near_row()), w, h);
        let view =
            ViewState { overlay: Some(Overlay::Journal { anchor: 0 }), ..looking_at(&state, near_row()) };
        let d = cold(&state, &view, w, h);

        assert_ne!(
            region_lines(&d.buffer, d.buffer.area),
            region_lines(&bare.buffer, bare.buffer.area),
            "at {w}x{h} opening the Journal changed nothing on screen"
        );
        assert_some_region_is(&d, size, "Journal overlay", "overlays::journal", |f, rect| {
            overlays::journal(f, rect, &state, &view)
        });
        assert_chrome_unchanged(&bare, &d, "the Journal opened");
    }
}

/// The Journal's own scroll anchor reaches it. Only the widget reads that
/// field, so a frame drawing a Journal that ignores it is a frame that built
/// its own instead of handing over the view.
/// @spec TUI-NAV-026
#[test]
fn the_journals_scroll_anchor_reaches_the_widget() {
    let state = settled();
    let at = |anchor: usize| ViewState {
        overlay: Some(Overlay::Journal { anchor }),
        ..looking_at(&state, near_row())
    };

    let top = cold(&state, &at(0), FLOOR_W, FLOOR_H);
    let scrolled = cold(&state, &at(3), FLOOR_W, FLOOR_H);
    let body = body_rect(&top.layout, FLOOR_W);

    assert_ne!(
        region_text(&top.buffer, body),
        region_text(&scrolled.buffer, body),
        "two scroll positions of the Journal drew the same body; the overlay's own anchor is \
         part of the view state the frame hands it"
    );
}

/// The help overlay is the help widget's drawing. There is no manual: this and
/// the hint bar are the manual, and the frame is what puts it on screen.
/// @spec TUI-NAV-025
#[test]
fn an_open_help_overlay_is_the_help_widgets_drawing() {
    let state = settled();

    for size in SIZES {
        let (w, h) = size;
        let bare = cold(&state, &looking_at(&state, near_row()), w, h);
        let view = ViewState { overlay: Some(Overlay::Help), ..looking_at(&state, near_row()) };
        let d = cold(&state, &view, w, h);

        assert_ne!(
            region_lines(&d.buffer, d.buffer.area),
            region_lines(&bare.buffer, bare.buffer.area),
            "at {w}x{h} pressing ? changed nothing on screen"
        );
        assert_some_region_is(&d, size, "help overlay", "overlays::help", |f, rect| {
            overlays::help(f, rect)
        });
        assert_chrome_unchanged(&bare, &d, "the help overlay opened");
    }
}

/// At every supported size — the floor with a banner row included, where the
/// overlay has the fewest rows — the help overlay shows every entry of its three
/// lists, in their order, each screen's keys on a line of their own.
/// @spec TUI-NAV-124, TUI-NAV-122
#[test]
fn the_help_overlay_at_every_size_cuts_no_entry() {
    let state = settled();
    let plain = ViewState { overlay: Some(Overlay::Help), ..looking_at(&state, near_row()) };
    let banner = ViewState { overlay: Some(Overlay::Help), ..returning(&state) };
    let cases = SIZES
        .iter()
        .map(|&(w, h)| (w, h, &plain, ""))
        .chain([(FLOOR_W, FLOOR_H, &banner, " with a banner"), (NARROW_W, FLOOR_H, &banner, " with a banner")]);
    for (w, h, view, how) in cases {
        let d = cold(&state, view, w, h);
        let lines = region_lines(&d.buffer, d.buffer.area);
        let text = lines.join("\n");
        for needle in HELP_ENTRIES {
            assert!(text.contains(needle), "the {w}x{h}{how} help overlay does not show {needle:?}:\n{text}");
        }
        assert!(lines.iter().any(|l| shows_cursor_entry(l)), "the {w}x{h}{how} help overlay does not show the cursor glyph's entry:\n{text}");
        let at = |needle: &str| lines.iter().position(|l| l.contains(needle)).unwrap();
        assert!(
            at("Emergency quit") < at("g gather") && at("d show or fold done") < at("affordable now"),
            "the {w}x{h}{how} help overlay does not keep the order key map, screens' keys, legend:\n{text}"
        );
        for screen in SCREEN_ENTRIES {
            let line = &lines[at(screen)];
            let others = HELP_ENTRIES.iter().filter(|o| *o != screen && line.contains(*o)).count();
            assert_eq!(others, 0, "at {w}x{h}{how} the screen keys {screen:?} share a line: {line:?}");
        }
    }
}

/// Where the overlay has the rows, nothing is packed: each entry has its line
/// and a blank line parts the lists.
/// @spec TUI-NAV-124
#[test]
fn the_help_overlay_with_room_draws_one_entry_to_a_line() {
    let state = settled();
    let view = ViewState { overlay: Some(Overlay::Help), ..looking_at(&state, near_row()) };
    let d = cold(&state, &view, 120, 60);
    let lines = region_lines(&d.buffer, d.buffer.area);
    for needle in HELP_ENTRIES {
        let holding: Vec<&String> = lines.iter().filter(|l| l.contains(needle)).collect();
        assert_eq!(holding.len(), 1, "at 120x60 {needle:?} should be on exactly one line");
        let others = HELP_ENTRIES.iter().filter(|o| *o != needle && holding[0].contains(*o)).count();
        assert_eq!(others, 0, "at 120x60 {needle:?} shares its line with another entry: {:?}", holding[0]);
        assert!(!shows_cursor_entry(holding[0]), "at 120x60 {needle:?} shares its line with the cursor glyph's entry");
    }
    assert_eq!(lines.iter().filter(|l| shows_cursor_entry(l)).count(), 1, "at 120x60 the cursor glyph's entry should have one line");
    let at = |needle: &str| lines.iter().position(|l| l.contains(needle)).unwrap();
    for after in ["Emergency quit", "d show or fold done"] {
        assert!(lines[at(after) + 1].trim().is_empty(), "at 120x60 no blank line follows the list ending {after:?}");
    }
}

/// It packs as little as fits: where packing the legend alone leaves room, the
/// key map keeps one entry to a line.
/// @spec TUI-NAV-124
#[test]
fn the_help_overlay_packs_the_legend_before_the_key_map() {
    let state = settled();
    let view = ViewState { overlay: Some(Overlay::Help), ..looking_at(&state, near_row()) };
    let d = cold(&state, &view, 100, 30);
    let lines = region_lines(&d.buffer, d.buffer.area);
    let line_of = |needle: &str| lines.iter().find(|l| l.contains(needle)).unwrap();
    for needle in &HELP_ENTRIES[..KEY_ENTRY_COUNT] {
        let line = line_of(needle);
        let others = HELP_ENTRIES.iter().filter(|o| *o != needle && line.contains(*o)).count();
        assert_eq!(others, 0, "at 100x30 the key map is packed although the legend alone would fit: {line:?}");
    }
    let packed = HELP_ENTRIES[KEY_ENTRY_COUNT + SCREEN_ENTRIES.len()..]
        .iter()
        .any(|needle| HELP_ENTRIES.iter().filter(|o| line_of(needle).contains(*o)).count() > 1);
    assert!(packed, "at 100x30 the legend should be packed for the overlay to fit:\n{}", lines.join("\n"));
}

/// The legend's entry for the cursor glyph, packed or one to a line.
fn shows_cursor_entry(line: &str) -> bool {
    line.contains("\u{25b8} cursor") || line.contains("\u{25b8}  cursor")
}

/// How many of `HELP_ENTRIES` come from the key map, which lists them first.
const KEY_ENTRY_COUNT: usize = 17;

/// The screens' keys among `HELP_ENTRIES`.
const SCREEN_ENTRIES: &[&str] =
    &["g gather / f forage / r rummage", "p promote (kitten)", "p send a pen-pal (colony)", "d show or fold done"];

/// One distinctive phrase from every entry of the key map, the screens' keys,
/// and the glyph legend, none a substring of another. The legend's cursor entry
/// is the one whose meaning is a word another entry also uses, so it is matched
/// with its glyph by `shows_cursor_entry`.
const HELP_ENTRIES: &[&str] = &[
    "Jump to that section",
    "Next / previous revealed section",
    "Move the cursor, cycle sections",
    "Previous / next group header",
    "commits an armed row",
    "Toggle the inspector",
    "Open the count prompt",
    "x max affordable",
    "Increment / decrement a count row",
    "Find-anything palette",
    "Journal: full scrollback",
    "Observe the sky",
    "Game speed down / up",
    "This help overlay",
    "Quit (saves first)",
    "Close the topmost transient",
    "Emergency quit",
    "g gather / f forage / r rummage",
    "p promote (kitten)",
    "p send a pen-pal (colony)",
    "d show or fold done",
    "affordable now",
    "partially affordable",
    "revealed, not yet affordable",
    "owned / done",
    "milestone / new content",
    "flavor event",
    "mechanical event",
    "idle kitten",
    "the Elder",
    "unread offline report",
    "open sky-event window",
];

/// An open palette is the palette widget's drawing, and the chrome keeps
/// rendering behind it.
/// @spec TUI-NAV-038
#[test]
fn an_open_palette_is_the_palette_widgets_drawing() {
    let state = settled();
    let query = probe(StructureId::ALL[7]);

    for size in SIZES {
        let (w, h) = size;
        let bare = cold(&state, &looking_at(&state, near_row()), w, h);
        let view = ViewState {
            overlay: Some(Overlay::Palette { query: query.clone(), selected: 0 }),
            ..looking_at(&state, near_row())
        };
        let d = cold(&state, &view, w, h);

        assert_ne!(
            region_lines(&d.buffer, d.buffer.area),
            region_lines(&bare.buffer, bare.buffer.area),
            "at {w}x{h} opening the palette changed nothing on screen"
        );
        assert_some_region_is(&d, size, "find palette", "overlays::palette", |f, rect| {
            overlays::palette(f, rect, &state, &view)
        });
        assert_chrome_unchanged(&bare, &d, "the palette opened");
    }
}

/// The palette narrows as characters arrive, which it can only do if the frame
/// hands it the query the player is typing rather than a palette of its own.
/// @spec TUI-NAV-038
#[test]
fn the_palettes_live_query_reaches_the_widget() {
    let state = settled();
    let typed = probe(StructureId::ALL[7]);
    let with = |query: &str| ViewState {
        overlay: Some(Overlay::Palette { query: query.to_string(), selected: 0 }),
        ..looking_at(&state, near_row())
    };

    let empty = cold(&state, &with(""), FLOOR_W, FLOOR_H);
    let narrowed = cold(&state, &with(&typed), FLOOR_W, FLOOR_H);
    let body = body_rect(&empty.layout, FLOOR_W);

    assert_ne!(
        region_text(&empty.buffer, body),
        region_text(&narrowed.buffer, body),
        "the palette drew the same results before and after {typed:?} was typed; the query is \
         view state, and the frame is what carries it to the widget"
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-027, TUI-NAV-050 — the returning player's banner
// ---------------------------------------------------------------------------

/// An unread report puts the banner widget in the banner row the layout carved
/// from the body — and the rest of the frame carries on around it, because a
/// banner demands nothing.
/// @spec TUI-NAV-027, TUI-NAV-050
#[test]
fn an_unread_report_puts_the_banner_widget_under_the_tab_bar() {
    let state = settled();
    let view = returning(&state);

    for size in SIZES {
        let (w, h) = size;
        let d = cold(&state, &view, w, h);
        let banner = d.layout.banner.unwrap_or_else(|| {
            panic!(
                "at {w}x{h} a report is unread and the layout gave the frame no banner row; \
                 the banner takes its row from the body (TUI-RENDER-003)"
            )
        });

        assert_region_is(&d, banner, size, "banner row", "overlays::banner", |f| {
            overlays::banner(f, banner, &state, &view)
        });
        assert!(
            has_content(&d.buffer, banner),
            "at {w}x{h} the banner row is blank; it is the first thing a returning player sees"
        );
        assert_eq!(banner.y, d.layout.tabs.y + 1, "the banner sits directly under the tab bar");
    }
}

/// With nothing unread there is no banner row and no banner anywhere: the row
/// goes back to the body. A banner that was always there would be furniture,
/// not news.
/// @spec TUI-NAV-027
#[test]
fn a_dismissed_report_leaves_no_banner_behind() {
    let state = settled();
    let mut view = returning(&state);
    let showing = cold(&state, &view, FLOOR_W, FLOOR_H);
    let banner = showing.layout.banner.expect("an unread report shows a banner");
    assert!(
        has_content(&showing.buffer, banner),
        "an unread report must put something in the banner row before its disappearance can \
         mean anything"
    );
    let banner_text = region_text(&showing.buffer, banner);

    view.reports[0].dismissed = true;
    let dismissed = cold(&state, &view, FLOOR_W, FLOOR_H);

    assert_eq!(dismissed.layout.banner, None, "a dismissed report keeps no row of its own");
    assert!(
        !drawn_anywhere(&dismissed, banner_text.trim()),
        "the banner is still on screen after its report was dismissed"
    );
    assert!(
        sidebar_rect(&dismissed.layout).height > sidebar_rect(&showing.layout).height,
        "the banner's row must go back to the body once it is dismissed"
    );
}

// ---------------------------------------------------------------------------
// TUI-RENDER-004 — below the floor, nothing but the card
// ---------------------------------------------------------------------------

/// Below the floor the frame draws the too-small card and routes nothing:
/// there is no region for a widget to own. The card is drawn, and no part of
/// the chrome, the list or the inspector is.
/// @spec TUI-RENDER-004
#[test]
fn below_the_floor_the_frame_routes_nothing_but_the_card() {
    let state = settled();
    let view = looking_at(&state, near_row());

    for (w, h) in [(FLOOR_W - 1, FLOOR_H), (FLOOR_W, FLOOR_H - 1), (40, 12)] {
        let d = cold(&state, &view, w, h);
        assert!(
            matches!(d.layout.body, Body::TooSmall(_)),
            "{w}x{h} is under the floor and must lay out as the too-small card"
        );
        assert!(has_content(&d.buffer, d.buffer.area), "{w}x{h}: the card must say something");

        for name in [probe(StructureId::ALL[4]), probe(StructureId::ALL[5])] {
            assert!(
                !drawn_anywhere(&d, &name),
                "{w}x{h}: a list row was drawn under the floor, where only the card is drawn"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// TUI-RENDER-001 — composing more parts does not cost the pure-function property
// ---------------------------------------------------------------------------

/// Every surface at once — a banner, a swapped inspector, an overlay — and the
/// frame is still a function of (snapshot, view, size): drawn after a run of
/// other frames at other sizes, it is the frame it would have been drawn cold.
/// @spec TUI-RENDER-001
#[test]
fn a_composed_frame_does_not_depend_on_the_frames_before_it() {
    let state = settled();
    let target = returning(&state);
    let reference = cold(&state, &target, FLOOR_W, FLOOR_H);

    let journal =
        ViewState { overlay: Some(Overlay::Journal { anchor: 2 }), ..looking_at(&state, far_row()) };
    let palette = ViewState {
        overlay: Some(Overlay::Palette { query: "a".to_string(), selected: 0 }),
        ..looking_at(&state, near_row())
    };

    let mut term = terminal(FLOOR_W, FLOOR_H);
    let mut prev: Option<Layout> = None;
    for view in [&journal, &palette, &inspecting(&state, far_row()), &looking_at(&state, near_row())]
    {
        prev = Some(draw(&mut term, &state, view, prev.as_ref()).layout);
    }
    let after = draw(&mut term, &state, &target, prev.as_ref());

    assert_eq!(
        region_lines(&after.buffer, after.buffer.area),
        region_lines(&reference.buffer, reference.buffer.area),
        "a frame drawn after overlays, a swap and another cursor is not the frame those inputs \
         draw cold; composition must cache nothing and remember nothing"
    );
}

/// Drawing is a read. Composing a banner and a badged tab bar must not dismiss
/// the report or mark the section visited on the way past — those chips clear
/// on the player's own signal (TUI-NAV-044), never on being looked at.
/// @spec TUI-RENDER-001
#[test]
fn composing_the_frame_touches_neither_the_snapshot_nor_the_view() {
    let state = settled();
    let view = returning(&state);
    let view_before = format!("{view:?}");
    let state_before = clowder_core::save_string(&state, 0);

    let mut term = terminal(FLOOR_W, FLOOR_H);
    let mut prev: Option<Layout> = None;
    for _ in 0..3 {
        prev = Some(draw(&mut term, &state, &view, prev.as_ref()).layout);
    }
    let last = draw(&mut term, &state, &view, prev.as_ref());

    assert_eq!(format!("{view:?}"), view_before, "the frame must not touch the view state");
    assert_eq!(
        clowder_core::save_string(&state, 0),
        state_before,
        "the frame must not touch the snapshot"
    );
    assert!(
        last.layout.banner.is_some(),
        "the report is still unread after three frames, so the banner is still there; drawing a \
         banner is not reading it"
    );
}

/// The same inputs draw the same frame, with every widget in play, at the
/// floor and above both breakpoints.
/// @spec TUI-RENDER-001
#[test]
fn the_same_inputs_compose_the_same_frame() {
    let state = settled();
    let view = returning(&state);

    for (w, h) in SIZES {
        let first = cold(&state, &view, w, h);
        let second = cold(&state, &view, w, h);
        assert_eq!(
            region_lines(&first.buffer, first.buffer.area),
            region_lines(&second.buffer, second.buffer.area),
            "{w}x{h}: two frames from identical inputs must be identical"
        );
        assert_eq!(first.layout, second.layout, "{w}x{h}: and so must the layouts they used");
    }
}

// ---------------------------------------------------------------------------
// TUI-RENDER-016 — the pane dividers
//
// Side-by-side panes are parted by one column of `│` at each boundary, and
// each column comes out of the list's space: its edge facing the sidebar at
// every width, and its edge facing the inspector as well wherever the
// inspector is a pane of its own. So the layout's rectangles are unchanged —
// the dividers live *inside* the list's rectangle — and whatever the list's
// space holds is fitted to what the dividers leave of it. Where a full-screen
// overlay takes the body there are no side-by-side panes, and so no divider.
//
// "The panes it separates" are the body's panes, so a divider runs from the
// body's first row to its last and stands on no other row: not the header,
// the tab bar, the banner row, the ticker or the hint bar.
// ---------------------------------------------------------------------------

/// The one glyph a divider is drawn from: the single-width light vertical.
const DIVIDER: &str = "│";

/// The columns a layout's dividers stand in: the list's space's edge facing
/// the sidebar, and where the inspector is a pane of its own, its edge facing
/// the inspector. Derived from whichever layout the frame reports drawing, so
/// a dead-band frame is held to the layout it actually drew.
fn divider_columns(l: &Layout) -> Vec<u16> {
    match l.body {
        Body::Narrow { main, .. } => vec![main.x],
        Body::Wide { list, .. } => vec![list.x, list.right() - 1],
        Body::TooSmall(r) => panic!("this claim needs a supported size, not {r:?}"),
    }
}

/// What the dividers leave of the list's space.
fn left_by_dividers(l: &Layout) -> Rect {
    match l.body {
        Body::Narrow { main, .. } => Rect::new(main.x + 1, main.y, main.width - 1, main.height),
        Body::Wide { list, .. } => Rect::new(list.x + 1, list.y, list.width - 2, list.height),
        Body::TooSmall(r) => panic!("this claim needs a supported size, not {r:?}"),
    }
}

/// A meadow far enough along that a heavy row with a consequence summary is
/// listed, alongside everything `settled` already has.
fn far_along() -> GameState {
    let mut s = settled();
    s.ideas = clowder_core::IdeaId::ALL.iter().copied().collect::<BTreeSet<_>>();
    s.revealed.extend(clowder_core::IdeaId::ALL.iter().map(|i| ContentId::Idea(*i)));
    s.discovered = clowder_core::ColonyId::ALL.iter().copied().collect::<BTreeSet<_>>();
    s.peak_devotion = 1.0e9;
    s.generation = 1;
    s.roster_high_water = 100;
    s
}

/// A view with a heavy row armed — so the list's space draws an armed row's
/// expansion beneath it — in whichever section lists one whose arming the
/// frame expands.
fn armed_view(state: &GameState) -> ViewState {
    use clowder_tui::nav::rows::RowKind;
    let (section, key, command) = Section::ALL
        .iter()
        .find_map(|s| {
            rows(state, *s).into_iter().find_map(|r| match r.kind {
                RowKind::Act { command: c @ clowder_core::Command::PassItDown, .. } => {
                    Some((*s, r.key, c))
                }
                _ => None,
            })
        })
        .expect("fixture: some section lists a heavy row with a consequence summary");
    let mut v = ViewState { section: Some(section), ..Default::default() };
    v.focus[section.index()] = Some(key.clone());
    v.armed = Some(clowder_tui::nav::ArmedRow {
        key,
        command,
        expires_at: std::time::Instant::now() + std::time::Duration::from_secs(3_600),
    });
    v
}

/// A view with a count prompt open on the focused row, digits typed.
fn prompting(state: &GameState, key: RowKey, wanted: bool) -> ViewState {
    ViewState {
        inspector_wanted: wanted,
        prompt: Some(clowder_tui::nav::CountPrompt { digits: "12".to_string(), row: Some(key.clone()) }),
        ..looking_at(state, key)
    }
}

/// Every view whose list's space this section's tests draw: the list at the
/// top and deep in its scroll (so both overflow notes are drawn), the list
/// with a focused row in reverse video, a prompt's preview, an armed row's
/// expansion, the inspector swapped in over it, and a banner taking a body
/// row.
fn list_space_views(state: &GameState) -> Vec<(&'static str, ViewState)> {
    vec![
        ("the list near its top", looking_at(state, near_row())),
        ("the list deep in its scroll", looking_at(state, far_row())),
        ("the list with a prompt open", prompting(state, near_row(), false)),
        ("the list with a row armed", armed_view(state)),
        ("the inspector asked for", inspecting(state, near_row())),
        ("the inspector asked for with a prompt open", prompting(state, far_row(), true)),
        ("a banner showing", returning(state)),
    ]
}

/// The claim at one frame: a `│` in every body row of each divider column
/// with nothing else in the cell; no divider glyph at those columns on any
/// row outside the body; and no full-height `│` column anywhere else in the
/// body.
#[track_caller]
fn assert_dividers_stand(d: &Drawn, what: &str) {
    use ratatui::style::{Color, Modifier};

    let area = d.buffer.area;
    let (w, h) = (area.width, area.height);
    let body = main_rect(&d.layout);
    let columns = divider_columns(&d.layout);

    for &x in &columns {
        for y in 0..h {
            let cell = d.buffer.cell((x, y)).expect("cell");
            if (body.y..body.bottom()).contains(&y) {
                assert_eq!(
                    cell.symbol(),
                    DIVIDER,
                    "at {w}x{h} with {what}, the divider at column {x} is broken at row {y} by \
                     `{}`: a divider runs the full height of the panes it parts.\n body:\n{}",
                    cell.symbol(),
                    region_text(&d.buffer, sidebar_rect(&d.layout).union(body)),
                );
                assert_eq!(
                    (cell.modifier, cell.bg),
                    (Modifier::empty(), Color::Reset),
                    "at {w}x{h} with {what}, the divider cell ({x},{y}) carries {:?} on {:?}. \
                     Its cells hold the glyph and nothing else: a focused row's reverse video, \
                     an armed row's expansion and the list's position indicator all end at the \
                     width the dividers leave.",
                    cell.modifier,
                    cell.bg
                );
            } else {
                assert_ne!(
                    cell.symbol(),
                    DIVIDER,
                    "at {w}x{h} with {what}, a divider glyph stands at ({x},{y}), outside the \
                     body's panes: a divider stands on no row but the panes' own — never the \
                     header, tab bar, banner, ticker or hint bar"
                );
            }
        }
    }

    for x in (0..w).filter(|x| !columns.contains(x)) {
        let full = (body.y..body.bottom())
            .all(|y| d.buffer.cell((x, y)).is_some_and(|c| c.symbol() == DIVIDER));
        assert!(
            !full,
            "at {w}x{h} with {what}, column {x} is a full-height divider, but the dividers are \
             taken from the list's space at its edges ({columns:?}) and stand nowhere else"
        );
    }
}

/// At every width from the floor to well past both breakpoints, at two
/// heights, with every kind of content in the list's space: one divider where
/// the inspector swaps over the list (80–99), two where it is a pane of its
/// own (100 and wider), each a `│` in every body row of the list's edge
/// column facing its neighbour, with nothing else in the cell, on no row
/// outside the body, and no full-height divider anywhere else. The frame's
/// regions are the layout algebra's own — the dividers are drawn in place and
/// resize nothing.
///
/// @spec TUI-RENDER-016
#[test]
fn a_divider_stands_at_each_boundary_between_side_by_side_panes_and_nowhere_else() {
    let state = far_along();
    let views = list_space_views(&state);

    for h in [FLOOR_H, 40] {
        for w in FLOOR_W..=160 {
            for (what, view) in &views {
                let d = cold(&state, view, w, h);
                let banner = view.reports.iter().any(|r| !r.dismissed);
                assert_eq!(
                    d.layout,
                    render::layout::compute(Rect::new(0, 0, w, h), None, banner),
                    "at {w}x{h} with {what}, the frame's regions are not the layout algebra's: \
                     the dividers are taken from inside the list's space and resize nothing"
                );
                let expected = if w < 100 { 1 } else { 2 };
                assert_eq!(
                    divider_columns(&d.layout).len(),
                    expected,
                    "fixture: a cold frame at {w} columns has {expected} pane boundary(ies)"
                );
                assert_dividers_stand(&d, what);
            }
        }
    }
}

/// Within a breakpoint's dead-band the divider follows whichever layout is
/// drawn: sweeping the width down from wide and back up, frame after frame,
/// each frame's dividers are the ones its own reported layout calls for —
/// two while the inspector is still a pane below 100 columns, one once it
/// swaps — and the layout is the algebra's for that width and that previous
/// layout.
///
/// @spec TUI-RENDER-016
#[test]
fn inside_a_dead_band_the_dividers_follow_the_layout_on_screen() {
    let state = far_along();
    let view = looking_at(&state, near_row());
    let mut widths: Vec<u16> = (FLOOR_W..=130).rev().collect();
    widths.extend(FLOOR_W..=130);

    let mut prev: Option<Layout> = None;
    let mut wide_below_breakpoint = false;
    for w in widths {
        let mut term = terminal(w, FLOOR_H);
        let d = draw(&mut term, &state, &view, prev.as_ref());
        assert_eq!(
            d.layout,
            render::layout::compute(Rect::new(0, 0, w, FLOOR_H), prev.as_ref(), false),
            "at {w}x{FLOOR_H} after {prev:?}, the frame's regions are not the layout algebra's"
        );
        if w < 100 && matches!(d.layout.body, Body::Wide { .. }) {
            wide_below_breakpoint = true;
        }
        assert_dividers_stand(&d, "the width swept through a dead-band");
        prev = Some(d.layout);
    }
    assert!(
        wide_below_breakpoint,
        "fixture: the downward sweep should keep the inspector's pane below 100 columns"
    );
}

/// Whatever holds the list's space fits its lines to the width the dividers
/// leave, rather than being drawn at full width and cut by them.
///
/// Where the inspector swaps in over the list, the space the dividers leave is
/// exactly the inspector widget's drawing into that narrower rectangle. For the
/// list, two frames whose dividers leave the same width — one divider at a
/// narrow width `n`, two at `n + 21` where the inspector is a pane — draw the
/// same list at the same height, so what the dividers leave must match cell
/// for cell, reverse video and colours included; and in each the leading
/// column of a focused row is its cursor glyph and of an overflow note its
/// first digit, so neither edge was cut. Checked at every narrow width that
/// has a wide partner, at two heights, for the list at its top and deep in its
/// scroll, with a prompt's preview, and with a row armed.
///
/// @spec TUI-RENDER-016
#[test]
fn whatever_holds_the_lists_space_fits_its_lines_to_the_width_the_dividers_leave() {
    let state = far_along();

    // The swapped inspector.
    for h in [FLOOR_H, 40] {
        for w in FLOOR_W..100 {
            for view in [
                inspecting(&state, near_row()),
                inspecting(&state, far_row()),
                prompting(&state, far_row(), true),
            ] {
                let d = cold(&state, &view, w, h);
                let space = left_by_dividers(&d.layout);
                let alone = widget_alone(w, h, |f| inspector::draw(f, space, &state, &view));
                assert!(has_content(&alone, space), "fixture: the inspector has something to say");
                assert_eq!(
                    region_lines(&d.buffer, space),
                    region_lines(&alone, space),
                    "at {w}x{h}, the inspector swapped in over the list is not the inspector \
                     drawn into the width its divider leaves ({space:?}).\n composed:\n{}\n \
                     inspector alone:\n{}",
                    region_text(&d.buffer, space),
                    region_text(&alone, space),
                );
            }
        }
    }

    // The list.
    let list_views = [
        ("the list near its top", looking_at(&state, near_row())),
        ("the list deep in its scroll", looking_at(&state, far_row())),
        ("the list with a prompt open", prompting(&state, near_row(), false)),
        ("the list with a row armed", armed_view(&state)),
    ];
    for h in [FLOOR_H, 40] {
        for n in FLOOR_W..=98 {
            let wide = n + 21;
            for (what, view) in &list_views {
                let a = cold(&state, view, n, h);
                let b = cold(&state, view, wide, h);
                assert!(matches!(a.layout.body, Body::Narrow { .. }), "fixture: {n} is narrow");
                assert!(matches!(b.layout.body, Body::Wide { .. }), "fixture: {wide} has a pane");
                let (sa, sb) = (left_by_dividers(&a.layout), left_by_dividers(&b.layout));
                assert_eq!(
                    (sa.width, sa.height),
                    (sb.width, sb.height),
                    "fixture: {n} and {wide} columns leave the list the same space"
                );

                let cells = |d: &Drawn, s: Rect| -> Vec<Vec<ratatui::buffer::Cell>> {
                    (s.y..s.bottom())
                        .map(|y| (s.x..s.right()).map(|x| d.buffer.cell((x, y)).expect("cell").clone()).collect())
                        .collect()
                };
                assert_eq!(
                    cells(&a, sa),
                    cells(&b, sb),
                    "with {what} at height {h}, the list drawn at {n} columns (one divider) and \
                     at {wide} (two) differs within the {} columns both leave it: the list is \
                     fitted to the width the dividers leave, not drawn full width and cut.\n \
                     at {n}:\n{}\n at {wide}:\n{}",
                    sa.width,
                    region_text(&a.buffer, sa),
                    region_text(&b.buffer, sb),
                );

                for (d, s, at) in [(&a, sa, n), (&b, sb, wide)] {
                    let lines = region_lines(&d.buffer, s);
                    let focused = lines
                        .iter()
                        .find(|l| l.contains('▸'))
                        .unwrap_or_else(|| panic!("fixture: {what} at {at} shows the cursor"));
                    assert!(
                        focused.starts_with('▸'),
                        "with {what} at {at}x{h}, the focused row does not begin at the first \
                         column the divider leaves: {focused:?}"
                    );
                    for note in lines.iter().filter(|l| l.contains("more above") || l.contains("more below")) {
                        assert!(
                            note.starts_with(|c: char| c.is_ascii_digit()),
                            "with {what} at {at}x{h}, the list's position indicator lost its \
                             leading column to a divider: {note:?}"
                        );
                    }
                }
            }
        }
    }
}

/// Where a full-screen overlay takes the body there are no side-by-side
/// panes, so there is no divider: at every width from the floor to past both
/// breakpoints, for each overlay, some region of the body is exactly the
/// overlay's own drawing — nothing cuts through its text — and no body column
/// is a full-height `│`.
///
/// @spec TUI-RENDER-016
#[test]
fn no_divider_cuts_through_a_full_screen_overlay() {
    let state = far_along();
    let overlays_open = [
        ("the Journal", Overlay::Journal { anchor: 0 }),
        ("help", Overlay::Help),
        ("the palette", Overlay::Palette { query: probe(StructureId::ALL[7]), selected: 0 }),
    ];

    for h in [FLOOR_H, 40] {
        for w in FLOOR_W..=160 {
            for (what, overlay) in &overlays_open {
                for base in [looking_at(&state, near_row()), inspecting(&state, near_row())] {
                    let view = ViewState { overlay: Some(overlay.clone()), ..base };
                    let d = cold(&state, &view, w, h);
                    let size = (w, h);
                    match overlay {
                        Overlay::Journal { .. } => assert_some_region_is(&d, size, what, "overlays::journal", |f, r| {
                            overlays::journal(f, r, &state, &view)
                        }),
                        Overlay::Help => assert_some_region_is(&d, size, what, "overlays::help", |f, r| {
                            overlays::help(f, r)
                        }),
                        Overlay::Palette { .. } => assert_some_region_is(&d, size, what, "overlays::palette", |f, r| {
                            overlays::palette(f, r, &state, &view)
                        }),
                    }
                    let body = sidebar_rect(&d.layout).union(main_rect(&d.layout));
                    let body = pane_rect(&d.layout).map_or(body, |p| body.union(p));
                    for x in body.x..body.right() {
                        let full = (body.y..body.bottom())
                            .all(|y| d.buffer.cell((x, y)).is_some_and(|c| c.symbol() == DIVIDER));
                        assert!(
                            !full,
                            "at {w}x{h} with {what} open, column {x} is a full-height divider \
                             through the overlay; an overlay takes the body and there are no \
                             panes for a divider to part"
                        );
                    }
                }
            }
        }
    }
}
