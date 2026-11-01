//! What the sidebar says about the row under the cursor: a mark beside each
//! store the row raises, lowers, or does both to, in a column every store
//! already keeps free.
//!
//! @spec TUI-NAV-198, TUI-NAV-199, TUI-NAV-200, TUI-RENDER-034

use clowder_core::{
    ArchId, ContentId, GameState, IdeaId, ResourceId, SiteId, StoreLink, StructureId,
};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::Terminal;

use clowder_tui::nav::rows::{drawn_rows, rows, Row, RowKind};
use clowder_tui::nav::{RowKey, Section, ViewState};
use clowder_tui::render::layout::{Body, Layout};

const RAISES: char = '\u{25b5}';
const LOWERS: char = '\u{25bf}';
const BOTH: char = '\u{25ca}';

fn meadow() -> GameState {
    let mut s = GameState::new(11);
    for b in StructureId::ALL {
        s.structures[b.ix()] = 2;
    }
    s.ideas = IdeaId::ALL.into_iter().collect();
    s.sites_reached = SiteId::ALL.into_iter().collect();
    s.revealed = StructureId::ALL
        .into_iter()
        .map(ContentId::Structure)
        .chain(IdeaId::ALL.into_iter().map(ContentId::Idea))
        .collect();
    s.generation = 1;
    s.peak_devotion = 1.0e12;
    s.arches_bought.insert(ArchId::EverdreamArch);
    for r in ResourceId::ALL {
        s.resources[r.ix()] = 5.0;
    }
    s
}

fn focused(section: Section, key: RowKey) -> ViewState {
    let mut v = ViewState {
        section: Some(section),
        ..ViewState::default()
    };
    v.focus[section.index()] = Some(key);
    v
}

fn drawn(s: &GameState, view: &ViewState, (w, h): (u16, u16)) -> (Buffer, Layout) {
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("TestBackend is infallible");
    let mut layout = None;
    term.draw(|f| layout = Some(clowder_tui::render::frame(f, s, view, None)))
        .expect("TestBackend is infallible");
    (
        term.backend().buffer().clone(),
        layout.expect("a frame returns its layout"),
    )
}

fn sidebar_rect(l: &Layout) -> Rect {
    match l.body {
        Body::Narrow { sidebar, .. } | Body::Wide { sidebar, .. } => sidebar,
        Body::TooSmall(_) => panic!("fixture: the frame is too small"),
    }
}

/// A resource's name as the sidebar spells it.
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

fn line(buf: &Buffer, rect: Rect, y: u16) -> String {
    (rect.x..rect.right())
        .map(|x| buf.cell((x, rect.y + y)).map(|c| c.symbol()).unwrap_or(" "))
        .collect()
}

/// The sidebar's marks: each store it draws whole and the character in the
/// column directly left of its stock's field.
fn marks(buf: &Buffer, rect: Rect) -> Vec<(ResourceId, char)> {
    let col = rect.width as usize - 14;
    let mut out = Vec::new();
    for y in 0..rect.height {
        let text = line(buf, rect, y);
        let cells: Vec<char> = text.chars().collect();
        let hit = ResourceId::ALL.into_iter().find(|r| {
            let name = label(*r);
            let cut: String = name.chars().take(col.saturating_sub(1)).collect();
            text.starts_with(&name) || (name.chars().count() >= col && text.starts_with(&cut))
        });
        if let Some(r) = hit {
            out.push((r, cells[col]));
        }
    }
    out
}

fn glyph(l: Option<StoreLink>) -> char {
    match l {
        Some(StoreLink::Raises) => RAISES,
        Some(StoreLink::Lowers) => LOWERS,
        Some(StoreLink::Both) => BOTH,
        None => ' ',
    }
}

/// The rows a section's list draws, a folded done group's left out: the cursor
/// can stand only on these.
fn listed(s: &GameState, section: Section) -> Vec<Row> {
    let view = ViewState {
        section: Some(section),
        ..ViewState::default()
    };
    drawn_rows(s, section, &view)
}

/// The row's command as a mark reads it: a count row by its increment.
fn command_of(kind: &RowKind) -> Option<clowder_core::Command> {
    match kind {
        RowKind::Act { command, .. } => Some(*command),
        RowKind::Count { increment, .. } => Some(*increment),
        _ => None,
    }
}

/// On every row with a command, in every section, each store drawn carries the
/// mark the core's query reports for that row, and no other.
/// @spec TUI-NAV-198
#[test]
fn every_row_marks_each_store_as_the_core_reports_it() {
    let s = meadow();
    let (mut rows_seen, mut raised, mut lowered, mut both) = (0, 0, 0, 0);
    for section in Section::ALL {
        for row in listed(&s, section) {
            let Some(cmd) = command_of(&row.kind) else {
                continue;
            };
            let view = focused(section, row.key.clone());
            let (buf, layout) = drawn(&s, &view, (130, 110));
            let links = s.store_links(&cmd);
            rows_seen += 1;
            for (r, got) in marks(&buf, sidebar_rect(&layout)) {
                let want = glyph(links.iter().find(|(x, _)| *x == r).map(|(_, l)| *l));
                assert_eq!(got, want, "{:?} and {r:?}", row.key);
                match got {
                    RAISES => raised += 1,
                    LOWERS => lowered += 1,
                    BOTH => both += 1,
                    _ => {}
                }
            }
        }
    }
    assert!(rows_seen > 50, "fixture: many rows carry a command");
    assert!(
        raised > 0 && lowered > 0 && both > 0,
        "fixture: every mark drawn: {raised} {lowered} {both}"
    );
}

/// A mark changes the one cell it stands in: no name shortens, no figure moves,
/// the notes read as they did, and the sidebar stays its width.
/// @spec TUI-RENDER-034
#[test]
fn a_mark_changes_only_the_column_it_stands_in() {
    let s = meadow();
    let marked = focused(Section::Meadow, RowKey::Fixed("GatherCatnip"));
    let plain = focused(Section::Meadow, RowKey::Resource(ResourceId::Catnip));
    for size in [(80u16, 24u16), (120, 30)] {
        let (a, la) = drawn(&s, &marked, size);
        let (b, lb) = drawn(&s, &plain, size);
        let rect = sidebar_rect(&la);
        assert_eq!(rect, sidebar_rect(&lb), "the sidebar keeps its place");
        let col = rect.width as usize - 14;
        let mut differ = 0;
        for y in 0..rect.height {
            let (ta, tb): (Vec<char>, Vec<char>) = (
                line(&a, rect, y).chars().collect(),
                line(&b, rect, y).chars().collect(),
            );
            for x in 0..ta.len() {
                if ta[x] != tb[x] {
                    assert_eq!(x, col, "at {size:?}, ({x},{y}) changed");
                    differ += 1;
                }
            }
        }
        assert!(differ > 0, "fixture: at {size:?} a mark is drawn");
    }
}

/// A mark is a glyph with no colour of its own.
/// @spec TUI-NAV-198
#[test]
fn a_mark_wears_the_inherited_foreground() {
    let s = meadow();
    let view = focused(Section::Meadow, RowKey::Fixed("GatherCatnip"));
    let (buf, layout) = drawn(&s, &view, (120, 40));
    let rect = sidebar_rect(&layout);
    let col = rect.width - 14;
    let mut seen = 0;
    for y in 0..rect.height {
        let cell = buf.cell((rect.x + col, rect.y + y)).unwrap();
        if [RAISES, LOWERS, BOTH]
            .iter()
            .any(|g| cell.symbol().starts_with(*g))
        {
            assert_eq!(cell.fg, Color::Reset, "row {y}");
            seen += 1;
        }
    }
    assert!(seen > 0, "fixture: a mark is drawn");
}

/// The marks stay while the narrow-mode swap shows the inspector in the list's
/// place, the row under the cursor being the one it describes.
/// @spec TUI-NAV-198
#[test]
fn the_swap_keeps_the_marks() {
    let s = meadow();
    let mut view = focused(Section::Meadow, RowKey::Fixed("GatherCatnip"));
    view.inspector_wanted = true;
    let (buf, layout) = drawn(&s, &view, (80, 40));
    let got = marks(&buf, sidebar_rect(&layout));
    assert!(
        got.contains(&(ResourceId::Catnip, RAISES)),
        "the swap's sidebar: {got:?}"
    );
}

/// Where the remembered row has left the list, the marks follow the row the
/// list highlights instead.
/// @spec TUI-NAV-198
#[test]
fn the_marks_follow_the_row_the_list_highlights() {
    let s = meadow();
    let gone = focused(Section::Meadow, RowKey::Fixed("NoSuchRow"));
    let first = rows(&s, Section::Meadow)
        .into_iter()
        .find(|r| r.selectable())
        .expect("fixture: a selectable row");
    let there = focused(Section::Meadow, first.key.clone());
    let (a, la) = drawn(&s, &gone, (120, 60));
    let (b, lb) = drawn(&s, &there, (120, 60));
    assert_eq!(
        marks(&a, sidebar_rect(&la)),
        marks(&b, sidebar_rect(&lb)),
        "falling back to {:?}",
        first.key
    );
}

/// A row with no command marks nothing: a stores row would only point at itself.
/// @spec TUI-NAV-199
#[test]
fn a_row_with_no_command_marks_nothing() {
    let s = meadow();
    let mut checked = 0;
    for section in Section::ALL {
        for row in listed(&s, section) {
            if !row.selectable() || command_of(&row.kind).is_some() {
                continue;
            }
            let (buf, layout) = drawn(&s, &focused(section, row.key.clone()), (120, 110));
            checked += 1;
            for (r, got) in marks(&buf, sidebar_rect(&layout)) {
                assert_eq!(got, ' ', "{:?} marks {r:?}", row.key);
            }
        }
    }
    assert!(checked > 10, "fixture: rows with no command");
}

/// The manual names the marks.
/// @spec TUI-NAV-200
#[test]
fn the_help_overlay_names_the_marks() {
    let mut term = Terminal::new(TestBackend::new(120, 60)).expect("TestBackend is infallible");
    term.draw(|f| {
        let area = f.area();
        clowder_tui::render::widgets::overlays::help(f, area);
    })
    .expect("TestBackend is infallible");
    let buf = term.backend().buffer().clone();
    let text: String = (0..60)
        .map(|y| line(&buf, Rect::new(0, 0, 120, 60), y))
        .collect::<Vec<_>>()
        .join("\n");
    for g in [RAISES, LOWERS, BOTH] {
        assert!(text.contains(g), "the legend names {g:?}");
    }
}
