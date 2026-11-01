//! What a store's card opens with: the store's drawing in its coat and its
//! description in flavour's dim, both giving way before a figure or a flow
//! line would be lost, and never following a figure's digits.
//!
//! @spec TUI-NAV-197, TUI-RENDER-031, TUI-RENDER-032, TUI-RENDER-033,
//! TUI-THEME-031

use clowder_core::content;
use clowder_core::{GameState, ResourceId, StructureId};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::Terminal;

use clowder_tui::nav::{RowKey, Section, ViewState};
use clowder_tui::render::layout::{compute, Body};
use clowder_tui::render::store_art;
use clowder_tui::render::widgets::inspector;
use clowder_tui::theme::{coat, coat_colours, Role};

/// Wide enough that no line of any card wraps, measured or drawn.
const WIDE: u16 = 100;
/// Taller than any card.
const TALL: u16 = 70;
/// The inspector's narrowest width.
const NARROWEST: u16 = 20;

/// A meadow where every store is listed and structures move many of them.
fn meadow() -> GameState {
    let mut s = GameState::new(11);
    for r in ResourceId::ALL {
        s.resources[r.ix()] = 5.0;
    }
    for b in StructureId::ALL {
        s.structures[b.ix()] = 2;
    }
    s
}

/// A meadow where every store is listed and nothing moves any of them.
fn still_meadow() -> GameState {
    let mut s = GameState::new(11);
    for r in ResourceId::ALL {
        s.resources[r.ix()] = 5.0;
    }
    s
}

fn looking_at(r: ResourceId) -> ViewState {
    let mut view = ViewState {
        section: Some(Section::Meadow),
        ..ViewState::default()
    };
    view.focus[Section::Meadow.index()] = Some(RowKey::Resource(r));
    view
}

/// The inspector drawn into a pane of `w` by `h` for a store's row.
fn card(s: &GameState, r: ResourceId, w: u16, h: u16) -> Buffer {
    let rect = Rect::new(0, 0, w, h);
    let view = looking_at(r);
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("TestBackend is infallible");
    term.draw(|f| inspector::draw(f, rect, s, &view))
        .expect("TestBackend is infallible");
    term.backend().buffer().clone()
}

/// Each row as the player sees it, trailing blank rows dropped. A no-break
/// space, which the inspector uses where a blank must survive wrapping, draws
/// as a blank like any other.
fn text(buf: &Buffer) -> Vec<String> {
    let mut out: Vec<String> = (0..buf.area.height)
        .map(|y| {
            (0..buf.area.width)
                .map(|x| buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "))
                .map(|s| if s == "\u{a0}" { " " } else { s })
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect();
    while out.last().is_some_and(String::is_empty) {
        out.pop();
    }
    out
}

fn fg(buf: &Buffer, x: usize, y: usize) -> Color {
    buf.cell((x as u16, y as u16)).unwrap().fg
}

/// A resource's name as its stores row spells it.
fn label(r: ResourceId) -> String {
    let tag = format!("{r:?}");
    let mut out = String::new();
    for (i, ch) in tag.chars().enumerate() {
        if i > 0 && ch.is_uppercase() {
            out.push(' ');
        }
        out.push(ch);
    }
    out
}

/// The drawing's rows as a pane shows them.
fn drawn_rows(r: ResourceId) -> Vec<String> {
    store_art::drawing(r)
        .iter()
        .map(|row| row.trim_end().to_string())
        .collect()
}

/// The description line as a pane wide enough to hold it shows it.
fn caption(r: ResourceId) -> String {
    format!("\u{b7} {}", content::resource_description(r))
}

// ---------------------------------------------------------------------------
// TUI-NAV-197 — what the card opens with, and in what order
// ---------------------------------------------------------------------------

/// Name, drawing, description, one blank line, then the figures: on every
/// store's card, with nothing between them.
/// @spec TUI-NAV-197, TUI-RENDER-031
#[test]
fn a_store_card_opens_with_its_drawing_then_its_description_then_its_figures() {
    let s = still_meadow();
    for r in ResourceId::ALL {
        let lines = text(&card(&s, r, WIDE, TALL));
        let rows = drawn_rows(r);
        let n = rows.len();
        assert_eq!(
            lines[0],
            label(r),
            "{r:?}: the name comes first: {lines:#?}"
        );
        assert_eq!(
            lines[1..=n],
            rows[..],
            "{r:?}: the drawing stands beneath the name: {lines:#?}"
        );
        assert_eq!(
            lines[n + 1],
            caption(r),
            "{r:?}: the description stands beneath the drawing: {lines:#?}"
        );
        assert_eq!(lines[n + 2], "", "{r:?}: one blank line: {lines:#?}");
        assert!(
            lines[n + 3].starts_with("Stock: "),
            "{r:?}: then the figures: {lines:#?}"
        );
    }
}

/// The description is flavour: behind `·`, every cell in `dim`, however many
/// rows it wraps to.
/// @spec TUI-NAV-197
#[test]
fn a_store_description_wears_flavours_dim_on_every_row_it_wraps_to() {
    let dim = Role::Dim.color().unwrap();
    let s = still_meadow();
    for w in [WIDE, NARROWEST] {
        for r in ResourceId::ALL {
            let buf = card(&s, r, w, TALL);
            let lines = text(&buf);
            let first = drawn_rows(r).len() + 1;
            assert!(
                lines[first].starts_with("\u{b7} "),
                "{r:?} at {w}: the description opens with its mark: {lines:#?}"
            );
            let blank = (first..lines.len())
                .find(|&y| lines[y].is_empty())
                .expect("a blank line follows the description");
            for (y, line) in lines.iter().enumerate().take(blank).skip(first) {
                for (x, ch) in line.chars().enumerate() {
                    if ch != ' ' {
                        assert_eq!(fg(&buf, x, y), dim, "{r:?} at {w}, ({x},{y}): {line:?}");
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// TUI-RENDER-031 — the drawing
// ---------------------------------------------------------------------------

/// Small enough for the inspector's narrowest width, in single-width marks,
/// with something to paint, and no two stores alike.
/// @spec TUI-RENDER-031
#[test]
fn every_store_has_a_small_drawing_of_its_own() {
    for (i, r) in ResourceId::ALL.into_iter().enumerate() {
        let d = store_art::drawing(r);
        assert!(
            (1..=5).contains(&d.len()),
            "{r:?}: {} rows, at most five",
            d.len()
        );
        for row in d {
            assert!(
                row.chars().count() <= 16,
                "{r:?}: {row:?} is past sixteen columns"
            );
            for ch in row.chars() {
                assert!(
                    ch == '\u{2588}' || ch == ' ' || ch.is_ascii_graphic(),
                    "{r:?}: {ch:?} in {row:?} is neither printable ASCII nor a block"
                );
            }
        }
        assert!(
            d.iter().any(|row| row.contains('\u{2588}')),
            "{r:?}: nothing to paint"
        );
        for other in ResourceId::ALL.into_iter().skip(i + 1) {
            assert_ne!(
                d,
                store_art::drawing(other),
                "{r:?} and {other:?} share a drawing"
            );
        }
    }
}

/// At the inspector's narrowest, every row stands whole from the left edge,
/// its leading blanks kept.
/// @spec TUI-RENDER-031
#[test]
fn a_drawing_stands_whole_at_the_inspectors_narrowest() {
    let s = still_meadow();
    for r in ResourceId::ALL {
        let lines = text(&card(&s, r, NARROWEST, TALL));
        let rows = drawn_rows(r);
        assert_eq!(
            lines[1..=rows.len()],
            rows[..],
            "{r:?} at the narrowest width: {lines:#?}"
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-THEME-031 — the drawing's colour
// ---------------------------------------------------------------------------

/// Each painted cell takes its column's colour under the store's coat, laid
/// across the drawing's width; every other mark is the inherited foreground.
/// @spec TUI-THEME-031
#[test]
fn a_drawing_wears_the_stores_coat_by_column_and_its_outline_wears_none() {
    let s = still_meadow();
    for r in ResourceId::ALL {
        let buf = card(&s, r, WIDE, TALL);
        let d = store_art::drawing(r);
        let widest = d.iter().map(|row| row.chars().count()).max().unwrap();
        let colours = coat_colours(coat(r), widest);
        for (row, art) in d.iter().enumerate() {
            let y = row + 1;
            for (x, ch) in art.chars().enumerate() {
                let want = match ch {
                    '\u{2588}' => colours[x],
                    _ => Color::Reset,
                };
                assert_eq!(
                    fg(&buf, x, y),
                    want,
                    "{r:?}: {ch:?} at ({x},{y}) in {art:?}"
                );
            }
        }
    }
}

/// The picture is the store's own and no state's: the same at every stock.
/// @spec TUI-THEME-031, TUI-RENDER-031
#[test]
fn a_drawing_does_not_change_with_the_store() {
    let r = ResourceId::ALL[0];
    let mut low = still_meadow();
    low.resources[r.ix()] = 0.5;
    let mut full = still_meadow();
    full.resources[r.ix()] = full.cap(r);
    let n = drawn_rows(r).len();
    let (a, b) = (card(&low, r, WIDE, TALL), card(&full, r, WIDE, TALL));
    for y in 1..=n {
        for x in 0..16 {
            assert_eq!(
                (
                    a.cell((x, y as u16)).unwrap().symbol(),
                    fg(&a, x as usize, y)
                ),
                (
                    b.cell((x, y as u16)).unwrap().symbol(),
                    fg(&b, x as usize, y)
                ),
                "({x},{y})"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// TUI-RENDER-032 — the card gives way from the top
// ---------------------------------------------------------------------------

/// Exactly tall enough, everything stands. One row short, the drawing is gone
/// and all the rest stands whole. One row short of that, the description is
/// gone too, and the figures and flow lines still stand whole.
/// @spec TUI-RENDER-032
#[test]
fn a_short_pane_gives_up_the_drawing_then_the_description_and_never_a_figure() {
    let s = meadow();
    let mut moved = 0;
    for r in ResourceId::ALL {
        let whole = text(&card(&s, r, WIDE, TALL));
        let n = drawn_rows(r).len();
        let total = whole.len() as u16;
        let figures = &whole[n + 3..];
        if figures
            .iter()
            .any(|l| l.starts_with("In, ") || l.starts_with("Out, "))
        {
            moved += 1;
        }

        let fits = text(&card(&s, r, WIDE, total));
        assert_eq!(fits, whole, "{r:?}: exactly tall enough holds everything");

        let mut no_drawing = vec![whole[0].clone()];
        no_drawing.extend_from_slice(&whole[n + 1..]);
        for h in [total - 1, total - n as u16] {
            assert_eq!(
                text(&card(&s, r, WIDE, h)),
                no_drawing,
                "{r:?} at height {h}: the drawing goes whole and all the rest stands"
            );
        }

        let mut bare = vec![whole[0].clone(), String::new()];
        bare.extend_from_slice(figures);
        assert_eq!(
            text(&card(&s, r, WIDE, total - n as u16 - 1)),
            bare,
            "{r:?}: then the description goes whole and every figure still stands"
        );
    }
    assert!(moved > 3, "fixture: flow lines stand on several cards");
}

/// The rule holds in the layouts a player meets at the smallest pane: a store
/// nothing moves shows its whole card at 80×24, side by side and swapped.
/// @spec TUI-RENDER-032, TUI-NAV-197
#[test]
fn a_store_nothing_moves_keeps_its_whole_card_at_the_smallest_panes() {
    let s = still_meadow();
    let r = ResourceId::ALL[0];
    let swap = match compute(Rect::new(0, 0, 80, 24), None, false).body {
        Body::Narrow { main, .. } => main,
        other => panic!("fixture: 80x24 lays out sidebar and list: {other:?}"),
    };
    let pane = match compute(Rect::new(0, 0, 120, 24), None, false).body {
        Body::Wide { inspector, .. } => inspector,
        other => panic!("fixture: 120x24 lays out three columns: {other:?}"),
    };
    for rect in [swap, pane] {
        let lines = text(&card(&s, r, rect.width, rect.height));
        let rows = drawn_rows(r);
        assert_eq!(lines[1..=rows.len()], rows[..], "{rect:?}: {lines:#?}");
        assert!(
            lines.iter().any(|l| l.starts_with("Stock: ")),
            "{rect:?}: {lines:#?}"
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-RENDER-033 — measured steady
// ---------------------------------------------------------------------------

/// A figure changing its digits changes where a line wraps at a narrow width,
/// and must never put the drawing or the description in or out: at every
/// width and height, the two stocks open their cards alike.
/// @spec TUI-RENDER-033
#[test]
fn a_figure_changing_its_digits_never_moves_the_picture_in_or_out() {
    let r = ResourceId::Catnip;
    let mut short = still_meadow();
    short.resources[r.ix()] = 1.0;
    let mut long = still_meadow();
    long.resources[r.ix()] = 123.125;
    let opening = |lines: &[String]| -> (bool, bool) {
        let rows = drawn_rows(r);
        let n = rows.len();
        let drawn = lines.len() > n && lines[1..=n] == rows[..];
        let captioned = lines.iter().any(|l| l.starts_with("\u{b7} "));
        (drawn, captioned)
    };
    let mut wraps_differ = false;
    for w in NARROWEST..=40 {
        let tall = (
            text(&card(&short, r, w, TALL)).len(),
            text(&card(&long, r, w, TALL)).len(),
        );
        wraps_differ |= tall.0 != tall.1;
        for h in 4..=26 {
            let a = text(&card(&short, r, w, h));
            let b = text(&card(&long, r, w, h));
            assert_eq!(
                opening(&a),
                opening(&b),
                "{w}x{h}: the digits of a figure moved the opening:\n{a:#?}\n{b:#?}"
            );
        }
    }
    assert!(
        wraps_differ,
        "fixture: the two stocks wrap differently somewhere"
    );
}
