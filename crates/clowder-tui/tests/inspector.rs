//! The inspector: truth one keypress away, and none at all when the pane fits.
//!
//! Two claims live in this file, and they are not the same claim.
//!
//! **Where it goes** (TUI-NAV-017, 029, 047, 048). The inspector is a
//! *persistent right-hand pane* wherever the terminal is wide enough for one,
//! and a *swap in place of the list* at the floor — never a popup, because a
//! popup would cover the chrome and read as something that stopped the world,
//! and this game never stops the world. Which of the two it is must be derived
//! from the width every frame rather than remembered, so it cannot disagree
//! with the layout after a resize; that is why `ViewState` stores only whether
//! the player *asked*, and `placement` takes the width.
//!
//! **What it says** (TUI-NAV-018). This is where tenet 3 is paid for: the row
//! shows a flavour name, and the inspector states the plain mechanical fact
//! behind it — the exact next cost with have and need per component, the
//! time-to-afford at the current net rate, the owned count and effects, and the
//! plain reason anything is not actionable. Every one of those answers is a
//! *rule*, so every one of them comes from a core query (`next_cost`,
//! `net_rate`, `lock_reason`, `affordability`). A shell that decided
//! affordability, a lock reason, or a rate for itself would be a game rule in
//! the frontend.
//!
//! **How that provenance is tested.** No test can watch which function an
//! implementation calls. What a test *can* do is pick states where the naive
//! shell-side answer and the core's answer disagree, and demand the core's.
//! Two such states are used here:
//!
//! * a job whose cost list is empty and whose stock therefore covers it
//!   completely (`affordability == 1.0`), which the core nonetheless reports as
//!   locked — a shell checking stock against cost would see nothing wrong;
//! * a Tradition, whose price is not denominated in any `ResourceId` at all, so
//!   its cost list is empty and there is no resource arithmetic that could
//!   produce the answer.
//!
//! **How rendered facts are located.** The inspector's exact wording is its own
//! business and nothing in this leaf pins it, so the assertions are about
//! *numbers* and *words* rather than sentences: a number is a maximal run of
//! digits and dots, compared against `render::fmt::exact` (the crate's own
//! unabbreviated form, per TUI-RENDER-012); a word is a run of three or more
//! letters. Facts that would need a phrasing to test — the exact shape of the
//! mechanical sentence, the units of the time-to-afford — are tested by what
//! they must *depend on* instead: two states that differ only in the core's
//! answer must not render the same.
//!
//! Sizes and breakpoints are never re-derived here: `render::layout::compute`
//! already owns them (TUI-RENDER-004..008), and every rectangle a test draws
//! into is the rectangle that layout would hand the inspector at that size.

use std::collections::BTreeSet;

use clowder_core::content::structure_def;
use clowder_core::{
    ContentId, GameState, JobId, Kitten, KittenTrait, LockReason, Location, ResourceId,
    StructureGate, StructureId, TraditionId,
};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::Terminal;

use clowder_tui::nav::rows::rows;
use clowder_tui::nav::{RowKey, Section, ViewState};
use clowder_tui::render::fmt;
use clowder_tui::render::layout::{compute, Body};
use clowder_tui::render::widgets::inspector::{self, Placement};

// ---------------------------------------------------------------------------
// Sizes
//
// The floor and the breakpoints are stated by the specs; the exact column
// widths of sidebar, list and pane are deferred to measurement, so nothing here
// pins one — every rectangle comes from `compute`.
// ---------------------------------------------------------------------------

/// TUI-NAV-029: below this the frame is a too-small card.
const FLOOR_W: u16 = 80;
const FLOOR_H: u16 = 24;

/// TUI-NAV-017: "at least about 100 columns" is where the pane appears.
const PANE_BREAKPOINT: u16 = 100;

/// TUI-NAV-048: where the list caps and the surplus becomes the inspector's.
const CAP_BREAKPOINT: u16 = 120;

/// A comfortable height for every layout question; none of them is about height.
const TALL: u16 = 40;

/// Wide enough that the inspector's pane has room for a sentence and a number
/// on one line, so a test about *what* is written is never a test about
/// wrapping.
const WIDE: u16 = 200;

/// The widest terminal any test walks to. Past the cap breakpoint every extra
/// column is the inspector's, so there is nothing new to learn beyond this.
const WIDEST: u16 = 240;

fn area(w: u16, h: u16) -> Rect {
    Rect::new(0, 0, w, h)
}

/// The rectangle layout hands the inspector as a persistent pane.
fn pane_rect(w: u16, h: u16) -> Rect {
    match compute(area(w, h), None, false).body {
        Body::Wide { inspector, .. } => inspector,
        other => panic!("fixture: {w}x{h} should lay out three columns, but laid out {other:?}"),
    }
}

/// The rectangle layout hands the inspector when it swaps in over the list.
fn swap_rect(w: u16, h: u16) -> Rect {
    match compute(area(w, h), None, false).body {
        Body::Narrow { main, .. } => main,
        other => panic!("fixture: {w}x{h} should lay out sidebar and list, but laid out {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Drawing and reading
// ---------------------------------------------------------------------------

/// A glyph no widget would ever draw, painted over the whole terminal so that
/// anything the inspector writes outside its rectangle is visible.
const SENTINEL: char = '#';

fn terminal(w: u16, h: u16) -> Terminal<TestBackend> {
    Terminal::new(TestBackend::new(w, h)).expect("TestBackend is infallible")
}

/// Draw the inspector into `rect` of an otherwise blank terminal.
fn render(state: &GameState, view: &ViewState, w: u16, h: u16, rect: Rect) -> Buffer {
    let view = &reachable(state, view);
    let mut term = terminal(w, h);
    term.draw(|f| inspector::draw(f, rect, state, view))
        .expect("TestBackend is infallible");
    term.backend().buffer().clone()
}

/// The view a player could be in with the same row under the cursor: where
/// that row is an Ideas row folded into the done group, the group opened by
/// its key first, the focus kept on the row (TUI-NAV-113).
fn reachable(state: &GameState, view: &ViewState) -> ViewState {
    let mut v = view.clone();
    let Some(section) = v.section else { return v };
    let Some(key) = v.focus[section.index()].clone() else { return v };
    if clowder_tui::nav::rows::drawn_rows(state, section, &v).iter().any(|r| r.key == key)
        || !rows(state, section).iter().any(|r| r.key == key)
    {
        return v;
    }
    let d = clowder_tui::nav::keys::resolve(
        crossterm::event::KeyEvent::new(crossterm::event::KeyCode::Char('d'), crossterm::event::KeyModifiers::NONE),
        &v,
    );
    clowder_tui::nav::update(d, &mut v, state);
    v.last_repeat = None;
    v.focus[section.index()] = Some(key);
    v
}

/// Draw the inspector into `rect` of a terminal already covered in `SENTINEL`,
/// so that the cells it does *not* own can be checked for damage.
fn render_over_sentinel(
    state: &GameState,
    view: &ViewState,
    w: u16,
    h: u16,
    rect: Rect,
) -> Buffer {
    let mut term = terminal(w, h);
    term.draw(|f| {
        let full = f.area();
        let fill: Vec<Line<'static>> = (0..full.height)
            .map(|_| Line::from(SENTINEL.to_string().repeat(full.width as usize)))
            .collect();
        f.render_widget(Paragraph::new(fill), full);
        inspector::draw(f, rect, state, view);
    })
    .expect("TestBackend is infallible");
    term.backend().buffer().clone()
}

/// The text of a rectangle, one string per row.
fn region_lines(buf: &Buffer, rect: Rect) -> Vec<String> {
    (rect.y..rect.bottom())
        .map(|y| {
            (rect.x..rect.right())
                .map(|x| buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "))
                .collect::<String>()
        })
        .collect()
}

fn text(buf: &Buffer, rect: Rect) -> String {
    region_lines(buf, rect).join("\n")
}

/// The inspector's drawing at a width that lays out three columns.
fn pane_text(state: &GameState, view: &ViewState, w: u16, h: u16) -> String {
    let rect = pane_rect(w, h);
    text(&render(state, view, w, h, rect), rect)
}

fn used_lines(buf: &Buffer, rect: Rect) -> usize {
    region_lines(buf, rect)
        .iter()
        .filter(|l| !l.trim().is_empty())
        .count()
}

fn longest_line(buf: &Buffer, rect: Rect) -> usize {
    region_lines(buf, rect)
        .iter()
        .map(|l| l.trim_end().chars().count())
        .max()
        .unwrap_or(0)
}

/// The non-blank lines of a drawing, trimmed — the unit a "what did it add"
/// comparison is made in.
fn line_set(s: &str) -> BTreeSet<String> {
    s.lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

/// What one drawing says that another does not.
fn extra_lines(with: &str, without: &str) -> BTreeSet<String> {
    let base = line_set(without);
    line_set(with).into_iter().filter(|l| !base.contains(l)).collect()
}

/// Every number on screen, as a maximal run of digits and dots. Token-wise
/// rather than substring-wise, so a two-digit number cannot be "found" inside a
/// four-digit one.
fn numbers(s: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut cur = String::new();
    for c in s.chars() {
        if c.is_ascii_digit() || c == '.' {
            cur.push(c);
        } else {
            let token = cur.trim_matches('.').to_string();
            if !token.is_empty() {
                out.insert(token);
            }
            cur.clear();
        }
    }
    let token = cur.trim_matches('.').to_string();
    if !token.is_empty() {
        out.insert(token);
    }
    out
}

/// Every word of three or more letters, lowercased.
fn words(s: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut cur = String::new();
    for c in s.chars() {
        if c.is_alphabetic() {
            cur.extend(c.to_lowercase());
        } else {
            if cur.chars().count() >= 3 {
                out.insert(cur.clone());
            }
            cur.clear();
        }
    }
    if cur.chars().count() >= 3 {
        out.insert(cur);
    }
    out
}

/// The words of every resource's name, so a test about *mechanics* is not a
/// test about which resources an item happens to cost.
fn resource_words() -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for r in ResourceId::ALL {
        for w in words(&spaced(&format!("{r:?}"))) {
            out.insert(w);
        }
    }
    out
}

/// Split an identifier-shaped name into words the way a label would.
fn spaced(tag: &str) -> String {
    let mut out = String::new();
    for (i, c) in tag.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

// ---------------------------------------------------------------------------
// Fixtures
//
// Content is chosen by *shape* rather than by identity, so these tests describe
// the inspector rather than the game's tables.
// ---------------------------------------------------------------------------

fn ungated(s: StructureId) -> bool {
    matches!(structure_def(s).gate, StructureGate::Always)
}

/// A structure that costs more than one resource and is gated on nothing, so
/// "have/need per component" has several components and the only thing standing
/// between the player and it is the price.
fn multi_component_structure() -> StructureId {
    StructureId::ALL
        .into_iter()
        .find(|s| ungated(*s) && structure_def(*s).base_costs.len() >= 2)
        .expect("fixture: some ungated structure costs more than one resource")
}

/// A structure that produces something and houses nobody.
fn producer() -> StructureId {
    StructureId::ALL
        .into_iter()
        .find(|s| {
            ungated(*s) && !structure_def(*s).productions.is_empty() && structure_def(*s).housing == 0
        })
        .expect("fixture: some ungated structure produces something and houses nobody")
}

/// A structure that houses kittens and produces nothing — a different mechanical
/// story from `producer`, told about the same kind of row.
fn housing() -> StructureId {
    StructureId::ALL
        .into_iter()
        .find(|s| {
            ungated(*s) && structure_def(*s).housing > 0 && structure_def(*s).productions.is_empty()
        })
        .expect("fixture: some ungated structure houses kittens and produces nothing")
}

/// A meadow with the given structures revealed and nothing in stock.
fn meadow(revealed: &[StructureId]) -> GameState {
    let mut s = GameState::new(11);
    s.revealed = revealed.iter().map(|b| ContentId::Structure(*b)).collect();
    s
}

fn structure_row(s: StructureId) -> RowKey {
    RowKey::Content(ContentId::Structure(s), "")
}

/// A view focused on one row of one section, and nothing else going on.
fn looking_at(section: Section, key: RowKey) -> ViewState {
    let mut v = ViewState {
        section: Some(section),
        ..Default::default()
    };
    v.focus[section.index()] = Some(key.clone());
    assert!(
        v.focus[section.index()].is_some(),
        "fixture: the view must be focused on {key:?}"
    );
    v
}

/// Assert the focused key really is a row of that section, so a test that finds
/// nothing on screen has found a bug rather than a bad fixture.
fn assert_is_a_row(state: &GameState, section: Section, key: &RowKey) {
    assert!(
        rows(state, section).iter().any(|r| &r.key == key),
        "fixture: {key:?} should be a row of {section:?}"
    );
}

fn plain_kitten() -> Kitten {
    Kitten {
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
    }
}

/// A job the core reports as unlocked in a fresh meadow — so the only thing that
/// can be wrong with it is that nobody is free to take it on.
fn open_job(state: &GameState) -> JobId {
    JobId::ALL
        .into_iter()
        .find(|j| state.job_unlocked(*j))
        .expect("fixture: a fresh meadow has at least one job open")
}

// ===========================================================================
// TUI-NAV-017 — where the inspector goes, and that it never blocks
// ===========================================================================

/// Above the breakpoint the inspector is a *persistent* pane: the player never
/// asks for it, because truth that costs a keypress is truth some players will
/// not pay for. So `wanted` must make no difference at all up here.
/// @spec TUI-NAV-017
#[test]
fn a_wide_terminal_shows_the_inspector_without_being_asked() {
    for w in [PANE_BREAKPOINT, 110, CAP_BREAKPOINT - 1, CAP_BREAKPOINT, WIDE, WIDEST] {
        assert_eq!(
            inspector::placement(w, false),
            Placement::Pane,
            "{w} columns is wide enough for a pane, and a pane is persistent: the player has \
             not pressed Space and must still be looking at the truth"
        );
        assert_eq!(
            inspector::placement(w, true),
            Placement::Pane,
            "{w} columns: asking for the inspector where it is already on screen must not \
             change where it is"
        );
    }
}

/// At the floor there is no room for a third column, so Space swaps the
/// inspector in over the list and Space or Esc puts the list back. Unasked, the
/// inspector is simply not there — the list is what the narrow body is for.
/// @spec TUI-NAV-017
#[test]
fn at_the_floor_the_inspector_is_a_swap_the_player_asks_for() {
    for w in FLOOR_W..PANE_BREAKPOINT {
        assert_eq!(
            inspector::placement(w, true),
            Placement::Swapped,
            "{w} columns has no room for a third column, so an inspector the player asked for \
             takes the list's place"
        );
        assert_eq!(
            inspector::placement(w, false),
            Placement::Absent,
            "{w} columns: nobody asked for the inspector, so the body is the list"
        );
    }
}

/// The placement the inspector reports and the layout the frame draws are the
/// same decision. If they could disagree, a resize would leave an inspector
/// drawn into a rectangle the layout gave to something else.
///
/// Below the floor the frame is a too-small card (TUI-NAV-029) and no width
/// under 80 columns could hold three of them, so the one thing the inspector may
/// never claim down there is a pane.
/// @spec TUI-NAV-017, TUI-NAV-029
#[test]
fn placement_agrees_with_the_layout_the_frame_will_use() {
    for w in FLOOR_W..=WIDEST {
        let body = compute(area(w, TALL), None, false).body;
        for wanted in [false, true] {
            let expected = match body {
                Body::Wide { .. } => Placement::Pane,
                Body::Narrow { .. } => {
                    if wanted {
                        Placement::Swapped
                    } else {
                        Placement::Absent
                    }
                }
                Body::TooSmall(_) => {
                    panic!("fixture: {w}x{TALL} is at or above the floor")
                }
            };
            assert_eq!(
                inspector::placement(w, wanted),
                expected,
                "{w} columns (asked: {wanted}): the layout drew {body:?}, so the inspector's \
                 placement must be {expected:?} — a placement that can disagree with the layout \
                 is a pane drawn over the list after a resize"
            );
        }
    }

    for w in 20..FLOOR_W {
        for wanted in [false, true] {
            assert_ne!(
                inspector::placement(w, wanted),
                Placement::Pane,
                "{w} columns is below the floor, where the frame is a too-small card; there is \
                 no pane to be had"
            );
        }
    }
}

/// The swap is a swap, not a popup. It takes the list's rectangle and nothing
/// else — the header, the tab bar, the ticker and the hint bar are still there,
/// which is how the player can see that the world did not stop. The same holds
/// for the pane: an inspector that painted outside its column would be covering
/// something the layout gave to someone else.
/// @spec TUI-NAV-017
#[test]
fn the_inspector_is_never_a_popup_over_the_chrome() {
    let s = multi_component_structure();
    let state = meadow(&[s]);
    let view = looking_at(Section::Meadow, structure_row(s));
    assert_is_a_row(&state, Section::Meadow, &structure_row(s));

    for (w, h, rect) in [
        (FLOOR_W, FLOOR_H, swap_rect(FLOOR_W, FLOOR_H)),
        (WIDE, TALL, pane_rect(WIDE, TALL)),
    ] {
        let buf = render_over_sentinel(&state, &view, w, h, rect);
        for y in 0..h {
            for x in 0..w {
                if rect.x <= x && x < rect.right() && rect.y <= y && y < rect.bottom() {
                    continue;
                }
                let cell = buf.cell((x, y)).map(|c| c.symbol().to_string()).unwrap_or_default();
                assert_eq!(
                    cell,
                    SENTINEL.to_string(),
                    "{w}x{h}: the inspector wrote to ({x},{y}), outside the {rect:?} it was \
                     given. It is a pane or a swap, never a popup over the chrome"
                );
            }
        }
    }
}

/// "Live-updating for the focused row" means two things, and both are testable:
/// the drawing follows the cursor, and it follows the state under a cursor that
/// has not moved. Nothing is remembered between draws for either to be true.
/// @spec TUI-NAV-017
#[test]
fn the_inspector_follows_the_focused_row_and_the_moving_state() {
    let a = producer();
    let b = housing();
    assert_ne!(a, b, "fixture: two different structures");

    let state = meadow(&[a, b]);
    let on_a = pane_text(&state, &looking_at(Section::Meadow, structure_row(a)), WIDE, TALL);
    let on_b = pane_text(&state, &looking_at(Section::Meadow, structure_row(b)), WIDE, TALL);

    assert!(
        on_a.contains(structure_def(a).name),
        "the inspector must name the row the cursor is on. It read:\n{on_a}"
    );
    assert!(
        on_b.contains(structure_def(b).name),
        "the inspector must name the row the cursor is on. It read:\n{on_b}"
    );
    assert_ne!(
        on_a, on_b,
        "the inspector is the truth about the *focused* row; two rows must not read the same"
    );

    // The same cursor, a state that has moved on: the stock the player is
    // waiting for has arrived, and the pane must already say so.
    let view = looking_at(Section::Meadow, structure_row(a));
    let mut later = meadow(&[a, b]);
    for (r, _) in structure_def(a).base_costs {
        later.resources[r.ix()] = 5_000.0;
    }
    let after = pane_text(&later, &view, WIDE, TALL);
    assert_ne!(
        on_a, after,
        "the pane live-updates for the focused row: the stocks moved and the cursor did not, and \
         the inspector must have noticed without being asked"
    );
}

// ===========================================================================
// TUI-NAV-029 — the placement is derived, never remembered
// ===========================================================================

/// The placement is a pure function of the width and whether the player asked.
/// That is the whole reason `ViewState` stores only the asking: a stored
/// "pane or swap" would survive a resize and then be wrong, and the player's
/// call for the inspector must survive the resize instead.
/// @spec TUI-NAV-029
#[test]
fn placement_is_a_pure_function_of_width_and_asking() {
    // A player who asked for the inspector at the floor, widened the terminal,
    // and narrowed it again: the ask survives, and each width answers for
    // itself.
    let walk = [FLOOR_W, WIDE, FLOOR_W, PANE_BREAKPOINT, 95, WIDEST, FLOOR_W];
    for w in walk {
        assert_eq!(
            inspector::placement(w, true),
            inspector::placement(w, true),
            "{w} columns must answer the same every time it is asked"
        );
    }

    assert_eq!(inspector::placement(FLOOR_W, true), Placement::Swapped);
    assert_eq!(inspector::placement(WIDE, true), Placement::Pane);
    assert_eq!(
        inspector::placement(FLOOR_W, true),
        Placement::Swapped,
        "coming back to the floor from a wide terminal must give the floor's answer; nothing \
         about the placement is carried across a resize"
    );

    // And the asking is the only other input: at one width, the two answers
    // differ only where the width leaves a choice.
    for w in [FLOOR_W, 90, PANE_BREAKPOINT - 1] {
        assert_ne!(
            inspector::placement(w, true),
            inspector::placement(w, false),
            "{w} columns: below the breakpoint the inspector is there only because the player \
             asked, so the ask must matter"
        );
    }
}

// ===========================================================================
// TUI-NAV-047 — one breakpoint, not two
// ===========================================================================

/// The dead-band itself belongs to `render::layout` (TUI-RENDER-008), which
/// enters a layout at its boundary and leaves it a few columns below, and has
/// its own tests. What this file must pin is that the inspector does not bring a
/// *second* breakpoint of its own: two thresholds a few columns apart would put
/// back exactly the flicker the dead-band exists to absorb — a one-column wobble
/// landing between them would swap the body's meaning under the player.
///
/// So the inspector's transition must be the layout's transition: the same
/// width, once, and monotone — once wide enough for a pane, wider is too.
/// @spec TUI-NAV-047
#[test]
fn the_inspector_shares_the_layouts_breakpoint_rather_than_keeping_its_own() {
    let layout_breakpoint = (FLOOR_W..=WIDEST)
        .find(|w| matches!(compute(area(*w, TALL), None, false).body, Body::Wide { .. }))
        .expect("fixture: some width lays out three columns");
    assert_eq!(
        layout_breakpoint, PANE_BREAKPOINT,
        "fixture: the pane breakpoint the specs describe"
    );

    assert_ne!(
        inspector::placement(layout_breakpoint - 1, true),
        Placement::Pane,
        "one column below the layout's breakpoint there is no third column to draw into"
    );
    assert_eq!(
        inspector::placement(layout_breakpoint, false),
        Placement::Pane,
        "at the layout's breakpoint the pane appears — the inspector's threshold is the \
         layout's, not one of its own a few columns away"
    );

    for wanted in [false, true] {
        let mut seen_pane = false;
        for w in FLOOR_W..=WIDEST {
            let is_pane = inspector::placement(w, wanted) == Placement::Pane;
            if is_pane {
                seen_pane = true;
            }
            assert!(
                is_pane || !seen_pane,
                "{w} columns (asked: {wanted}) went back to a swap after a narrower width had \
                 already earned a pane; a second threshold like that flickers on the very \
                 one-column wobble the dead-band is for"
            );
        }
        assert!(seen_pane, "some width in {FLOOR_W}..={WIDEST} must earn a pane");
    }
}

// ===========================================================================
// TUI-NAV-048 — past the cap, the surplus is the inspector's
// ===========================================================================

/// At 120 columns and beyond the list stops growing and every further column
/// goes to the inspector. That is only worth doing if the inspector *spends*
/// them: a pane that renders at a fixed narrow width in a hundred-column column
/// has taken the surplus and wasted it.
/// @spec TUI-NAV-048
#[test]
fn beyond_the_cap_the_surplus_width_is_the_inspectors_and_it_spends_it() {
    // Fixture: the layout really does hand the surplus over, one column at a
    // time, while the list holds still.
    let at_cap = compute(area(CAP_BREAKPOINT, TALL), None, false).body;
    let (capped_list, cap_pane) = match at_cap {
        Body::Wide { list, inspector, .. } => (list.width, inspector.width),
        other => panic!("fixture: {CAP_BREAKPOINT} columns should lay out three panes, got {other:?}"),
    };
    for w in CAP_BREAKPOINT..=WIDEST {
        match compute(area(w, TALL), None, false).body {
            Body::Wide { list, inspector, .. } => {
                assert_eq!(list.width, capped_list, "fixture: {w}: the list is capped");
                assert_eq!(
                    inspector.width,
                    cap_pane + (w - CAP_BREAKPOINT),
                    "fixture: {w}: every column past the cap belongs to the inspector"
                );
            }
            other => panic!("fixture: {w} columns should lay out three panes, got {other:?}"),
        }
        assert_eq!(
            inspector::placement(w, false),
            Placement::Pane,
            "{w} columns lays out three columns, and the third one is the inspector's"
        );
    }

    // And the inspector uses the room it was given.
    let s = multi_component_structure();
    let mut state = meadow(&[s]);
    for (r, _) in structure_def(s).base_costs {
        state.resources[r.ix()] = 3.0;
    }
    let view = looking_at(Section::Meadow, structure_row(s));

    let tight = pane_rect(CAP_BREAKPOINT, TALL);
    let roomy = pane_rect(WIDE, TALL);
    assert!(
        roomy.width > tight.width * 2,
        "fixture: {WIDE} columns should give the inspector a far wider pane than {CAP_BREAKPOINT} \
         does ({} vs {})",
        roomy.width,
        tight.width
    );

    let tight_buf = render(&state, &view, CAP_BREAKPOINT, TALL, tight);
    let roomy_buf = render(&state, &view, WIDE, TALL, roomy);

    assert!(
        longest_line(&roomy_buf, roomy) > tight.width as usize,
        "the inspector drew nothing past column {} of a {}-column pane; the surplus width was \
         given to it precisely so the truth could be laid out instead of stacked. It read:\n{}",
        tight.width,
        roomy.width,
        text(&roomy_buf, roomy)
    );
    assert!(
        used_lines(&roomy_buf, roomy) <= used_lines(&tight_buf, tight),
        "the same facts took more lines in a {}-column pane than in a {}-column one; extra width \
         must buy fewer wrapped lines, never more",
        roomy.width,
        tight.width
    );
}

// ===========================================================================
// TUI-NAV-018 — what the inspector says
// ===========================================================================

/// The flavour name, in full. The list may shorten a name with an ellipsis when
/// its column is tight (TUI-RENDER-014); the inspector is where the whole of it
/// stays available, so a player never has to guess what a truncated row was.
/// @spec TUI-NAV-018
#[test]
fn the_inspector_names_the_item_in_full() {
    let s = multi_component_structure();
    let state = meadow(&[s]);
    let view = looking_at(Section::Meadow, structure_row(s));
    let shown = pane_text(&state, &view, WIDE, TALL);

    assert!(
        shown.contains(structure_def(s).name),
        "the inspector must carry the item's whole flavour name — that is what makes an \
         ellipsis in the list safe. It read:\n{shown}"
    );
    assert!(
        !shown.contains('\u{2026}'),
        "nothing in a {}-column pane needed shortening, so nothing may be shortened. It read:\n{shown}",
        pane_rect(WIDE, TALL).width
    );
}

/// The label TUI-NAV-089 draws in the name's place on a later row of an item,
/// keyed by the row's own tag.
fn later_row_label(key: &RowKey) -> Option<&'static str> {
    match key {
        RowKey::Content(ContentId::Craft(_), "tinker") => Some("tinkerers"),
        RowKey::Colony(_, "penpal") => Some("send a pen-pal"),
        _ => None,
    }
}

/// Whether `key` is a later row of its item in `section` — a row of an item
/// that carries several, other than the first of them listed — which is the
/// row the list draws with a label in its name's place (TUI-NAV-089).
fn is_later_row(state: &GameState, section: Section, key: &RowKey) -> bool {
    let same_item = |k: &RowKey| match (k, key) {
        (RowKey::Content(a, _), RowKey::Content(b, _)) => a == b,
        (RowKey::Kitten(a, _), RowKey::Kitten(b, _)) => a == b,
        (RowKey::Colony(a, _), RowKey::Colony(b, _)) => a == b,
        _ => k == key,
    };
    rows(state, section).iter().find(|r| same_item(&r.key)).is_some_and(|first| &first.key != key)
}

/// The inspector's non-blank lines, trimmed, as the frame draws it at `w`
/// columns with the player having called for it — the persistent pane at and
/// past the breakpoint, the swap in the list's place below it.
fn inspector_lines_in_frame(state: &GameState, view: &ViewState, w: u16, h: u16) -> Vec<String> {
    let view = ViewState { inspector_wanted: true, ..view.clone() };
    let mut term = terminal(w, h);
    let mut layout = None;
    term.draw(|f| layout = Some(clowder_tui::render::frame(f, state, &view, None)))
        .expect("TestBackend is infallible");
    let rect = match layout.expect("frame returns its layout").body {
        Body::Wide { inspector, .. } => inspector,
        Body::Narrow { main, .. } => Rect::new(main.x + 1, main.y, main.width.saturating_sub(1), main.height),
        other => panic!("fixture: {w}x{h} lays out an inspector, not {other:?}"),
    };
    region_lines(term.backend().buffer(), rect)
        .into_iter()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

/// While the cursor is on a row the list draws with a label in its name's
/// place, the inspector states that label whole, beneath the item's name —
/// the label beginning on a line of its own once the name's lines end — at
/// every width, persistent pane and narrow swap alike; on a row drawn with its
/// item's name it states no label. Quantified over every selectable row of
/// every section of a meadow where every recipe, kitten, and colony is listed,
/// at widths from the floor through the pane breakpoint to the widest.
/// @spec TUI-NAV-091
#[test]
fn the_inspector_states_a_later_rows_label_beneath_the_name_at_every_width() {
    let mut state = world(true);
    state.discovered = clowder_core::ColonyId::ALL.into_iter().collect();
    let labels = ["tinkerers", "send a pen-pal"];

    let mut labelled = std::collections::BTreeMap::<&str, usize>::new();
    for w in [FLOOR_W, 90, PANE_BREAKPOINT - 1, PANE_BREAKPOINT, CAP_BREAKPOINT, WIDE, WIDEST] {
        for (section, row) in every_row(&state) {
            let view = looking_at(section, row.key.clone());
            let shown = inspector_lines_in_frame(&state, &view, w, TALL);
            let whr = format!("at {w} columns, on {:?} in {}", row.key, section.title());

            if is_later_row(&state, section, &row.key) {
                let label = later_row_label(&row.key).unwrap_or_else(|| {
                    panic!("fixture: {whr}, a later row with no label TUI-NAV-089 names")
                });
                // The item's name, as the widest pane states it on one line.
                let name = inspector_lines_in_frame(&state, &view, WIDEST, TALL)
                    .into_iter()
                    .next()
                    .expect("the inspector states the item's name");
                let beneath = (1..=shown.len()).any(|i| {
                    let (above, below) = shown.split_at(i);
                    let rest = below.join(" ");
                    above.join(" ") == name
                        && (rest == label || rest.starts_with(&format!("{label} ")))
                });
                assert!(
                    beneath,
                    "{whr}: the list draws {label:?} in this row's name place, so the inspector \
                     states that label whole, beginning beneath the item's name {name:?}. It read:\n{}",
                    shown.join("\n")
                );
                *labelled.entry(label).or_default() += 1;
            } else {
                for label in labels {
                    assert!(
                        !shown.iter().any(|l| l == label),
                        "{whr}: a row drawn with its item's name states no label, but the \
                         inspector states {label:?}. It read:\n{}",
                        shown.join("\n")
                    );
                }
            }
        }
    }
    for label in labels {
        assert!(labelled.get(label).copied().unwrap_or(0) > 0, "fixture: no row labelled {label:?} was inspected");
    }
}

/// The Elder's mark, whole, as the roster's row draws it where it has room
/// (TUI-NAV-105).
const ELDER_MARK: &str = "◆ Elder";

/// One frame drawn at `w` columns with the player having called for the
/// inspector: the list's lines where the list is on screen — the three-pane
/// layout — or none where the narrow-mode swap has it off screen, and the
/// inspector's non-blank lines, trimmed.
fn list_and_inspector_in_frame(
    state: &GameState,
    view: &ViewState,
    w: u16,
    h: u16,
) -> (Option<Vec<String>>, Vec<String>) {
    let view = ViewState { inspector_wanted: true, ..view.clone() };
    let mut term = terminal(w, h);
    let mut layout = None;
    term.draw(|f| layout = Some(clowder_tui::render::frame(f, state, &view, None)))
        .expect("TestBackend is infallible");
    let (list, rect) = match layout.expect("frame returns its layout").body {
        Body::Wide { list, inspector, .. } => (Some(list), inspector),
        Body::Narrow { main, .. } => (None, Rect::new(main.x + 1, main.y, main.width.saturating_sub(1), main.height)),
        other => panic!("fixture: {w}x{h} lays out an inspector, not {other:?}"),
    };
    let buf = term.backend().buffer();
    let inspector =
        region_lines(buf, rect).into_iter().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect();
    (list.map(|r| region_lines(buf, r)), inspector)
}

/// While the cursor is on the row of the kitten the core holds as the Elder,
/// the inspector states that the kitten is the Elder as one line directly
/// beneath the kitten's name — the whole mark, `◆ Elder` — and nothing more:
/// no word of what being the Elder does. It states it at every width, the
/// persistent pane and the narrow-mode swap alike, and whether or not the
/// Elder's row in the list has room for the mark: a name too long for any
/// name's column, whose row carries no mark at all, is stated the Elder here
/// all the same. On every other kitten's row, while another kitten is the
/// Elder and while the core holds none, it states no such line, and no other
/// row of any section states one. Quantified over every kitten of a roster —
/// short names, a name that wraps by its words, a name no column holds — as
/// the Elder, as not the Elder, and with no Elder, at widths from the floor
/// through the pane breakpoint to the widest; and over every selectable row of
/// every section with an Elder standing.
///
/// @spec TUI-NAV-108
#[test]
fn the_inspector_states_the_elders_mark_beneath_the_name_on_the_elders_row_alone_at_every_width() {
    let mut base = world(true);
    base.kittens[1].name = format!("Marmalad{}", "e".repeat(72));
    base.kittens[2].name = "Clementine of the Seven Long Whiskers and the Very Soft Paws".to_string();
    base.elder = None;
    let kittens: Vec<(usize, RowKey)> = rows(&base, Section::Clowder)
        .into_iter()
        .filter_map(|r| match r.key {
            RowKey::Kitten(ix, _) => Some((ix, r.key)),
            _ => None,
        })
        .collect();
    assert_eq!(kittens.len(), base.kittens.len(), "fixture: every kitten's row is listed");

    let (mut stated, mut stated_in_the_swap, mut stated_with_no_room, mut stated_beside_the_row) = (0u32, 0u32, 0u32, 0u32);
    for w in [FLOOR_W, 90, PANE_BREAKPOINT - 1, PANE_BREAKPOINT, CAP_BREAKPOINT, WIDE, WIDEST] {
        for (k, key) in &kittens {
            let k = *k;
            let name = &base.kittens[k].name;
            let view = looking_at(Section::Clowder, key.clone());
            let whr = format!("at {w} columns, on kitten {k}'s row");

            // With no Elder: the name, and nothing else. A kitten's row has
            // no definition, no count, and no cost to state.
            let (_, plain) = list_and_inspector_in_frame(&base, &view, w, TALL);
            assert!(
                plain.join(" ") == *name || plain.concat() == *name,
                "fixture: {whr} with the core holding no Elder, the inspector states the kitten's \
                 name and nothing else. It read:\n{}",
                plain.join("\n")
            );
            assert!(
                !plain.iter().any(|l| l.contains('◆') || l.contains("Elder")),
                "{whr}: the core holds no Elder, but the inspector states one (TUI-NAV-108). It read:\n{}",
                plain.join("\n")
            );

            // Another kitten the Elder: still no such line.
            let other = (k + 1) % base.kittens.len();
            let mut elsewhere = base.clone();
            elsewhere.elder = Some(other);
            let (_, shown) = list_and_inspector_in_frame(&elsewhere, &view, w, TALL);
            assert_eq!(
                shown, plain,
                "{whr} with kitten {other} the Elder: this kitten is not the Elder, so the \
                 inspector states what it states with no Elder at all — no line for the Elder \
                 (TUI-NAV-108)"
            );

            // This kitten the Elder: one line more, the whole mark, beneath the name.
            let mut own = base.clone();
            own.elder = Some(k);
            let (list, shown) = list_and_inspector_in_frame(&own, &view, w, TALL);
            let mut expected = plain.clone();
            expected.push(ELDER_MARK.to_string());
            assert_eq!(
                shown, expected,
                "{whr}, the kitten the core holds as the Elder: the inspector states the kitten's \
                 name, then the whole mark `{ELDER_MARK}` as one line beneath it, and nothing of \
                 what being the Elder does (TUI-NAV-108)"
            );
            stated += 1;
            match list {
                None => stated_in_the_swap += 1,
                Some(lines) => {
                    let row = lines
                        .iter()
                        .find(|l| l.contains(&name.chars().take(8).collect::<String>()))
                        .unwrap_or_else(|| panic!("fixture: {whr}, the kitten's row is drawn in the list beside the pane"));
                    if row.contains('◆') {
                        stated_beside_the_row += 1;
                    } else {
                        stated_with_no_room += 1;
                    }
                }
            }
        }
    }
    assert!(stated > 0, "fixture: the Elder's row was inspected");
    assert!(stated_in_the_swap > 0, "fixture: the Elder's row was inspected with the list off screen");
    assert!(stated_beside_the_row > 0, "fixture: the Elder's row was inspected beside a row carrying the mark");
    assert!(
        stated_with_no_room > 0,
        "fixture: the Elder's row was inspected beside a row whose name left the mark no room"
    );

    // No row but the Elder's states the line, in any section.
    let mut state = base.clone();
    let elder = 0usize;
    state.elder = Some(elder);
    let mut others = 0u32;
    for w in [FLOOR_W, PANE_BREAKPOINT, WIDEST] {
        for (section, row) in every_row(&state) {
            if matches!(row.key, RowKey::Kitten(ix, _) if ix == elder) {
                continue;
            }
            let view = looking_at(section, row.key.clone());
            let shown = inspector_lines_in_frame(&state, &view, w, TALL);
            assert!(
                !shown.iter().any(|l| l.contains('◆')),
                "at {w} columns, on {:?} in {}: this is not the Elder's row, but the inspector \
                 states the Elder's mark (TUI-NAV-108). It read:\n{}",
                row.key,
                section.title(),
                shown.join("\n")
            );
            others += 1;
        }
    }
    assert!(others > 0, "fixture: rows other than the Elder's were inspected");
}

/// The exact next cost, component by component, with what the player has beside
/// what the item needs. Exact, because this is the number a decision turns on
/// and the compact form on the row has already rounded it (TUI-RENDER-012); per
/// component, because a single total cannot tell a player which of two
/// ingredients they are short of.
///
/// The needs come from `next_cost` and the haves from `stock` — both core
/// queries — and the stocks below are chosen so that every number the inspector
/// must show is distinct, and so that one component is covered while another is
/// not.
/// @spec TUI-NAV-018
#[test]
fn the_next_cost_is_exact_and_per_component_with_have_and_need() {
    let s = multi_component_structure();
    let mut state = meadow(&[s]);

    // Distinctive, exactly-representable stocks: one component generously
    // covered, one plainly short.
    let costs = state.next_cost(s);
    assert!(costs.len() >= 2, "fixture: this structure costs several things");
    state.resources[costs[0].0.ix()] = 4_096.5;
    state.resources[costs[1].0.ix()] = 0.25;

    let view = looking_at(Section::Meadow, structure_row(s));
    let shown = pane_text(&state, &view, WIDE, TALL);
    let on_screen = numbers(&shown);

    for (r, need) in state.next_cost(s) {
        let have = state.stock(r);
        assert!(
            on_screen.contains(&fmt::exact(need)),
            "the inspector must state the exact amount of {r:?} this costs ({}); the numbers it \
             showed were {on_screen:?} and it read:\n{shown}",
            fmt::exact(need)
        );
        assert!(
            on_screen.contains(&fmt::exact(have)),
            "the inspector must state what the player has of {r:?} ({}) beside what is needed; \
             the numbers it showed were {on_screen:?} and it read:\n{shown}",
            fmt::exact(have)
        );
    }
}

/// Time-to-afford is computed at the core's *net* rate — production less
/// consumption, throttling and all — and the shell may not derive a rate of its
/// own. A behavioural test cannot see which function was called, so it asks the
/// question the other way round: two states in which everything the inspector
/// shows is identical *except* the core's net rate must not draw the same pane.
/// A shell that ignored the rate, or invented one from the item's own def, could
/// not tell them apart.
/// @spec TUI-NAV-018
#[test]
fn the_time_to_afford_moves_with_the_cores_net_rate() {
    let s = multi_component_structure();
    let (target, _) = state_cost_head(s);

    // Something else in the meadow that makes the resource this item is short
    // of. Copies of it change the net rate and nothing else the inspector shows
    // about the focused row.
    let source = StructureId::ALL
        .into_iter()
        .find(|o| {
            *o != s
                && structure_def(*o)
                    .productions
                    .iter()
                    .any(|(r, amount)| *r == target && *amount > 0.0)
        })
        .expect("fixture: something in the meadow produces the resource this item costs");

    let still = meadow(&[s]);
    let mut flowing = meadow(&[s]);
    flowing.structures[source.ix()] = 6;

    assert_eq!(
        still.next_cost(s),
        flowing.next_cost(s),
        "fixture: the price of the focused item must be the same in both states"
    );
    assert_eq!(
        still.structures[s.ix()], flowing.structures[s.ix()],
        "fixture: the owned count of the focused item must be the same in both states"
    );
    for r in ResourceId::ALL {
        assert_eq!(
            still.stock(r),
            flowing.stock(r),
            "fixture: the stocks must be the same in both states"
        );
    }
    assert_ne!(
        still.net_rate(target),
        flowing.net_rate(target),
        "fixture: {source:?} should move the net rate of {target:?} ({} vs {})",
        still.net_rate(target),
        flowing.net_rate(target)
    );

    let view = looking_at(Section::Meadow, structure_row(s));
    let a = pane_text(&still, &view, WIDE, TALL);
    let b = pane_text(&flowing, &view, WIDE, TALL);

    assert_ne!(
        a, b,
        "the same item, the same price, the same stocks, and a different net rate drew the same \
         pane — so the time-to-afford is not being taken from the core's rate. It read:\n{a}"
    );
}

/// The first resource an item costs, and that cost.
fn state_cost_head(s: StructureId) -> (ResourceId, f64) {
    let def = structure_def(s);
    let (r, amount) = def.base_costs[0];
    (r, amount)
}

/// The owned count. A player deciding on the next copy is deciding against the
/// copies they already have, and the escalating price only makes sense beside
/// the number that escalated it.
/// @spec TUI-NAV-018
#[test]
fn the_owned_count_is_on_screen() {
    const OWNED: u16 = 37;

    let s = multi_component_structure();
    let mut state = meadow(&[s]);
    state.structures[s.ix()] = OWNED as u32;
    for (r, _) in structure_def(s).base_costs {
        state.resources[r.ix()] = 11.5;
    }

    // The count must be recognisable as itself, so no other number the
    // inspector is obliged to show may be spelled the same way.
    let owned = OWNED.to_string();
    for (r, need) in state.next_cost(s) {
        assert_ne!(fmt::exact(need), owned, "fixture: the cost of {r:?} must not read as {owned}");
        assert_ne!(
            fmt::exact(state.stock(r)),
            owned,
            "fixture: the stock of {r:?} must not read as {owned}"
        );
    }

    let view = looking_at(Section::Meadow, structure_row(s));
    let shown = pane_text(&state, &view, WIDE, TALL);
    assert!(
        numbers(&shown).contains(&owned),
        "the player owns {owned} of this and the inspector did not say so; the numbers it showed \
         were {:?} and it read:\n{shown}",
        numbers(&shown)
    );
}

/// The plain reason a thing is not actionable — taken from the core, which is
/// the only place the reason is *decided*.
///
/// This state is the discriminator: the core reports the row as locked while
/// reporting its cost as fully covered, because what is missing is not a
/// resource at all. A shell that worked affordability out for itself, by
/// comparing stock against cost, would see nothing worth mentioning and say
/// nothing — and the player would be left staring at a row that does nothing
/// when they press Enter.
/// @spec TUI-NAV-018
#[test]
fn an_item_the_core_calls_locked_says_so_even_when_the_coffers_are_full() {
    let mut free = GameState::new(11);
    free.kittens = vec![plain_kitten()];
    let job = open_job(&free);
    let id = ContentId::Job(job);

    // One kitten, already at work. The row exists — a job is revealed once
    // there is a clowder to do it — and there is still nobody free to take
    // this one on, which is a lock the coffers cannot explain.
    let mut locked = GameState::new(11);
    let mut busy = plain_kitten();
    busy.job = Some(job);
    locked.kittens = vec![busy];

    assert_eq!(
        locked.affordability(id),
        1.0,
        "fixture: the core reports this row's cost as fully covered"
    );
    assert!(
        matches!(locked.lock_reason(id), Some(LockReason::NoOneFree)),
        "fixture: and yet the core reports it locked, for a reason no stock could explain: {:?}",
        locked.lock_reason(id)
    );
    assert!(
        free.lock_reason(id).is_none(),
        "fixture: with someone free, the core reports nothing wrong"
    );

    let key = RowKey::Content(id, "");
    assert_is_a_row(&locked, Section::Clowder, &key);
    assert_is_a_row(&free, Section::Clowder, &key);
    let view = looking_at(Section::Clowder, key);

    let blocked = pane_text(&locked, &view, WIDE, TALL);
    let open = pane_text(&free, &view, WIDE, TALL);
    let said = extra_lines(&blocked, &open);

    assert!(
        !said.is_empty(),
        "the core calls this row locked and the inspector said nothing the unlocked row does not \
         also say. The reason is the core's to decide and the inspector's to state.\n\
         locked:\n{blocked}\nunlocked:\n{open}"
    );
}

/// A reason is a reason, not a padlock: two items locked for different reasons
/// must not read alike, or "why can't I?" has been answered with "you can't".
///
/// Both states below are the same item at the same price with the same currency
/// in hand; only the core's answer differs — already yours, against not enough
/// of a currency that is not a resource and so cannot be compared against any
/// stock.
/// @spec TUI-NAV-018
#[test]
fn two_different_lock_reasons_do_not_read_the_same() {
    let t = TraditionId::ALL[0];
    let id = ContentId::Tradition(t);

    // A generation has passed, so the ladder exists at all.
    let mut short = GameState::new(11);
    short.generation = 1;

    let mut owned = short.clone();
    owned.traditions.insert(t);

    let mut affordable = short.clone();
    affordable.elder_wisdom = 1_000_000.0;

    assert!(
        matches!(short.lock_reason(id), Some(LockReason::ShortBy { .. })),
        "fixture: with nothing put by, the core reports a shortfall: {:?}",
        short.lock_reason(id)
    );
    assert!(
        matches!(owned.lock_reason(id), Some(LockReason::AlreadyYours)),
        "fixture: once taken, the core reports it as already yours: {:?}",
        owned.lock_reason(id)
    );
    assert!(
        affordable.lock_reason(id).is_none(),
        "fixture: with plenty put by, the core reports nothing wrong"
    );
    assert_eq!(
        short.affordability(id),
        1.0,
        "fixture: this item's price is in no resource at all, so nothing about the player's \
         stocks could produce either answer"
    );

    let key = RowKey::Content(id, "");
    assert_is_a_row(&short, Section::Legacy, &key);
    let view = looking_at(Section::Legacy, key);

    let a = pane_text(&short, &view, WIDE, TALL);
    let b = pane_text(&owned, &view, WIDE, TALL);
    let open = pane_text(&affordable, &view, WIDE, TALL);

    assert!(
        !extra_lines(&a, &open).is_empty(),
        "a shortfall the player cannot see in any resource must still be stated.\n\
         short:\n{a}\nunblocked:\n{open}"
    );
    assert!(
        !extra_lines(&b, &open).is_empty(),
        "an item already taken must say so.\nowned:\n{b}\nunblocked:\n{open}"
    );
    assert_ne!(
        a, b,
        "two different reasons read identically; the inspector's job is to say *why*, not that"
    );
}

/// The plain mechanical statement, and it is this item's rather than a template.
/// The row shows a cute name; the inspector must say what the thing does, or the
/// whimsy has been charged to the player's clarity.
///
/// Wording is nobody's to pin, so the test compares two items with different
/// mechanics — one that produces, one that houses — over the words they use,
/// ignoring their names and every resource name so that a difference in what
/// they *cost* cannot stand in for a difference in what they *do*.
/// @spec TUI-NAV-018
#[test]
fn the_inspector_states_each_items_own_mechanics() {
    let a = producer();
    let b = housing();
    let state = meadow(&[a, b]);

    let shown_a = pane_text(&state, &looking_at(Section::Meadow, structure_row(a)), WIDE, TALL);
    let shown_b = pane_text(&state, &looking_at(Section::Meadow, structure_row(b)), WIDE, TALL);

    let ignore: BTreeSet<String> = resource_words()
        .into_iter()
        .chain(words(structure_def(a).name))
        .chain(words(structure_def(b).name))
        .collect();

    let said_a: BTreeSet<String> = words(&shown_a).difference(&ignore).cloned().collect();
    let said_b: BTreeSet<String> = words(&shown_b).difference(&ignore).cloned().collect();

    assert!(
        !said_a.is_empty(),
        "the inspector said nothing about this item beyond its name and its price. The plain \
         mechanical statement is what the flavour name is borrowing against. It read:\n{shown_a}"
    );
    assert_ne!(
        said_a, said_b,
        "one of these produces something and the other houses kittens, and the inspector used \
         the same words for both — that is a template, not a description.\n{shown_a}\n---\n{shown_b}"
    );
}

// ===========================================================================
// TUI-NAV-018, TUI-NAV-078, TUI-NAV-080 — what the inspector states, whole
//
// The tests above pin the inspector's facts one example at a time. The tests
// below pin each claim over the whole set it quantifies: every structure's
// definition, every job, every recipe, every row that takes an item, every
// store. Wording is still nobody's to pin, so four wording-free instruments
// carry them:
//
// * **numbers** — the figures on screen, compared against `fmt::exact` of the
//   value the spec names (TUI-RENDER-012's exact form). A claim that the
//   inspector states *nothing but* certain figures is a subset test: every
//   number on screen must be one the spec allows on that row.
// * **masked text** — the drawing with every number replaced by one
//   placeholder and whitespace collapsed. Two states whose masked drawings are
//   equal said the same things about different amounts; two whose masked
//   drawings differ said something different (a mark, a note, a wait).
// * **waits** — a stretch of real time on screen, read as a number followed by
//   a plain unit of time (`s`, `min`, `hours`, …), consecutive parts summed.
//   The spec requires plain units of time, so any such rendering is found.
// * **lines** — the non-blank lines of a drawing; the first is the name.
// ===========================================================================

use clowder_core::content as tables;
use clowder_core::content::TICKS_PER_SECOND;
use clowder_core::{ArchId, CraftId, DreamLoomId, IdeaId, RefinementId, RiteId, RuleId, SiteId};
use clowder_tui::nav::rows::{Row, RowKind};

/// Whitespace collapsed, so a sentence wrapped across the pane reads whole.
fn flat(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A drawing with every number — sign included — replaced by one placeholder,
/// and whitespace collapsed.
fn masked(s: &str) -> String {
    let mut out = String::new();
    let mut run = String::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let starts_number = c.is_ascii_digit()
            || ((c == '-' || c == '+' || c == '.')
                && chars.get(i + 1).is_some_and(|n| n.is_ascii_digit()));
        if starts_number || (!run.is_empty() && (c.is_ascii_digit() || c == '.')) {
            run.push(c);
        } else {
            if !run.is_empty() {
                out.push('#');
                if run.ends_with('.') {
                    out.push('.');
                }
                run.clear();
            }
            out.push(c);
        }
        i += 1;
    }
    if !run.is_empty() {
        out.push('#');
    }
    flat(&out)
}

/// A figure as `numbers` reads it: the exact form, sign dropped.
fn bare(x: f64) -> String {
    fmt::exact(x).trim_start_matches('-').to_string()
}

fn per_second(per_tick: f64) -> String {
    bare(per_tick * TICKS_PER_SECOND as f64)
}

fn percent(ratio: f64) -> String {
    bare(ratio * 100.0)
}

/// The plain units of time a wait may be stated in, and their length in
/// seconds.
const TIME_UNITS: [(&str, f64); 18] = [
    ("s", 1.0),
    ("sec", 1.0),
    ("secs", 1.0),
    ("second", 1.0),
    ("seconds", 1.0),
    ("m", 60.0),
    ("min", 60.0),
    ("mins", 60.0),
    ("minute", 60.0),
    ("minutes", 60.0),
    ("h", 3_600.0),
    ("hr", 3_600.0),
    ("hrs", 3_600.0),
    ("hour", 3_600.0),
    ("hours", 3_600.0),
    ("d", 86_400.0),
    ("day", 86_400.0),
    ("days", 86_400.0),
];

fn unit_seconds(word: &str) -> Option<f64> {
    TIME_UNITS.iter().find(|(u, _)| *u == word).map(|(_, s)| *s)
}

/// One stretch of time stated on screen.
#[derive(Debug, Clone, Copy)]
struct Wait {
    /// What it says, in seconds.
    seconds: f64,
    /// The smallest unit it is stated in — how coarse its rounding can be.
    granularity: f64,
    /// Its byte range in the flattened drawing.
    start: usize,
    end: usize,
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Word(String),
    Other(char),
}

fn tokens(t: &str) -> Vec<(Tok, usize, usize)> {
    let mut out = Vec::new();
    let bytes: Vec<(usize, char)> = t.char_indices().collect();
    let mut i = 0;
    while i < bytes.len() {
        let (start, c) = bytes[i];
        if c.is_whitespace() {
            i += 1;
        } else if c.is_ascii_digit() {
            let mut j = i;
            while j < bytes.len() && (bytes[j].1.is_ascii_digit() || bytes[j].1 == '.') {
                j += 1;
            }
            // A trailing full stop ends the sentence, not the number.
            while j > i + 1 && bytes[j - 1].1 == '.' {
                j -= 1;
            }
            let end = bytes.get(j).map(|b| b.0).unwrap_or(t.len());
            let text = &t[start..end];
            match text.parse::<f64>() {
                Ok(n) => out.push((Tok::Num(n), start, end)),
                Err(_) => out.push((Tok::Other(c), start, end)),
            }
            i = j;
        } else if c.is_alphabetic() {
            let mut j = i;
            while j < bytes.len() && bytes[j].1.is_alphabetic() {
                j += 1;
            }
            let end = bytes.get(j).map(|b| b.0).unwrap_or(t.len());
            out.push((Tok::Word(t[start..end].to_lowercase()), start, end));
            i = j;
        } else {
            let end = bytes.get(i + 1).map(|b| b.0).unwrap_or(t.len());
            out.push((Tok::Other(c), start, end));
            i += 1;
        }
    }
    out
}

/// Every wait stated in a drawing: a number directly followed by a plain unit
/// of time, with any further number-and-unit parts that follow it (separated
/// by nothing, a comma, or "and") summed into the same wait.
fn waits(text: &str) -> Vec<Wait> {
    let t = flat(text);
    let toks = tokens(&t);
    let part = |k: usize| -> Option<(f64, f64, usize, usize)> {
        match (toks.get(k), toks.get(k + 1)) {
            (Some((Tok::Num(n), s, _)), Some((Tok::Word(w), _, e))) => {
                unit_seconds(w).map(|u| (n * u, u, *s, *e))
            }
            _ => None,
        }
    };
    let mut out = Vec::new();
    let mut k = 0;
    while k < toks.len() {
        let Some((secs, gran, start, mut end)) = part(k) else {
            k += 1;
            continue;
        };
        let mut w = Wait { seconds: secs, granularity: gran, start, end };
        k += 2;
        loop {
            let skip = match toks.get(k) {
                Some((Tok::Other(','), _, _)) => 1,
                Some((Tok::Word(a), _, _)) if a == "and" => 1,
                _ => 0,
            };
            match part(k + skip) {
                Some((s, g, _, e)) => {
                    w.seconds += s;
                    w.granularity = w.granularity.min(g);
                    end = e;
                    w.end = end;
                    k += skip + 2;
                }
                None => break,
            }
        }
        out.push(w);
    }
    out
}

/// The numbers of a drawing that are not part of a stated wait.
fn numbers_outside_waits(text: &str) -> BTreeSet<String> {
    let t = flat(text);
    let mut kept = String::new();
    let mut at = 0;
    for w in waits(text) {
        kept.push_str(&t[at..w.start]);
        kept.push(' ');
        at = w.end;
    }
    kept.push_str(&t[at..]);
    numbers(&kept)
}

/// The non-blank lines of a drawing, trimmed, in order.
fn lines_of(text: &str) -> Vec<String> {
    text.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect()
}

/// The one line of a drawing that states `figure`.
fn line_stating(text: &str, figure: &str) -> String {
    let found: Vec<String> =
        lines_of(text).into_iter().filter(|l| numbers(l).contains(figure)).collect();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one line stating {figure}; found {found:?} in:\n{text}"
    );
    flat(&found[0])
}

fn def_name(id: ContentId) -> Option<&'static str> {
    Some(match id {
        ContentId::Structure(s) => tables::structure_def(s).name,
        ContentId::Idea(i) => tables::idea_def(i).name,
        ContentId::Craft(c) => tables::craft_def(c).name,
        ContentId::Job(j) => tables::job_def(j).name,
        ContentId::Rite(r) => tables::rite_def(r).name,
        ContentId::Arch(a) => tables::arch_def(a).name,
        ContentId::Refinement(r) => tables::refinement_def(r).name,
        ContentId::Rule(r) => tables::rule_def(r).name,
        ContentId::DreamLoom(l) => tables::dream_loom_def(l).name,
        ContentId::Tradition(_) => return None,
    })
}

/// The figures a name itself carries, which may appear on screen with it.
fn name_numbers(state: &GameState, key: &RowKey) -> BTreeSet<String> {
    match key {
        RowKey::Content(id, _) => def_name(*id).map(numbers).unwrap_or_default(),
        RowKey::Kitten(ix, _) => numbers(&state.kittens[*ix].name),
        _ => BTreeSet::new(),
    }
}

/// A stock distinct for every resource and exactly representable, so each
/// "have" on screen is recognisably that resource's.
fn distinct_stock(r: ResourceId) -> f64 {
    1_000.5 + r.ix() as f64 * 0.25
}

fn named_kitten(i: usize) -> Kitten {
    let mut k = plain_kitten();
    k.name = format!("kitten-{i}");
    k
}

/// A meadow where every kind of row is listed: every structure and idea
/// revealed, curiosity storage built, every site reached, a generation passed,
/// devotion far past every rite's gate, and idle kittens at home. Every
/// structure is owned twice, so no structure's next price is its table price.
/// With `ideas_held`, every idea is researched and one item of each kind held
/// once is already the player's; without, the ideas are revealed but not yet
/// researched.
fn world(ideas_held: bool) -> GameState {
    let mut s = GameState::new(11);
    for r in ResourceId::ALL {
        s.resources[r.ix()] = distinct_stock(r);
    }
    for b in StructureId::ALL {
        s.structures[b.ix()] = 2;
    }
    s.revealed = StructureId::ALL
        .into_iter()
        .map(ContentId::Structure)
        .chain(IdeaId::ALL.into_iter().map(ContentId::Idea))
        .collect();
    s.sites_reached = SiteId::ALL.into_iter().collect();
    s.generation = 1;
    s.peak_devotion = 1.0e12;
    s.kittens = (0..6).map(named_kitten).collect();
    if ideas_held {
        s.ideas = IdeaId::ALL.into_iter().collect();
        s.arches_bought.insert(ArchId::ALL[0]);
        s.dream_looms.insert(DreamLoomId::ALL[0]);
        s.traditions.insert(TraditionId::ALL[0]);
        s.refinements.insert(RefinementId::ALL[0]);
        s.rules.insert(RuleId::ALL[0]);
        s.rite_counts[RiteId::ALL[0].ix()] = 1;
    }
    s
}

fn every_row(state: &GameState) -> Vec<(Section, Row)> {
    Section::ALL
        .into_iter()
        .flat_map(|section| {
            rows(state, section).into_iter().filter(|r| r.selectable()).map(move |r| (section, r))
        })
        .collect()
}

/// The next cost TUI-NAV-078 has the inspector state on this row, if any: the
/// core's price for one more, on a row that takes a content item, unless that
/// price names no component, the row is a craft's tinkerer count row, or the
/// core reports the item already the player's.
fn stated_next_cost(state: &GameState, key: &RowKey) -> Option<Vec<(ResourceId, f64)>> {
    let RowKey::Content(id, tag) = key else { return None };
    if *tag == "tinker" || state.lock_reason(*id) == Some(LockReason::AlreadyYours) {
        return None;
    }
    let cost = state.cost_of_next(*id, 1);
    (!cost.is_empty()).then_some(cost)
}

/// Every figure the next cost and the lock reason may put on this row.
fn cost_and_reason_numbers(state: &GameState, key: &RowKey) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    if let Some(cost) = stated_next_cost(state, key) {
        for (r, need) in cost {
            out.insert(bare(need));
            out.insert(bare(state.stock(r)));
        }
    }
    if let RowKey::Content(id, tag) = key {
        if *tag != "tinker" {
            if let Some(LockReason::ShortBy { missing }) = state.lock_reason(*id) {
                for (_, amount) in missing {
                    out.insert(bare(amount));
                }
            }
        }
    }
    out
}

/// Every field a structure's definition carries apart from its name, price,
/// escalation, reveal threshold, and gate, where it departs from its baseline —
/// each with the forms it may be stated in: per-tick fields per game second,
/// ratios as the percentage they add, everything else as it stands.
fn structure_fields(s: StructureId) -> Vec<(String, Vec<String>)> {
    let d = tables::structure_def(s);
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    for (r, a) in d.productions {
        out.push((format!("production of {r:?}"), vec![per_second(*a)]));
    }
    if d.housing != 0 {
        out.push(("housing".into(), vec![bare(d.housing as f64)]));
    }
    for (r, a) in d.cap_bonuses {
        out.push((format!("storage for {r:?}"), vec![bare(*a)]));
    }
    if d.happiness_points != 0.0 {
        out.push(("mood points".into(), vec![bare(d.happiness_points)]));
    }
    if d.crowding_relief != 0.0 {
        // A pooled fraction of crowding relieved: as it stands, or as a
        // percentage.
        out.push((
            "crowding relief".into(),
            vec![bare(d.crowding_relief), percent(d.crowding_relief)],
        ));
    }
    for (r, a) in d.resource_ratios {
        out.push((format!("ratio on {r:?}"), vec![percent(*a)]));
    }
    if d.craft_ratio != 0.0 {
        out.push(("craft ratio".into(), vec![percent(d.craft_ratio)]));
    }
    if d.luxury_demand != 0.0 {
        out.push(("luxury demand".into(), vec![percent(d.luxury_demand)]));
    }
    if d.jamboree_ratio != 0.0 {
        out.push(("jamboree ratio".into(), vec![percent(d.jamboree_ratio)]));
    }
    if d.jamboree_arrival != 0.0 {
        out.push(("jamboree arrival".into(), vec![percent(d.jamboree_arrival)]));
    }
    if d.global_ratio != 0.0 {
        out.push(("global ratio".into(), vec![percent(d.global_ratio)]));
    }
    if let Some(conv) = &d.converter {
        for (r, a) in conv.inputs {
            out.push((format!("converter input {r:?}"), vec![per_second(*a)]));
        }
        for (r, a) in conv.outputs {
            out.push((format!("converter output {r:?}"), vec![per_second(*a)]));
        }
    }
    if d.mess != 0.0 {
        out.push(("mess".into(), vec![per_second(d.mess)]));
    }
    if d.energy_prod != 0.0 {
        out.push(("energy made".into(), vec![bare(d.energy_prod)]));
    }
    if d.energy_cons != 0.0 {
        out.push(("energy used".into(), vec![bare(d.energy_cons)]));
    }
    if d.outdoor_ratio != 0.0 {
        out.push(("outdoor ratio".into(), vec![percent(d.outdoor_ratio)]));
    }
    if d.long_sun_cap != 0.0 {
        out.push(("long sun extension".into(), vec![bare(d.long_sun_cap)]));
    }
    out
}

/// Copies held of the structure under inspection in `structure_pane`.
const HELD: u32 = 3;

/// A structure's row with its gate open, `HELD` copies owned, and its next
/// price covered — so the drawing holds its name, statement, copies, and a
/// covered next cost, and no reason or wait.
fn structure_pane(s: StructureId) -> (GameState, String) {
    let mut state = meadow(&[s]);
    state.ideas = IdeaId::ALL.into_iter().collect();
    state.sites_reached = SiteId::ALL.into_iter().collect();
    state.structures[s.ix()] = HELD;
    for (r, need) in state.next_cost(s) {
        state.resources[r.ix()] = need + 0.5;
    }
    assert_eq!(state.lock_reason(ContentId::Structure(s)), None, "fixture: {s:?} is open and covered");
    assert_is_a_row(&state, Section::Meadow, &structure_row(s));
    let shown = pane_text(&state, &looking_at(Section::Meadow, structure_row(s)), WIDE, TALL);
    (state, shown)
}

/// The cursor's row is re-derived every frame from the row the list
/// highlights. When the remembered row has left its list, the list highlights
/// its first row, and the inspector must describe that row — not go blank.
/// @spec TUI-NAV-018
#[test]
fn the_inspector_describes_the_row_the_list_falls_back_to_when_the_focused_row_leaves() {
    let gone = producer();
    let state = meadow(&[]);
    let key = structure_row(gone);
    assert!(
        !rows(&state, Section::Meadow).iter().any(|r| r.key == key),
        "fixture: {gone:?} is not listed in a meadow that has revealed nothing"
    );
    // The row the list highlights: the first the cursor can rest on, a group
    // header above it being a signpost, not a row (TUI-NAV-099).
    let first = rows(&state, Section::Meadow)
        .into_iter()
        .find(|r| r.selectable())
        .expect("fixture: a fresh meadow lists something selectable");

    let fallen = pane_text(&state, &looking_at(Section::Meadow, key), WIDE, TALL);
    let on_first = pane_text(&state, &looking_at(Section::Meadow, first.key.clone()), WIDE, TALL);
    assert!(!on_first.trim().is_empty(), "fixture: the first row has something to say");
    assert_eq!(
        fallen, on_first,
        "the focused row left the list, so the list highlights {:?}; the inspector describes the \
         row the cursor is on and must describe that one",
        first.key
    );
}

/// The name, whole, on every row that has one — every piece of content with a
/// named definition, and every kitten — so any name the list cut with an
/// ellipsis can be read in full here.
/// @spec TUI-NAV-018
#[test]
fn every_row_names_its_item_whole() {
    let mut checked = 0;
    for state in [world(true), world(false)] {
        for (section, row) in every_row(&state) {
            let name = match &row.key {
                RowKey::Content(id, _) => match def_name(*id) {
                    Some(n) => n.to_string(),
                    None => continue,
                },
                RowKey::Kitten(ix, _) => state.kittens[*ix].name.clone(),
                _ => continue,
            };
            let shown = pane_text(&state, &looking_at(section, row.key.clone()), WIDE, TALL);
            assert!(
                flat(&shown).contains(&flat(&name)),
                "the inspector on {:?} must carry its whole name {name:?}; it read:\n{shown}",
                row.key
            );
            assert!(!shown.contains('\u{2026}'), "nothing in the pane may be cut; it read:\n{shown}");
            checked += 1;
        }
    }
    assert!(checked > 50, "fixture: the world lists many named rows ({checked})");
}

/// A structure's statement is every field its definition sets apart from its
/// name, price, escalation, reveal threshold, and gate — each wherever it
/// departs from its baseline, per-tick fields per game second, ratios as
/// percentages — and states nothing else: no table price beside the core's, no
/// escalation, no total for the copies owned, no baseline field. Quantified
/// over every structure the tables define.
/// @spec TUI-NAV-018
#[test]
fn a_structures_statement_is_every_field_its_definition_sets_and_nothing_else() {
    let mut failures = Vec::new();
    for s in StructureId::ALL {
        let (state, shown) = structure_pane(s);
        let key = structure_row(s);
        let on_screen = numbers_outside_waits(&shown);

        let fields = structure_fields(s);
        for (field, forms) in &fields {
            if !forms.iter().any(|f| on_screen.contains(f)) {
                failures.push(format!("{s:?}: its {field} ({forms:?}) is not stated"));
            }
        }

        let mut allowed: BTreeSet<String> = fields.iter().flat_map(|(_, f)| f.clone()).collect();
        allowed.extend(cost_and_reason_numbers(&state, &key));
        allowed.extend(name_numbers(&state, &key));
        allowed.insert(HELD.to_string());
        let stray: Vec<&String> = on_screen.difference(&allowed).collect();
        if !stray.is_empty() {
            failures.push(format!(
                "{s:?}: states {stray:?}, which is no field its definition sets, no figure of the \
                 core's next cost, and not the copies held"
            ));
        }
        if !failures.is_empty() && failures.len() < 4 {
            failures.push(format!("  ({s:?} read:\n{shown})"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The qualifiers that shape a structure's productions — seasonal, scaled by
/// the energy surplus, routed through the outdoor channel — are stated exactly
/// where the definition sets them. Wording-free: some word must appear on every
/// structure carrying the qualifier and on no structure without it.
/// @spec TUI-NAV-018
#[test]
fn a_structures_qualifiers_are_stated_exactly_where_its_definition_sets_them() {
    let panes: Vec<(StructureId, BTreeSet<String>)> = StructureId::ALL
        .into_iter()
        .map(|s| {
            let (_, shown) = structure_pane(s);
            let said: BTreeSet<String> = words(&shown)
                .difference(&words(tables::structure_def(s).name))
                .cloned()
                .collect::<BTreeSet<_>>()
                .difference(&resource_words())
                .cloned()
                .collect();
            (s, said)
        })
        .collect();

    type Qualifier = (&'static str, fn(StructureId) -> bool);
    let qualifiers: [Qualifier; 3] = [
        ("seasonal", |s| tables::structure_def(s).seasonal),
        ("scaled by the energy surplus", |s| tables::structure_def(s).surplus_scaled),
        ("routed through the outdoor channel", |s| tables::structure_def(s).outdoor),
    ];
    for (label, has) in qualifiers {
        let with: Vec<&BTreeSet<String>> = panes.iter().filter(|(s, _)| has(*s)).map(|(_, w)| w).collect();
        let without: BTreeSet<String> =
            panes.iter().filter(|(s, _)| !has(*s)).flat_map(|(_, w)| w.iter().cloned()).collect();
        assert!(!with.is_empty(), "fixture: some structure is {label}");
        let common: BTreeSet<String> = with
            .iter()
            .skip(1)
            .fold(with[0].clone(), |acc, w| acc.intersection(w).cloned().collect());
        let distinguishing: BTreeSet<&String> = common.difference(&without).collect();
        assert!(
            !distinguishing.is_empty(),
            "no word is said of every structure that is {label} and of none that is not; the \
             statement must carry that qualifier where the definition sets it"
        );
    }
}

/// A job's statement is its yield per worker per game second, and its count is
/// the kittens in the meadow holding it — a kitten away keeps the job but is not
/// counted. Nothing is worked out beside them: no total for the workers, no
/// yield after mood, no next cost (a job's price names nothing), and no
/// stand-in sentence — every line after the name carries a figure. Quantified
/// over every job.
/// @spec TUI-NAV-018
#[test]
fn a_jobs_row_states_its_yield_per_worker_per_second_and_the_workers_in_the_meadow() {
    const HOME: usize = 3;
    const AWAY: usize = 2;
    for j in JobId::ALL {
        let mut state = world(true);
        let mut roster = Vec::new();
        for i in 0..HOME {
            let mut k = named_kitten(i);
            k.job = Some(j);
            roster.push(k);
        }
        for i in 0..AWAY {
            let mut k = named_kitten(HOME + i);
            k.job = Some(j);
            k.location = Location::NeighborsYard;
            roster.push(k);
        }
        roster.push(named_kitten(HOME + AWAY));
        state.kittens = roster;

        let key = RowKey::Content(ContentId::Job(j), "");
        assert_is_a_row(&state, Section::Clowder, &key);
        assert_eq!(state.effective_workers(j), HOME as u32, "fixture: {HOME} of {j:?} are in the meadow");
        assert_eq!(state.lock_reason(ContentId::Job(j)), None, "fixture: a kitten is free");
        assert!(state.cost_of_next(ContentId::Job(j), 1).is_empty(), "fixture: a job's price names nothing");

        let shown = pane_text(&state, &looking_at(Section::Clowder, key.clone()), WIDE, TALL);
        let on_screen = numbers_outside_waits(&shown);

        let mut allowed = name_numbers(&state, &key);
        allowed.insert(HOME.to_string());
        assert!(
            on_screen.contains(&HOME.to_string()),
            "{j:?}: {HOME} kittens in the meadow hold this job and the inspector did not say so; \
             it read:\n{shown}"
        );
        assert!(
            !on_screen.contains(&(HOME + AWAY).to_string()),
            "{j:?}: {AWAY} of the job's kittens are away and must not be counted; it read:\n{shown}"
        );
        if let Some((r, per_tick)) = tables::job_def(j).yield_per_tick {
            let y = per_second(per_tick);
            assert!(
                on_screen.contains(&y),
                "{j:?}: the yield per worker per game second is {y}; it read:\n{shown}"
            );
            for w in words(&spaced(&format!("{r:?}"))) {
                assert!(words(&shown).contains(&w), "{j:?}: the yield names {r:?}; it read:\n{shown}");
            }
            allowed.insert(y);
        }
        let stray: Vec<&String> = on_screen.difference(&allowed).collect();
        assert!(
            stray.is_empty(),
            "{j:?}: states {stray:?} — no yield per worker, no count in the meadow; a total, an \
             after-mood yield, or a price is the core's to report. It read:\n{shown}"
        );

        let lines = lines_of(&shown);
        for line in lines.iter().skip(1) {
            assert!(
                line.chars().any(|c| c.is_ascii_digit()),
                "{j:?}: the line {line:?} carries no figure — a job's row states its name, its \
                 yield, and its count, and no stand-in sentence or empty next cost. It read:\n{shown}"
            );
        }
    }
}

/// A craft's rows state the resource the recipe makes and the work tier a
/// tinkerer meets it at. The make row prices one more; the tinkerer count row
/// binds rather than makes, so it states no next cost and no reason, and counts
/// the tinkerers in the meadow bound to this recipe — not those away, not those
/// bound elsewhere. Quantified over every recipe.
/// @spec TUI-NAV-018, TUI-NAV-078
#[test]
fn a_crafts_rows_state_what_it_makes_and_the_tinker_row_counts_bound_tinkerers_in_the_meadow() {
    const BOUND_HOME: usize = 7;
    const BOUND_AWAY: usize = 4;
    for c in CraftId::ALL {
        let other = CraftId::ALL.into_iter().find(|o| *o != c).expect("fixture: two recipes");
        let mut state = world(true);
        let mut roster = Vec::new();
        let tinkerer = |i: usize, bound: CraftId, away: bool| {
            let mut k = named_kitten(i);
            k.job = Some(JobId::Tinkerer);
            k.tinkering = Some(bound);
            if away {
                k.location = Location::NeighborsYard;
            }
            k
        };
        for i in 0..BOUND_HOME {
            roster.push(tinkerer(i, c, false));
        }
        for i in 0..BOUND_AWAY {
            roster.push(tinkerer(BOUND_HOME + i, c, true));
        }
        roster.push(tinkerer(BOUND_HOME + BOUND_AWAY, other, false));
        roster.push(tinkerer(BOUND_HOME + BOUND_AWAY + 1, other, false));
        roster.push(named_kitten(BOUND_HOME + BOUND_AWAY + 2));
        state.kittens = roster;

        let def = tables::craft_def(c);
        assert_ne!(def.tier as usize, BOUND_HOME, "fixture: the count must not read as the tier");
        let output_words = words(&spaced(&format!("{:?}", tables::craft_output(c))));
        let tier = (def.tier != 0).then(|| bare(def.tier as f64));

        let make = RowKey::Content(ContentId::Craft(c), "craft");
        let bind = RowKey::Content(ContentId::Craft(c), "tinker");
        assert_is_a_row(&state, Section::Crafts, &make);
        assert_is_a_row(&state, Section::Crafts, &bind);

        for key in [&make, &bind] {
            let shown = pane_text(&state, &looking_at(Section::Crafts, key.clone()), WIDE, TALL);
            let on_screen = numbers_outside_waits(&shown);
            for w in &output_words {
                assert!(
                    words(&shown).contains(w),
                    "{key:?}: the statement names the resource the recipe makes; it read:\n{shown}"
                );
            }
            let mut allowed = name_numbers(&state, key);
            if let Some(t) = &tier {
                assert!(on_screen.contains(t), "{key:?}: the work tier is {t}; it read:\n{shown}");
                allowed.insert(t.clone());
            }
            if key == &bind {
                allowed.insert(BOUND_HOME.to_string());
                assert!(
                    on_screen.contains(&BOUND_HOME.to_string()),
                    "{key:?}: {BOUND_HOME} tinkerers in the meadow are bound to this recipe; it \
                     read:\n{shown}"
                );
            } else {
                allowed.extend(cost_and_reason_numbers(&state, key));
            }
            let stray: Vec<&String> = on_screen.difference(&allowed).collect();
            assert!(
                stray.is_empty(),
                "{key:?} states {stray:?}; the make row states the recipe's tier and next cost, the \
                 tinkerer row its tier and the bound tinkerers in the meadow — no away kittens, no \
                 price, no reason. It read:\n{shown}"
            );
        }
    }
}

/// For a kind whose definition carries a blurb — an idea, a refinement, a rule
/// — the blurb is the statement, and none of the effect fields beside it is
/// restated. An item held once states no count of copies. Quantified over every
/// such row, researched and not.
/// @spec TUI-NAV-018
#[test]
fn a_blurb_kinds_statement_is_its_blurb_and_nothing_restated_beside_it() {
    let mut seen = 0;
    for state in [world(true), world(false)] {
        for (section, row) in every_row(&state) {
            let RowKey::Content(id, _) = &row.key else { continue };
            let blurb = match id {
                ContentId::Idea(i) => tables::idea_def(*i).blurb,
                ContentId::Refinement(r) => tables::refinement_def(*r).blurb,
                ContentId::Rule(r) => tables::rule_def(*r).blurb,
                _ => continue,
            };
            seen += 1;
            let shown = pane_text(&state, &looking_at(section, row.key.clone()), WIDE, TALL);
            if !blurb.is_empty() {
                assert!(
                    flat(&shown).contains(&flat(blurb)),
                    "{id:?}: the statement is the definition's own blurb; it read:\n{shown}"
                );
            }
            let mut allowed = numbers(blurb);
            allowed.extend(cost_and_reason_numbers(&state, &row.key));
            allowed.extend(name_numbers(&state, &row.key));
            let stray: Vec<String> =
                numbers_outside_waits(&shown).difference(&allowed).cloned().collect();
            assert!(
                stray.is_empty(),
                "{id:?} states {stray:?}, which is in neither its blurb nor the core's next cost or \
                 reason — an effect restated, a count of an item held once, or a price where the \
                 item is already yours. It read:\n{shown}"
            );
        }
    }
    assert!(seen > 20, "fixture: many blurb rows are listed ({seen})");
}

/// A rite's, an arch's, a tradition's, and a dream loom's definitions carry
/// nothing past name, price, and gate, so their rows have no statement — and no
/// stand-in sentence for one. Wording-free: a line the row shows both when the
/// item is already the player's and when it is open to take is neither the
/// next cost (stated only when open) nor the reason (stated only when held), so
/// beyond the name no such line may exist. A rite counts its copies; the
/// others, held once, count nothing. Quantified over every item of the four
/// kinds.
/// @spec TUI-NAV-018
#[test]
fn rows_whose_definition_states_nothing_carry_no_statement_and_no_stand_in() {
    let mut checked = 0;
    let cover = |state: &mut GameState, id: ContentId| {
        for (r, need) in state.cost_of_next(id, 1) {
            state.resources[r.ix()] = need + 0.5;
        }
    };
    let mut cases: Vec<(Section, ContentId, GameState, GameState, Option<&str>)> = Vec::new();
    for r in RiteId::ALL {
        let id = ContentId::Rite(r);
        let mut open = GameState::new(11);
        open.peak_devotion = 1.0e12;
        cover(&mut open, id);
        let mut held = open.clone();
        held.rite_counts[r.ix()] = 1;
        cases.push((Section::Sunbeams, id, held, open, Some("copies")));
    }
    for a in ArchId::ALL {
        let id = ContentId::Arch(a);
        let mut open = GameState::new(11);
        open.ideas.insert(IdeaId::ParadeRoutes);
        cover(&mut open, id);
        let mut held = open.clone();
        held.arches_bought.insert(a);
        cases.push((Section::Wonders, id, held, open, None));
    }
    for t in TraditionId::ALL {
        let id = ContentId::Tradition(t);
        let mut open = GameState::new(11);
        open.generation = 1;
        open.elder_wisdom = 1.0e9;
        let mut held = open.clone();
        held.traditions.insert(t);
        cases.push((Section::Legacy, id, held, open, None));
    }
    for l in DreamLoomId::ALL {
        let id = ContentId::DreamLoom(l);
        let mut open = GameState::new(11);
        open.ideas.insert(IdeaId::DeepNapTheory);
        cover(&mut open, id);
        let mut held = open.clone();
        held.dream_looms.insert(l);
        cases.push((Section::Dreams, id, held, open, None));
    }

    for (section, id, held, open, counts) in cases {
        let key = RowKey::Content(id, "");
        assert_is_a_row(&held, section, &key);
        assert_is_a_row(&open, section, &key);
        assert_eq!(held.lock_reason(id), Some(LockReason::AlreadyYours), "fixture: {id:?} is held");
        assert_eq!(open.lock_reason(id), None, "fixture: {id:?} is open to take");

        let view = looking_at(section, key.clone());
        let a = pane_text(&held, &view, WIDE, TALL);
        let b = pane_text(&open, &view, WIDE, TALL);
        let la = lines_of(&a);
        let lb = lines_of(&b);
        let common: Vec<&String> = la.iter().skip(1).filter(|l| lb.iter().skip(1).any(|m| m == *l)).collect();
        assert!(
            common.is_empty(),
            "{id:?}: {common:?} is said whether the item is held or open — neither a next cost nor \
             a reason, so a statement its definition does not carry.\nheld:\n{a}\nopen:\n{b}"
        );

        let mut allowed = name_numbers(&held, &key);
        if counts.is_some() {
            allowed.insert("1".to_string());
        }
        let held_numbers = numbers_outside_waits(&a);
        let stray: Vec<&String> = held_numbers.difference(&allowed).collect();
        assert!(
            stray.is_empty(),
            "{id:?} held states {stray:?} — no next cost for what is already yours, and a count of \
             copies only for a rite. It read:\n{a}"
        );
        checked += 1;
    }
    assert!(checked > 20, "fixture: every rite, arch, tradition, and loom ({checked})");
}

/// A row whose action has no content definition — a fixed action, a kitten's
/// rows, a site, an adventure, a pen pal — has nothing to state beyond its name:
/// no statement, no stand-in, no price, no reason — save, on a row the list
/// draws with a label in its name's place, that label (TUI-NAV-091). (Pounce
/// and Trade rows state the core's one-unit price instead, TUI-NAV-073.)
/// Quantified over every such row a full meadow lists.
/// @spec TUI-NAV-018, TUI-NAV-091
#[test]
fn a_row_with_no_definition_states_only_its_name() {
    let mut checked = 0;
    for state in [world(true), world(false)] {
        for (section, row) in every_row(&state) {
            if matches!(row.key, RowKey::Content(..) | RowKey::Resource(_)) {
                continue;
            }
            if let RowKind::Act { command, .. } = &row.kind {
                if matches!(command, clowder_core::Command::Pounce(_) | clowder_core::Command::Trade(..)) {
                    continue;
                }
            }
            let shown = pane_text(&state, &looking_at(section, row.key.clone()), WIDE, TALL);
            let lines = lines_of(&shown);
            let label = is_later_row(&state, section, &row.key)
                .then(|| later_row_label(&row.key).expect("fixture: a later row has a label"));
            assert_eq!(
                lines.len(),
                1 + usize::from(label.is_some()),
                "{:?} has no definition to state; the inspector names it, states the row's label \
                 {label:?} where the list draws one, and says nothing else. It read:\n{shown}",
                row.key
            );
            checked += 1;
        }
    }
    assert!(checked > 10, "fixture: a full meadow lists many rows with no definition ({checked})");
}

/// The copies held of a rite are stated — a rite is held in copies, like a
/// structure (which `the_owned_count_is_on_screen` pins).
/// @spec TUI-NAV-018
#[test]
fn the_copies_held_of_a_rite_are_on_screen() {
    const COPIES: u32 = 13;
    let r = RiteId::ALL
        .into_iter()
        .find(|r| tables::rite_restackable(*r))
        .expect("fixture: some rite restacks");
    let mut state = GameState::new(11);
    state.peak_devotion = 1.0e12;
    state.rites.insert(RiteId::Enlightenment);
    state.rite_counts[r.ix()] = COPIES;
    let key = RowKey::Content(ContentId::Rite(r), "");
    assert_is_a_row(&state, Section::Sunbeams, &key);
    let shown = pane_text(&state, &looking_at(Section::Sunbeams, key.clone()), WIDE, TALL);
    assert!(
        numbers_outside_waits(&shown).contains(&COPIES.to_string()),
        "{COPIES} copies of this rite are held and the inspector did not say so; it read:\n{shown}"
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-078 — the next cost and the wait
// ---------------------------------------------------------------------------

/// On every row that takes a content item and whose price names a component,
/// the next cost is the core's price for one more, each component with need and
/// have. Quantified over every such row of a full meadow, ideas researched and
/// not.
/// @spec TUI-NAV-078
#[test]
fn every_row_that_takes_an_item_states_the_cores_price_for_one_more_with_need_and_have() {
    let mut kinds = BTreeSet::new();
    for state in [world(true), world(false)] {
        for (section, row) in every_row(&state) {
            let Some(cost) = stated_next_cost(&state, &row.key) else { continue };
            let RowKey::Content(id, _) = &row.key else { unreachable!() };
            kinds.insert(format!("{id:?}").split('(').next().unwrap_or_default().to_string());
            let shown = pane_text(&state, &looking_at(section, row.key.clone()), WIDE, TALL);
            let on_screen = numbers(&shown);
            for (r, need) in cost {
                assert!(
                    on_screen.contains(&bare(need)) && on_screen.contains(&bare(state.stock(r))),
                    "{:?}: the core prices one more at {} of {r:?}, with {} held; both must be \
                     stated. It read:\n{shown}",
                    row.key,
                    bare(need),
                    bare(state.stock(r))
                );
            }
        }
    }
    assert!(kinds.len() >= 7, "fixture: every priced kind is listed ({} kinds)", kinds.len());
}

/// No next cost where there is none to state: on an item the core reports
/// already the player's, on a craft's tinkerer count row, and where the core's
/// price names no component — a tradition, priced outside the resource
/// vocabulary, whose open row therefore states its name and nothing else.
/// @spec TUI-NAV-078
#[test]
fn no_next_cost_is_stated_for_what_is_yours_on_a_tinker_row_or_where_the_price_names_nothing() {
    let state = world(true);
    let mut yours = 0;
    let mut tinker = 0;
    for (section, row) in every_row(&state) {
        let RowKey::Content(id, tag) = &row.key else { continue };
        let is_yours = state.lock_reason(*id) == Some(LockReason::AlreadyYours);
        if !(is_yours || *tag == "tinker") {
            continue;
        }
        let cost = state.cost_of_next(*id, 1);
        if cost.is_empty() {
            continue;
        }
        if is_yours {
            yours += 1;
        } else {
            tinker += 1;
        }
        let shown = pane_text(&state, &looking_at(section, row.key.clone()), WIDE, TALL);
        let on_screen = numbers_outside_waits(&shown);
        let blurb = match id {
            ContentId::Idea(i) => numbers(tables::idea_def(*i).blurb),
            ContentId::Refinement(r) => numbers(tables::refinement_def(*r).blurb),
            ContentId::Rule(r) => numbers(tables::rule_def(*r).blurb),
            // A craft's tinker row states its own work tier (TUI-NAV-018), a
            // figure of the recipe's own definition rather than a leaked
            // price; exempted on the same footing as a blurb's own figures,
            // since some recipe's tier happens to equal one of its own
            // ingredient amounts.
            ContentId::Craft(c) => {
                let tier = tables::craft_def(*c).tier;
                if tier != 0 { [bare(tier as f64)].into_iter().collect() } else { BTreeSet::new() }
            }
            _ => BTreeSet::new(),
        };
        for (r, need) in cost {
            for figure in [bare(need), bare(state.stock(r))] {
                assert!(
                    !on_screen.contains(&figure) || blurb.contains(&figure),
                    "{:?} ({}) states {figure}, a figure of the next cost of {r:?}; there is no next \
                     cost to state here. It read:\n{shown}",
                    row.key,
                    if is_yours { "already yours" } else { "a tinkerer count row" }
                );
            }
        }
    }
    assert!(yours > 3 && tinker > 3, "fixture: held items ({yours}) and tinker rows ({tinker}) are listed");

    let t = TraditionId::ALL[0];
    let mut open = GameState::new(11);
    open.generation = 1;
    open.elder_wisdom = 1.0e9;
    let key = RowKey::Content(ContentId::Tradition(t), "");
    assert!(open.cost_of_next(ContentId::Tradition(t), 1).is_empty(), "fixture: the core names no component");
    assert_eq!(open.lock_reason(ContentId::Tradition(t)), None, "fixture: open to take");
    let shown = pane_text(&open, &looking_at(Section::Legacy, key), WIDE, TALL);
    assert_eq!(
        lines_of(&shown).len(),
        1,
        "the core's price names no component, so no next cost is stated — not even a heading or a \
         \"costs nothing\", which would be false for a price outside the vocabulary. It read:\n{shown}"
    );
}

/// A structure with several components, its price covered with room to spare.
fn covered(s: StructureId) -> GameState {
    let mut state = meadow(&[s]);
    for (r, need) in state.next_cost(s) {
        state.resources[r.ix()] = need + 100.5;
    }
    state
}

/// The core's gate accepts a stock a hair under the need — its tolerance — and
/// reports no reason. The marks come from that report, so the component is
/// covered: the drawing must say the same things as with the price covered
/// outright. A shell comparing stock to need would mark it short and give it a
/// wait `Enter` would contradict.
/// @spec TUI-NAV-078
#[test]
fn a_component_the_core_accepts_a_hair_under_its_need_is_marked_covered() {
    let s = multi_component_structure();
    let view = looking_at(Section::Meadow, structure_row(s));
    let plain = covered(s);
    let (r0, need0) = plain.next_cost(s)[0];
    let mut hair = plain.clone();
    hair.resources[r0.ix()] = need0 - 1.0e-10;
    assert!(hair.stock(r0) < need0, "fixture: the stock is under the need");
    assert_eq!(hair.lock_reason(ContentId::Structure(s)), None, "fixture: the core's gate accepts it");

    let a = pane_text(&plain, &view, WIDE, TALL);
    let b = pane_text(&hair, &view, WIDE, TALL);
    assert_eq!(
        masked(&a),
        masked(&b),
        "the core reports no reason, so every component is covered; the drawings must differ only \
         in their figures.\ncovered:\n{a}\na hair under:\n{b}"
    );
    assert!(waits(&b).is_empty(), "no wait stands beside a covered component; it read:\n{b}");
}

/// Under a shortfall, each component it names is marked short by the amount it
/// names, and each it leaves out is marked covered — exactly as it would be with
/// no shortfall at all.
/// @spec TUI-NAV-078
#[test]
fn a_shortfall_marks_short_exactly_the_components_it_names_by_the_amounts_it_names() {
    let s = multi_component_structure();
    let view = looking_at(Section::Meadow, structure_row(s));
    let plain = covered(s);
    let cost = plain.next_cost(s);
    let (r0, need0) = cost[0];
    let (r1, _) = cost[1];
    let mut short = plain.clone();
    short.resources[r0.ix()] = need0 * 0.25;
    let missing = match short.lock_reason(ContentId::Structure(s)) {
        Some(LockReason::ShortBy { missing }) => missing,
        other => panic!("fixture: the core reports a shortfall, not {other:?}"),
    };
    assert_eq!(missing.len(), 1, "fixture: the shortfall names only {r0:?}");
    let (named, amount) = missing[0];
    assert_eq!(named, r0);

    let a = pane_text(&plain, &view, WIDE, TALL);
    let b = pane_text(&short, &view, WIDE, TALL);

    let have1 = bare(plain.stock(r1));
    assert_eq!(
        line_stating(&b, &have1),
        line_stating(&a, &have1),
        "{r1:?} is left out of the shortfall, so it is covered, and its line must read as it does \
         with nothing short.\nshort:\n{b}\ncovered:\n{a}"
    );
    let short_line = line_stating(&b, &bare(short.stock(r0)));
    assert!(
        numbers(&short_line).contains(&bare(amount)),
        "{r0:?} is short by {}, and its component line must say by how much; it read {short_line:?}",
        bare(amount)
    );
    assert_ne!(
        masked(&short_line),
        masked(&line_stating(&a, &bare(plain.stock(r0)))),
        "{r0:?} short and {r0:?} covered must be marked differently"
    );
}

/// Where a reason other than a shortfall holds the item back, need and have
/// stand unmarked: the drawing is the same whether the stock covers the price or
/// not, and a component's line differs from the covered mark the same
/// component wears once the reason is gone.
/// @spec TUI-NAV-078
#[test]
fn under_a_reason_other_than_a_shortfall_components_stand_unmarked() {
    let (g, idea) = StructureId::ALL
        .into_iter()
        .find_map(|s| match structure_def(s).gate {
            StructureGate::Idea(i) if !structure_def(s).base_costs.is_empty() => Some((s, i)),
            _ => None,
        })
        .expect("fixture: some structure is gated on an idea");
    let view = looking_at(Section::Meadow, structure_row(g));

    let gated_covered = covered(g);
    let mut gated_short = gated_covered.clone();
    for (r, need) in gated_short.next_cost(g) {
        gated_short.resources[r.ix()] = need * 0.25;
    }
    let mut open_covered = gated_covered.clone();
    open_covered.ideas.insert(idea);
    for st in [&gated_covered, &gated_short] {
        assert_eq!(
            st.lock_reason(ContentId::Structure(g)),
            Some(LockReason::NotYetReached),
            "fixture: the gate holds {g:?} back whatever the stock"
        );
        assert_is_a_row(st, Section::Meadow, &structure_row(g));
    }
    assert_eq!(open_covered.lock_reason(ContentId::Structure(g)), None, "fixture: gate open, covered");

    let a = pane_text(&gated_covered, &view, WIDE, TALL);
    let b = pane_text(&gated_short, &view, WIDE, TALL);
    assert_eq!(
        masked(&a),
        masked(&b),
        "another reason holds the item back, so stock marks nothing and no wait is stated.\n\
         covered:\n{a}\nshort:\n{b}"
    );
    assert!(waits(&b).is_empty(), "no wait under another reason; it read:\n{b}");

    let c = pane_text(&open_covered, &view, WIDE, TALL);
    let (r, _) = gated_covered.next_cost(g)[0];
    let have = bare(gated_covered.stock(r));
    assert_ne!(
        line_stating(&a, &have),
        line_stating(&c, &have),
        "with no reason reported the component is marked covered; under another reason it stands \
         unmarked, so the two lines must differ.\nunmarked:\n{a}\ncovered:\n{c}"
    );
}

/// A structure, a component of its price, and something else that makes that
/// component — so the component can be short with the core's net rate for it
/// positive and its need within the cap.
fn waiting_fixture() -> (StructureId, ResourceId, GameState) {
    for s in StructureId::ALL.into_iter().filter(|s| ungated(*s)) {
        let costs = meadow(&[s]).next_cost(s);
        for (r, need) in &costs {
            for p in StructureId::ALL {
                let pd = structure_def(p);
                if p == s || pd.converter.is_some() || !pd.productions.iter().any(|(pr, a)| pr == r && *a > 0.0) {
                    continue;
                }
                let mut state = meadow(&[s]);
                state.structures[p.ix()] = 6;
                for (o, n) in &costs {
                    state.resources[o.ix()] = if o == r { 0.0 } else { n + 100.5 };
                }
                if state.net_rate(*r) > 0.0 && *need <= state.cap(*r) {
                    return (s, *r, state);
                }
            }
        }
    }
    panic!("fixture: some structure's component is made by another structure, within its cap");
}

/// Beside a component the core reports short stands a wait: the shortfall over
/// the core's net rate and the current speed — real time — in plain units of
/// time, never rounded to sooner than the rate gives. Checked at every notch
/// from the slowest speed to the fastest.
/// @spec TUI-NAV-078
#[test]
fn a_short_components_wait_is_its_shortfall_over_the_rate_and_the_speed_never_sooner() {
    let (s, r, base) = waiting_fixture();
    let view = looking_at(Section::Meadow, structure_row(s));
    let shortfall = match base.lock_reason(ContentId::Structure(s)) {
        Some(LockReason::ShortBy { missing }) => {
            assert_eq!(missing.len(), 1, "fixture: only {r:?} is short");
            missing[0].1
        }
        other => panic!("fixture: the core reports a shortfall, not {other:?}"),
    };
    let rate = base.net_rate(r);
    for speed in [0.25, 0.5, 1.0, 2.0, 4.0, 8.0] {
        let mut state = base.clone();
        state.speed = speed;
        let shown = pane_text(&state, &view, WIDE, TALL);
        let found = waits(&shown);
        assert_eq!(
            found.len(),
            1,
            "at speed {speed}, one component is short and growing, so one wait is stated, in plain \
             units of time; found {found:?} in:\n{shown}"
        );
        let expected = shortfall / (rate * speed);
        let w = found[0];
        assert!(
            w.seconds >= expected * (1.0 - 1e-9),
            "at speed {speed} the wait is {expected}s of real time; the inspector said {}s, which is \
             sooner than the rate gives. It read:\n{shown}",
            w.seconds
        );
        assert!(
            w.seconds < expected + w.granularity,
            "at speed {speed} the wait is {expected}s of real time; the inspector said {}s, more \
             than one unit of its own rounding later. It read:\n{shown}",
            w.seconds
        );
    }
}

/// Where the need exceeds the cap the core reports, the component says the
/// stores cannot hold that much yet, with no wait — alone, even where the stock
/// is not growing either. Otherwise, where the rate is zero or less, it says the
/// stock is not growing, with no wait. Wording-free: no wait is found in either
/// case, the cap note reads the same whether or not the stock grows, and it
/// reads differently from the not-growing note.
/// @spec TUI-NAV-078
#[test]
fn a_need_above_the_cap_or_a_stock_not_growing_gets_a_note_in_place_of_a_wait() {
    // A structure costing a capped resource it does not itself touch, the
    // copies that lift that component above the cap, a store that lifts the cap
    // back over it, and a maker of that resource.
    let mut found = None;
    'search: for s in StructureId::ALL.into_iter().filter(|s| ungated(*s)) {
        let sd = structure_def(s);
        if sd.converter.is_some() {
            continue;
        }
        for (r, _) in sd.base_costs {
            let base_cap = GameState::new(11).cap(*r);
            if !(base_cap > 0.0 && base_cap.is_finite()) || sd.productions.iter().any(|(p, _)| p == r) {
                continue;
            }
            let Some(q) = StructureId::ALL.into_iter().find(|q| {
                let d = structure_def(*q);
                *q != s
                    && d.cap_bonuses.iter().any(|(c, a)| c == r && *a > 0.0)
                    && d.productions.is_empty()
                    && d.converter.is_none()
                    && d.energy_cons == 0.0
            }) else {
                continue;
            };
            let Some(p) = StructureId::ALL.into_iter().find(|p| {
                let d = structure_def(*p);
                *p != s && d.converter.is_none() && d.productions.iter().any(|(c, a)| c == r && *a > 0.0)
            }) else {
                continue;
            };
            for owned in 1..200u32 {
                let mut st = meadow(&[s]);
                st.structures[s.ix()] = owned;
                let need = st.next_cost(s).iter().find(|(c, _)| c == r).map(|(_, n)| *n).unwrap();
                if need > st.cap(*r) {
                    found = Some((s, *r, owned, q, p));
                    break 'search;
                }
            }
        }
    }
    let (s, r, owned, q, p) = found.expect("fixture: some capped component can outgrow its cap");

    let setup = |store: u32, maker: u32| {
        let mut st = meadow(&[s]);
        st.structures[s.ix()] = owned;
        st.structures[q.ix()] = store;
        st.structures[p.ix()] = maker;
        for (c, n) in st.next_cost(s) {
            st.resources[c.ix()] = if c == r { 0.0 } else { n + 100.5 };
        }
        st
    };
    let need = setup(0, 0).next_cost(s).iter().find(|(c, _)| *c == r).unwrap().1;
    let store = (1..10_000u32)
        .find(|m| setup(*m, 0).cap(r) >= need)
        .expect("fixture: enough stores hold the need");

    let above_growing = setup(0, 6);
    let above_still = setup(0, 0);
    let within_still = setup(store, 0);
    assert!(need > above_growing.cap(r) && above_growing.net_rate(r) > 0.0, "fixture: above the cap, growing");
    assert!(need > above_still.cap(r) && above_still.net_rate(r) <= 0.0, "fixture: above the cap, not growing");
    assert!(need <= within_still.cap(r) && within_still.net_rate(r) <= 0.0, "fixture: within the cap, not growing");

    let view = looking_at(Section::Meadow, structure_row(s));
    let a = pane_text(&above_growing, &view, WIDE, TALL);
    let b = pane_text(&above_still, &view, WIDE, TALL);
    let c = pane_text(&within_still, &view, WIDE, TALL);
    for (label, shown) in [("above the cap, growing", &a), ("above the cap, not growing", &b), ("not growing", &c)] {
        assert!(waits(shown).is_empty(), "{label}: no wait is stated; it read:\n{shown}");
    }
    let mut fast = above_growing.clone();
    fast.speed = 8.0;
    assert_eq!(
        a,
        pane_text(&fast, &view, WIDE, TALL),
        "above the cap no wait is stated, so the speed changes nothing on the row"
    );
    assert_eq!(
        masked(&a),
        masked(&b),
        "above the cap the stores note stands alone, growing or not.\ngrowing:\n{a}\nnot growing:\n{b}"
    );
    assert_ne!(
        masked(&b),
        masked(&c),
        "the stores note and the not-growing note must read differently.\nabove the cap:\n{b}\n\
         within it:\n{c}"
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-080 — a stores row
// ---------------------------------------------------------------------------

/// The stock, exact; the cap the core reports, or that there is none; and the
/// core's net rate per game second — not per real second, so the game speed
/// (set away from 1 here) changes nothing. Quantified over every resource,
/// each held so that its stores row is listed.
/// @spec TUI-NAV-080
#[test]
fn a_stores_row_states_its_exact_stock_its_cap_or_none_and_its_net_rate_per_game_second() {
    let mut state = world(true);
    state.speed = 4.0;
    let mut finite_body: Option<(ResourceId, Vec<String>)> = None;
    let mut infinite: Vec<(ResourceId, Vec<String>, String)> = Vec::new();
    for r in ResourceId::ALL {
        let key = RowKey::Resource(r);
        assert_is_a_row(&state, Section::Meadow, &key);
        let shown = pane_text(&state, &looking_at(Section::Meadow, key.clone()), WIDE, TALL);
        let on_screen = numbers(&shown);
        let stock = bare(state.stock(r));
        let rate = bare(state.net_rate(r));
        let cap = state.cap(r);
        assert!(on_screen.contains(&stock), "{r:?}: the exact stock {stock}; it read:\n{shown}");
        assert!(on_screen.contains(&rate), "{r:?}: the net rate per game second {rate}; it read:\n{shown}");
        let mut allowed: BTreeSet<String> = [stock, rate].into_iter().collect();
        let body: Vec<String> = lines_of(&shown).into_iter().skip(1).map(|l| masked(&l)).collect();
        if cap.is_finite() {
            let c = bare(cap);
            assert!(on_screen.contains(&c), "{r:?}: the cap the core reports, {c}; it read:\n{shown}");
            allowed.insert(c);
            finite_body.get_or_insert((r, body));
        } else {
            assert!(
                !words(&shown).contains("inf"),
                "{r:?} has no cap, which is said in words rather than as a figure; it read:\n{shown}"
            );
            infinite.push((r, body, shown.clone()));
        }
        let stray: Vec<&String> = on_screen.difference(&allowed).collect();
        assert!(
            stray.is_empty(),
            "{r:?}: a stores row states its stock, cap, and net rate per game second; {stray:?} is \
             none of them. It read:\n{shown}"
        );
    }
    let (fr, fbody) = finite_body.expect("fixture: some resource is capped");
    assert!(!infinite.is_empty(), "fixture: some resource is uncapped");
    for (r, body, shown) in infinite {
        assert_eq!(
            body.len(),
            fbody.len(),
            "{r:?} has no cap and must say so where {fr:?} states its cap — the same facts, one \
             worded differently. It read:\n{shown}"
        );
        assert_ne!(body, fbody, "{r:?}: saying there is no cap must not read as a cap; it read:\n{shown}");
    }
}

// ---------------------------------------------------------------------------
// TUI-RENDER-012 — the inspector's figures are exact, and hold every figure
// the sidebar and the list abbreviate
// ---------------------------------------------------------------------------

/// Every figure on screen with the sign glued before it, if any: a maximal
/// run of digits and dots, with a `+` or `-` directly before it kept.
fn signed_numbers(s: &str) -> Vec<String> {
    let chars: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_digit() {
            let start = if i > 0 && (chars[i - 1] == '+' || chars[i - 1] == '-') { i - 1 } else { i };
            let mut j = i;
            while j < chars.len() && (chars[j].is_ascii_digit() || chars[j] == '.') {
                j += 1;
            }
            out.push(chars[start..j].iter().collect::<String>().trim_end_matches('.').to_string());
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

/// A meadow like `world(true)` with every store holding a ragged, noisy
/// stock, so an exact figure has digits to round and a compact one would
/// abbreviate.
fn ragged_world() -> GameState {
    let mut s = world(true);
    for r in ResourceId::ALL {
        s.resources[r.ix()] = 1234.5678912 * (1.0 + r.ix() as f64 * 0.37) + 0.1 + 0.2;
    }
    s
}

/// On a stores row, the store's net rate is drawn in the exact form of its
/// size behind `+` where it is above zero and `-` where it is below, and with
/// no sign where it is zero — the one signed figure the row states.
/// Quantified over every resource, in meadows where stores gain, drain, and
/// stand still.
/// @spec TUI-RENDER-012
#[test]
fn a_stores_rows_net_rate_is_its_exact_size_behind_its_sign() {
    let mut classes = [0usize; 3];
    for state in [world(true), world(false), GameState::new(11)] {
        let mut state = state;
        for r in ResourceId::ALL {
            state.resources[r.ix()] = state.resources[r.ix()].max(1.0);
        }
        let rates = state.net_rates();
        for r in ResourceId::ALL {
            let key = RowKey::Resource(r);
            assert_is_a_row(&state, Section::Meadow, &key);
            let shown = pane_text(&state, &looking_at(Section::Meadow, key), WIDE, TALL);
            let signed: Vec<String> =
                signed_numbers(&shown).into_iter().filter(|t| t.starts_with('+') || t.starts_with('-')).collect();
            let rate = rates[r.ix()];
            let size = fmt::exact(rate.abs());
            if size == "0" {
                classes[0] += 1;
                assert!(
                    signed.is_empty(),
                    "{r:?} stands still (rate {rate:e}); its rate is stated with no sign, and it read \
                     {signed:?}:\n{shown}"
                );
            } else {
                let expected = format!("{}{size}", if rate > 0.0 { '+' } else { '-' });
                classes[if rate > 0.0 { 1 } else { 2 }] += 1;
                assert_eq!(
                    signed,
                    vec![expected.clone()],
                    "{r:?}'s net rate {rate:e} is stated as {expected:?}, the exact form of its size \
                     behind its sign, and is the row's one signed figure:\n{shown}"
                );
            }
        }
    }
    assert!(classes.iter().all(|n| *n > 0), "fixture: stores still, gaining, and draining: {classes:?}");
}

/// Every figure the inspector states is in the exact form: no magnitude
/// marker, no exponent, no grouping, no trailing zero, and no more fraction
/// than three places or the fourth significant figure allows — over every
/// row of every section of a meadow whose stocks carry binary noise. A stock
/// that is not finite is said in words as beyond reach.
/// @spec TUI-RENDER-012, TUI-RENDER-018
#[test]
fn every_figure_the_inspector_states_is_in_the_exact_form() {
    let state = ragged_world();
    let mut checked = 0;
    for (section, row) in every_row(&state) {
        let shown = pane_text(&state, &looking_at(section, row.key.clone()), WIDEST, TALL);
        let body: String = lines_of(&shown).into_iter().skip(1).collect::<Vec<_>>().join("\n");
        let chars: Vec<char> = format!("{body} ").chars().collect();
        for w in chars.windows(3) {
            let marker = w[0].is_ascii_digit() && "KMBTQ".contains(w[1]) && !w[2].is_alphanumeric();
            let exponent = w[0].is_ascii_digit() && w[1] == 'e' && (w[2].is_ascii_digit() || w[2] == '-' || w[2] == '+');
            let grouping = w[0].is_ascii_digit() && (w[1] == ',' || w[1] == '_') && w[2].is_ascii_digit();
            assert!(
                !(marker || exponent || grouping),
                "{:?}: {:?} glues a marker, exponent, or grouping to a figure; the inspector's \
                 figures are exact (TUI-RENDER-012):\n{shown}",
                row.key,
                w.iter().collect::<String>()
            );
        }
        assert!(!body.contains("<0.001") && !body.contains(">999Q"), "{:?}: a compact bound in the inspector:\n{shown}", row.key);
        for t in numbers(&body) {
            checked += 1;
            let v: f64 = t.parse().unwrap_or_else(|_| panic!("{:?}: {t:?} is not a figure", row.key));
            if let Some((_, frac)) = t.split_once('.') {
                assert!(!frac.ends_with('0'), "{:?}: {t:?} ends its fraction in a zero:\n{shown}", row.key);
                let places = 3.max(3 - v.log10().floor() as i32) as usize;
                assert!(
                    frac.len() <= places,
                    "{:?}: {t:?} carries more than {places} places — past both three places and \
                     its fourth significant figure (TUI-RENDER-018):\n{shown}",
                    row.key
                );
            }
        }
    }
    assert!(checked > 0, "fixture: figures were read");

    let mut endless = ragged_world();
    endless.resources[ResourceId::ALL[0].ix()] = f64::INFINITY;
    let shown = pane_text(&endless, &looking_at(Section::Meadow, RowKey::Resource(ResourceId::ALL[0])), WIDE, TALL);
    let w = words(&shown);
    assert!(
        flat(&shown).to_lowercase().contains("beyond reach") && !w.contains("inf") && !w.contains("infinity"),
        "a stock that is not finite is said in words as beyond reach, never printed \
         (TUI-RENDER-018):\n{shown}"
    );
}

/// Every figure the sidebar or a list row abbreviates has its exact form in
/// the inspector, a row's on that row and the sidebar's on its store's row in
/// the stores group: a row's owned count and each component of its next cost
/// (the one-unit price on a Pounce or Trade row), and a store's stock, its cap
/// where the core reports one, and its net rate. Quantified over every row of
/// every section and every store.
/// @spec TUI-RENDER-012
#[test]
fn every_abbreviated_figure_has_its_exact_form_on_its_rows_inspector() {
    let state = ragged_world();
    let rates = state.net_rates();
    for (section, row) in every_row(&state) {
        let shown = pane_text(&state, &looking_at(section, row.key.clone()), WIDEST, TALL);
        let on_screen = numbers(&shown);
        let mut owed: Vec<(String, f64)> = Vec::new();
        match &row.key {
            RowKey::Content(ContentId::Structure(b), _) => owed.push(("the owned count".into(), state.structures[b.ix()] as f64)),
            RowKey::Content(ContentId::Rite(r), _) => owed.push(("the owned count".into(), state.rite_counts[r.ix()] as f64)),
            RowKey::Resource(r) => {
                owed.push(("the stock".into(), state.stock(*r)));
                if state.cap(*r).is_finite() {
                    owed.push(("the cap".into(), state.cap(*r)));
                }
                owed.push(("the net rate".into(), rates[r.ix()]));
            }
            _ => {}
        }
        let cost = stated_next_cost(&state, &row.key).or_else(|| match (&row.key, &row.kind) {
            (RowKey::Content(..), _) => None,
            (_, RowKind::Act { command, .. }) => state.unit_price(command).filter(|c| !c.is_empty()),
            _ => None,
        });
        for (r, need) in cost.unwrap_or_default() {
            owed.push((format!("the {r:?} component of the next cost"), need));
        }
        for (what, v) in owed {
            let form = bare(v);
            assert!(
                on_screen.contains(&form),
                "{section:?} {:?}: {what} ({v:e}) is abbreviated on the row or the sidebar, so its \
                 exact form {form:?} is owed on this row's inspector (TUI-RENDER-012):\n{shown}",
                row.key
            );
        }
    }
}
