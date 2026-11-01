//! The Meadow's energy and mess readouts, and the header's `↯energy` chip.
//!
//! @spec TUI-NAV-020, TUI-NAV-098, TUI-NAV-100, TUI-NAV-167, TUI-NAV-168,
//! TUI-NAV-169, TUI-NAV-170, TUI-THEME-015, TUI-RENDER-030

use clowder_core::content::structure_def;
use clowder_core::{GameState, StructureId};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::Terminal;

use clowder_tui::nav::rows::{rows, RowKind};
use clowder_tui::nav::{RowKey, Section, ViewState};
use clowder_tui::render::layout::{Body, Layout};
use clowder_tui::theme::Role;

const WIDEST: u16 = 200;
const TALL: u16 = 60;

struct Drawn {
    buffer: Buffer,
    layout: Layout,
}

fn draw(state: &GameState, view: &ViewState, w: u16, h: u16) -> Drawn {
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("TestBackend is infallible");
    let mut layout = None;
    term.draw(|f| layout = Some(clowder_tui::render::frame(f, state, view, None)))
        .expect("TestBackend is infallible");
    Drawn {
        buffer: term.backend().buffer().clone(),
        layout: layout.expect("a frame was drawn"),
    }
}

fn region(buf: &Buffer, rect: Rect) -> Vec<String> {
    (rect.y..rect.bottom())
        .map(|y| {
            (rect.x..rect.right())
                .map(|x| buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "))
                .collect::<String>()
        })
        .collect()
}

fn meadow_view(key: Option<RowKey>) -> ViewState {
    let mut v = ViewState {
        section: Some(Section::Meadow),
        ..Default::default()
    };
    v.focus[Section::Meadow.index()] = key;
    v
}

fn list_rect(d: &Drawn) -> Rect {
    match d.layout.body {
        Body::Wide { list, .. } => Rect::new(list.x + 1, list.y, list.width - 2, list.height),
        _ => panic!("fixture: a wide layout"),
    }
}

fn inspector_text(state: &GameState, key: RowKey) -> String {
    let d = draw(state, &meadow_view(Some(key)), WIDEST, TALL);
    let Body::Wide { inspector, .. } = d.layout.body else {
        panic!("fixture: the inspector pane");
    };
    region(&d.buffer, inspector)
        .into_iter()
        .map(|l| {
            l.trim_start_matches('│')
                .trim_end_matches([' ', '│'])
                .trim()
                .to_string()
        })
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn header(state: &GameState, w: u16) -> (String, Drawn) {
    let d = draw(state, &meadow_view(None), w, TALL);
    let line = region(&d.buffer, Rect::new(0, 0, w, 1)).remove(0);
    (line, d)
}

/// A meadow that makes `made` and uses `used` energy: Cogworks (+3 each)
/// against Box Labyrinths (−2 each).
fn powered(cogworks: u32, labyrinths: u32) -> GameState {
    let mut s = GameState::new(41);
    s.structures[StructureId::Cogworks.ix()] = cogworks;
    s.structures[StructureId::BoxLabyrinth.ix()] = labyrinths;
    s
}

fn keys(state: &GameState) -> Vec<RowKey> {
    rows(state, Section::Meadow)
        .into_iter()
        .map(|r| r.key)
        .collect()
}

// ---------------------------------------------------------------------------
// TUI-NAV-167, 098, 100 — when the rows are listed, and where
// ---------------------------------------------------------------------------

/// No energy structure built: no Energy row. One built: the row stands in an
/// `energy & mess` group between the structures and the stores, display-only.
/// @spec TUI-NAV-167, TUI-NAV-098, TUI-NAV-100
#[test]
fn the_energy_row_is_listed_once_anything_makes_or_uses_energy() {
    let bare = GameState::new(40);
    assert!(!keys(&bare).contains(&RowKey::Energy));
    assert!(!keys(&bare).contains(&RowKey::Mess));

    let s = powered(1, 0);
    let all = rows(&s, Section::Meadow);
    let at = all
        .iter()
        .position(|r| r.key == RowKey::Energy)
        .expect("listed");
    assert_eq!(all[at].kind, RowKind::Display);
    let header = all[..at]
        .iter()
        .rev()
        .find(|r| matches!(r.key, RowKey::Fixed(_)) && r.kind == RowKind::Group)
        .expect("a group header above");
    assert_eq!(header.key, RowKey::Fixed("energy & mess"));
    let structures = all
        .iter()
        .position(|r| r.key == RowKey::Fixed("structures"));
    let stores = all.iter().position(|r| r.key == RowKey::Fixed("stores"));
    if let Some(st) = structures {
        assert!(st < at, "structures above");
    }
    if let Some(st) = stores {
        assert!(at < st, "stores below");
    }
}

/// A structure that makes mess lists the Mess row, as does any mess held.
/// @spec TUI-NAV-167
#[test]
fn the_mess_row_is_listed_with_anything_messy() {
    let mut s = GameState::new(42);
    s.structures[StructureId::GravelPit.ix()] = 1;
    assert!(structure_def(StructureId::GravelPit).mess > 0.0, "fixture");
    assert!(keys(&s).contains(&RowKey::Mess));
    let mut held = GameState::new(43);
    held.mess = 50.0;
    assert!(keys(&held).contains(&RowKey::Mess));
    let row = rows(&held, Section::Meadow)
        .into_iter()
        .find(|r| r.key == RowKey::Mess)
        .unwrap();
    assert_eq!(row.kind, RowKind::Display);
}

/// A structure that tidies mess lists the Mess row too, even in a clean
/// meadow with nothing else messy built.
/// @spec TUI-NAV-167
#[test]
fn the_mess_row_is_listed_with_anything_tidying() {
    let mut s = GameState::new(44);
    s.structures[StructureId::BroomCupboard.ix()] = 1;
    assert!(
        structure_def(StructureId::BroomCupboard).mess < 0.0,
        "fixture"
    );
    assert_eq!(s.mess, 0.0, "fixture");
    assert!(keys(&s).contains(&RowKey::Mess));
}

// ---------------------------------------------------------------------------
// TUI-NAV-168, TUI-THEME-015, TUI-RENDER-030 — what the rows read
// ---------------------------------------------------------------------------

/// The Energy row reads made and used, and `short` in attention while short.
/// @spec TUI-NAV-168, TUI-THEME-015, TUI-RENDER-030
#[test]
fn the_energy_row_reads_made_and_used_and_short() {
    let fine = powered(2, 1);
    let d = draw(&fine, &meadow_view(None), WIDEST, TALL);
    let lines = region(&d.buffer, list_rect(&d));
    let row = lines.iter().find(|l| l.contains("Energy")).expect("drawn");
    assert!(row.contains("6 made · 2 used"), "{row:?}");
    assert!(!row.contains("short"));

    let short = powered(1, 3);
    let d = draw(&short, &meadow_view(None), WIDEST, TALL);
    let rect = list_rect(&d);
    let lines = region(&d.buffer, rect);
    let y = lines
        .iter()
        .position(|l| l.contains("Energy"))
        .expect("drawn");
    assert!(
        lines[y].contains("3 made · 6 used · short"),
        "{:?}",
        lines[y]
    );
    let col =
        lines[y].chars().count() - lines[y][lines[y].find("short").unwrap()..].chars().count();
    let cell = |x: usize| {
        d.buffer
            .cell((rect.x + x as u16, rect.y + y as u16))
            .unwrap()
            .fg
    };
    let attention = Role::Attention.color().unwrap();
    for x in col..col + 5 {
        assert_eq!(cell(x), attention, "`short` wears attention");
    }
    let made =
        lines[y].chars().count() - lines[y][lines[y].find("3 made").unwrap()..].chars().count();
    assert_ne!(cell(made), attention, "the figures do not");
}

/// The Mess row reads `tidy` with no level, `level N` with one.
/// @spec TUI-NAV-168
#[test]
fn the_mess_row_reads_tidy_or_its_level() {
    let mut s = GameState::new(44);
    s.mess = 50.0;
    let d = draw(&s, &meadow_view(None), WIDEST, TALL);
    let lines = region(&d.buffer, list_rect(&d));
    assert!(
        lines
            .iter()
            .any(|l| l.contains("Mess") && l.contains("tidy")),
        "{lines:#?}"
    );
    s.mess = 1.0e5;
    let d = draw(&s, &meadow_view(None), WIDEST, TALL);
    let lines = region(&d.buffer, list_rect(&d));
    assert!(
        lines
            .iter()
            .any(|l| l.contains("Mess") && l.contains("level 1")),
        "{lines:#?}"
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-169, 170 — what the inspector says
// ---------------------------------------------------------------------------

/// Totals, the shares one structure to a line, and what a shortfall slows.
/// @spec TUI-NAV-169
#[test]
fn the_energy_inspector_says_where_it_goes() {
    let short = powered(1, 3);
    let cog = structure_def(StructureId::Cogworks).name;
    let lab = structure_def(StructureId::BoxLabyrinth).name;
    let text = inspector_text(&short, RowKey::Energy);
    for want in [
        "Made: 3".to_string(),
        "Used: 6".to_string(),
        "Makes energy:".to_string(),
        format!("{cog} ×1 +3"),
        "Uses energy:".to_string(),
        format!("{lab} ×3 -6"),
        "Short of energy: global production, Elder Wisdom, and the Long Sun run at 50%."
            .to_string(),
    ] {
        assert!(text.contains(&want), "missing {want:?} in:\n{text}");
    }
    let fine = inspector_text(&powered(3, 1), RowKey::Energy);
    assert!(
        fine.contains("Enough energy for everything, with 7 to spare."),
        "{fine}"
    );
    let even = inspector_text(&powered(2, 3), RowKey::Energy);
    assert!(
        even.contains("Exactly enough energy for everything."),
        "{even}"
    );
}

/// The amount and its direction, what the level slows, and the next level.
/// @spec TUI-NAV-170
#[test]
fn the_mess_inspector_says_what_it_slows() {
    let mut s = GameState::new(45);
    s.mess = 50.0;
    let calm = inspector_text(&s, RowKey::Mess);
    assert!(calm.contains("Mess: 50, falling by"), "{calm}");
    assert!(calm.contains("Not enough to slow anything down."), "{calm}");
    assert!(calm.contains("Level 0 begins at 10000."), "{calm}");

    s.mess = 1.0e6;
    let messy = inspector_text(&s, RowKey::Mess);
    assert!(
        messy.contains("Catnip grows at 51.2% of its pace."),
        "{messy}"
    );
    assert!(
        messy.contains("Kittens arrive at 75% of their pace."),
        "{messy}"
    );
    assert!(messy.contains("Happiness −5."), "{messy}");
    assert!(messy.contains("Level 3 begins at 10000000."), "{messy}");
}

// ---------------------------------------------------------------------------
// TUI-NAV-020 — the chip
// ---------------------------------------------------------------------------

/// `↯energy` stands in the header while the meadow is short, last in the
/// cluster, in attention; never otherwise.
/// @spec TUI-NAV-020, TUI-THEME-015
#[test]
fn the_energy_chip_shows_while_short() {
    let (line, _) = header(&powered(2, 1), WIDEST);
    assert!(!line.contains("↯energy"), "{line}");
    let (line, d) = header(&powered(1, 3), WIDEST);
    assert!(line.trim_end().ends_with("↯energy"), "{line}");
    let col = line.chars().count() - line[line.find("↯energy").unwrap()..].chars().count();
    for x in col..col + "↯energy".chars().count() {
        assert_eq!(
            d.buffer.cell((x as u16, 0)).unwrap().fg,
            Role::Attention.color().unwrap()
        );
    }
    let (narrow, _) = header(&powered(1, 3), 80);
    assert!(narrow.contains("↯energy"), "kept at the floor: {narrow}");
    let _ = Color::Reset;
}
