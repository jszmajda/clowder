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
    let mut term = terminal(w, h);
    term.draw(|f| inspector::draw(f, rect, state, view))
        .expect("TestBackend is infallible");
    term.backend().buffer().clone()
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
    let mut v = ViewState::default();
    v.section = Some(section);
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
