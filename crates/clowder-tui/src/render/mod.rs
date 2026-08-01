//! Drawing. A frame is a pure function of (snapshot, view state, size):
//! nothing cached between frames, no game value mirrored shell-side, and no
//! decision depending on what was drawn last. That is what makes layout
//! testable against an in-memory buffer at an exact size, which in turn is
//! what makes the size-dependent navigation specs verifiable rather than
//! hoped-for. @spec TUI-RENDER-001

pub mod fmt;
pub mod widgets;
pub mod layout;

use clowder_core::{content, ContentId, GameState};
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::nav::rows::{rows, Row, RowKey, RowKind};
use crate::nav::{Section, ViewState};

pub use layout::{Body, Layout};

/// The section a frame draws when the view state has not chosen one yet.
const DEFAULT_SECTION: Section = Section::Meadow;

/// A row's stable display name — the content name where the row is content,
/// or a readable label invented from the row's own fixed key otherwise.
/// @spec TUI-RENDER-001
fn row_label(state: &GameState, row: &Row) -> String {
    match &row.key {
        RowKey::Content(id) => content_name(*id),
        RowKey::Resource(r) => humanize(&format!("{r:?}")),
        RowKey::Colony(c) => humanize(&format!("{c:?}")),
        RowKey::Site(s) => humanize(&format!("{s:?}")),
        RowKey::Adventure(Some(a)) => humanize(&format!("{a:?}")),
        RowKey::Adventure(None) => "No Adventure".to_string(),
        RowKey::Kitten(ix) => {
            let name = state.kittens.get(*ix).map(|k| k.name.as_str()).unwrap_or("");
            if name.is_empty() {
                format!("Kitten {}", ix + 1)
            } else {
                name.to_string()
            }
        }
        RowKey::Fixed(tag) => humanize(tag),
    }
}

/// The def name for a piece of content, wherever its def table lives.
///
/// Traditions carry no name in their def (only their Elder-Wisdom price), so
/// a Tradition row's label is invented from its id the same way a fixed row's
/// is. @spec TUI-RENDER-001
fn content_name(id: ContentId) -> String {
    match id {
        ContentId::Structure(s) => content::structure_def(s).name.to_string(),
        ContentId::Idea(i) => content::idea_def(i).name.to_string(),
        ContentId::Craft(c) => content::craft_def(c).name.to_string(),
        ContentId::Job(j) => content::job_def(j).name.to_string(),
        ContentId::Rite(r) => content::rite_def(r).name.to_string(),
        ContentId::Arch(a) => content::arch_def(a).name.to_string(),
        ContentId::Refinement(r) => content::refinement_def(r).name.to_string(),
        ContentId::Rule(r) => content::rule_def(r).name.to_string(),
        ContentId::Tradition(t) => humanize(&format!("{t:?}")),
        ContentId::DreamLoom(l) => content::dream_loom_def(l).name.to_string(),
    }
}

/// Turn an identifier-shaped tag (`"GatherCatnip"`, `"HostJamboree"`) into a
/// spaced label, so a fixed row reads like the others without a name table of
/// its own.
fn humanize(tag: &str) -> String {
    let mut out = String::with_capacity(tag.len() + 4);
    for (i, c) in tag.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

/// The window of row indices to draw, and how many rows lie outside it.
struct Window {
    start: usize,
    len: usize,
    hidden: usize,
}

/// Derive the visible slice of a row list from the cursor alone — never from
/// a stored offset — keeping a small margin above and below where the list
/// has rows to spare. @spec TUI-RENDER-009
fn window_for(total: usize, cursor: usize, capacity: usize) -> Window {
    if capacity == 0 || total == 0 {
        return Window { start: 0, len: 0, hidden: total };
    }
    if total <= capacity {
        return Window { start: 0, len: total, hidden: 0 };
    }

    // One row of the pane is reserved for the overflow note, so the rows
    // themselves get `capacity - 1`. @spec TUI-RENDER-011
    let usable = capacity.saturating_sub(1).max(1);
    const MARGIN: usize = 3;

    let max_start = total - usable;
    let start = cursor.saturating_sub(MARGIN).min(max_start);
    let hidden = total - (start + usable);
    Window { start, len: usable, hidden }
}

/// Render one section's rows into a pane, deriving the scroll window from the
/// cursor. @spec TUI-RENDER-009, TUI-RENDER-010, TUI-RENDER-011, TUI-RENDER-015
fn draw_list(f: &mut Frame, area: Rect, state: &GameState, section: Section, cursor_key: Option<&RowKey>) {
    let all = rows(state, section);
    let cursor = cursor_key
        .and_then(|k| all.iter().position(|r| &r.key == k))
        .unwrap_or(0);

    let capacity = area.height as usize;
    let win = window_for(all.len(), cursor, capacity);

    let mut lines: Vec<Line<'static>> = Vec::with_capacity(win.len);
    for row in all.iter().skip(win.start).take(win.len) {
        let label = row_label(state, row);
        let text = match row.kind {
            RowKind::Group => format!("── {label} ──"),
            _ => label,
        };
        lines.push(Line::from(fmt::fit(&text, area.width as usize)));
    }
    if win.hidden > 0 {
        lines.push(Line::from(format!("{} more below", win.hidden)));
    }

    f.render_widget(Paragraph::new(lines), area);
}

/// The sidebar: a handful of always-relevant numbers, never the full content
/// table — the sidebar's cost must not grow with the resource count either.
/// @spec TUI-RENDER-012, TUI-RENDER-013, TUI-RENDER-015
fn draw_sidebar(f: &mut Frame, area: Rect, state: &GameState) {
    let lines: Vec<Line<'static>> = vec![
        Line::from(format!("Day {}", state.ticks / content::TICKS_PER_SECOND.max(1))),
        Line::from(format!("Kittens {}", state.kittens.len())),
    ];
    f.render_widget(Paragraph::new(lines), area);
}

/// The inspector: detail on the focused row alone, never the whole list — the
/// inspector's cost must not grow with the section's row count.
/// @spec TUI-RENDER-012, TUI-RENDER-015
fn draw_inspector(f: &mut Frame, area: Rect, state: &GameState, section: Section, cursor_key: Option<&RowKey>) {
    let all = rows(state, section);
    let focused = cursor_key.and_then(|k| all.iter().find(|r| &r.key == k));
    let lines: Vec<Line<'static>> = match focused {
        Some(row) => vec![Line::from(row_label(state, row))],
        None => vec![],
    };
    f.render_widget(Paragraph::new(lines), area);
}

/// Draw one frame, returning the layout it used.
///
/// The layout must escape: the breakpoints' dead-band is a function of the
/// *previous* frame's layout, so a frame that computed one and dropped it
/// would leave the next frame unable to tell which side of a boundary it was
/// already on. @spec TUI-RENDER-001, TUI-RENDER-002, TUI-RENDER-008
pub fn frame(
    f: &mut Frame,
    state: &GameState,
    view: &ViewState,
    prev: Option<&Layout>,
) -> Layout {
    let area = f.area();
    // TUI-NAV-050: an unread report is what puts the banner on screen.
    let banner = view.reports.iter().any(|r| !r.dismissed);
    let layout = layout::compute(area, prev, banner);

    // @spec TUI-RENDER-004
    if let Body::TooSmall(rect) = layout.body {
        f.render_widget(Paragraph::new("too small"), rect);
        return layout;
    }

    let section = view.section.unwrap_or(DEFAULT_SECTION);
    let cursor_key = view.focus.get(section.index()).and_then(|k| k.as_ref());

    f.render_widget(Paragraph::new(Line::from(format!("clowder — {}", section.title()))), layout.header);

    let tabs: Vec<String> = Section::ALL
        .iter()
        .filter(|s| !rows(state, **s).is_empty())
        .map(|s| s.title().to_string())
        .collect();
    f.render_widget(Paragraph::new(Line::from(tabs.join("  "))), layout.tabs);

    match layout.body {
        Body::TooSmall(_) => unreachable!("handled above"),
        Body::Narrow { sidebar, main } => {
            draw_sidebar(f, sidebar, state);
            if view.inspector_wanted {
                draw_inspector(f, main, state, section, cursor_key);
            } else {
                draw_list(f, main, state, section, cursor_key);
            }
        }
        Body::Wide { sidebar, list, inspector } => {
            draw_sidebar(f, sidebar, state);
            draw_list(f, list, state, section, cursor_key);
            draw_inspector(f, inspector, state, section, cursor_key);
        }
    }

    f.render_widget(Paragraph::new(Line::from("")), layout.ticker);
    f.render_widget(Paragraph::new(Line::from("")), layout.hints);

    layout
}
