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
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::nav::rows::{rows, Row, RowKey, RowKind};
use crate::nav::{CountPrompt, Overlay, Section, ViewState};
use crate::theme::Role;

pub use layout::{Body, Layout};

/// The section a frame draws when the view state has not chosen one yet.
const DEFAULT_SECTION: Section = Section::Meadow;

/// A palette role as a style. `None` means the terminal's own foreground —
/// clowder paints no colour where none is meant. @spec TUI-THEME-001
fn style_for(role: Role) -> Style {
    match role.color() {
        Some(c) => Style::default().fg(c),
        None => Style::default(),
    }
}

/// The glyph-and-role a row wears, taken from the core's answers rather than
/// worked out here. Glyph first: the marker is legible with every colour
/// stripped. @spec TUI-NAV-009, TUI-NAV-053, TUI-THEME-014
fn marker_for(state: &GameState, row: &Row) -> (&'static str, Role) {
    let RowKey::Content(id, _) = &row.key else {
        return ("●", Role::Ready);
    };
    if state.lock_reason(*id) == Some(clowder_core::LockReason::AlreadyYours) {
        return ("✓", Role::Dim);
    }
    match state.availability(*id) {
        clowder_core::Availability::Teased => ("○", Role::Dim),
        _ => {
            let covered = state.affordability(*id);
            if covered >= 1.0 {
                ("●", Role::Ready)
            } else if covered > 0.0 {
                ("◐", Role::Partial)
            } else {
                ("○", Role::Dim)
            }
        }
    }
}

/// How many of this the meadow already has, blank where the notion does not
/// apply. @spec TUI-NAV-009
fn owned_count(state: &GameState, row: &Row) -> String {
    match &row.key {
        RowKey::Content(ContentId::Structure(b), _) => {
            let n = state.structures[b.ix()];
            if n > 0 { format!("×{n}") } else { String::new() }
        }
        _ => String::new(),
    }
}

/// What an open count prompt on `row` previews, in the row list's cost slot:
/// the core's per-component total for taking the count (a repeat-N row), N
/// times the core's one-unit price (a Pounce or Trade row), or the typed
/// count alone — DeepNap, a count row, a count row proper, or a row the core
/// prices no other way — with nothing beyond the typed digits for an empty
/// or zero count. @spec TUI-NAV-067
fn count_prompt_preview(state: &GameState, row: &Row, digits: &str) -> String {
    use crate::nav::CountSemantics;

    let n: u32 = digits.parse().unwrap_or(0);
    if n == 0 {
        return format!("×{digits}");
    }

    let priced: Vec<(clowder_core::ResourceId, f64)> = match &row.kind {
        RowKind::Act { semantics: CountSemantics::RepeatN, .. } => {
            let RowKey::Content(id, _) = row.key else { return format!("×{n}") };
            state.cost_of_next(id, n)
        }
        RowKind::Act { command, semantics: CountSemantics::CountTaking } if !row.heavy => {
            match state.unit_price(command) {
                Some(unit) => unit.into_iter().map(|(r, each)| (r, each * n as f64)).collect(),
                None => return format!("×{n}"),
            }
        }
        // DeepNap (heavy count-taking), a count row, and a row Single-tagged
        // — x never opens on a Single row, so this arm is reached only by a
        // count row's own `Count` kind. @spec TUI-NAV-067
        _ => return format!("×{n}"),
    };

    if priced.is_empty() {
        return format!("×{n}");
    }
    let parts: Vec<String> = priced
        .iter()
        .map(|(r, amount)| format!("{} {}", fmt::exact(*amount), format!("{r:?}").to_lowercase()))
        .collect();
    format!("×{n}  {}", parts.join(", "))
}

/// The next copy's price, compactly. @spec TUI-NAV-009, TUI-RENDER-012
fn next_cost_text(state: &GameState, row: &Row) -> String {
    let RowKey::Content(id, _) = &row.key else { return String::new() };
    let costs = state.cost_of_next(*id, 1);
    match costs.first() {
        Some((r, amount)) => {
            // Resource variants are already the player-facing names; lowered
            // so a cost reads as prose rather than as an identifier.
            format!("{} {}", fmt::compact(*amount), format!("{r:?}").to_lowercase())
        }
        None => String::new(),
    }
}

/// A row's stable display name — the content name where the row is content,
/// or a readable label invented from the row's own fixed key otherwise.
/// @spec TUI-RENDER-001
fn row_label(state: &GameState, row: &Row) -> String {
    match &row.key {
        RowKey::Content(id, _) => content_name(*id),
        RowKey::Resource(r) => humanize(&format!("{r:?}")),
        RowKey::Colony(c, _) => humanize(&format!("{c:?}")),
        RowKey::Site(s) => humanize(&format!("{s:?}")),
        RowKey::Adventure(Some(a)) => humanize(&format!("{a:?}")),
        RowKey::Adventure(None) => "No Adventure".to_string(),
        RowKey::Kitten(ix, _) => {
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
/// cursor. An open count prompt's preview draws on the exact row it was
/// opened on, by that row's identity, never by list position — so a cursor
/// that has since drifted (or an open prompt whose own row has left the
/// list) never previews on the wrong line.
/// @spec TUI-RENDER-009, TUI-RENDER-010, TUI-RENDER-011, TUI-RENDER-015, TUI-NAV-067
fn draw_list(
    f: &mut Frame,
    area: Rect,
    state: &GameState,
    section: Section,
    cursor_key: Option<&RowKey>,
    prompt: Option<&CountPrompt>,
) {
    let all = rows(state, section);
    let cursor = cursor_key
        .and_then(|k| all.iter().position(|r| &r.key == k))
        .unwrap_or(0);
    let prompt_key = prompt.and_then(|p| p.row.as_ref());

    let capacity = area.height as usize;
    let win = window_for(all.len(), cursor, capacity);

    let mut lines: Vec<Line<'static>> = Vec::with_capacity(win.len + 2);
    // TUI-RENDER-011: rows scrolled off above need saying too, or a cursor
    // deep in a long list looks like the top of it.
    if win.start > 0 {
        lines.push(Line::from(Span::styled(
            format!("{} more above", win.start),
            style_for(Role::Dim),
        )));
    }
    for (i, row) in all.iter().enumerate().skip(win.start).take(win.len) {
        let focused = i == cursor && row.selectable();
        let on_this_row = prompt.filter(|_| prompt_key == Some(&row.key));
        lines.push(row_line(state, row, focused, area.width as usize, on_this_row));
    }
    if win.hidden > 0 {
        lines.push(Line::from(Span::styled(
            format!("{} more below", win.hidden),
            style_for(Role::Dim),
        )));
    }

    f.render_widget(Paragraph::new(lines), area);
}

/// One list row, drawn as the design's anatomy: cursor, availability marker,
/// name, owned count, next cost.
///
/// Colour is the *second* channel throughout — every marker is a distinct
/// glyph before it is a colour, so the row reads the same to someone who
/// cannot tell the colours apart. Focus is the cursor glyph plus reverse
/// video, never a palette colour: the row's colour already says what its
/// state is, and a second colour would be saying something else at the same
/// time. @spec TUI-NAV-009, TUI-NAV-021, TUI-NAV-022, TUI-THEME-006, TUI-THEME-014
fn row_line(
    state: &GameState,
    row: &Row,
    focused: bool,
    width: usize,
    prompt: Option<&CountPrompt>,
) -> Line<'static> {
    let label = row_label(state, row);
    if matches!(row.kind, RowKind::Group) {
        return Line::from(Span::styled(
            fmt::fit(&format!("── {label} ──"), width),
            style_for(Role::Dim),
        ));
    }

    let (marker, role) = marker_for(state, row);
    let cursor = if focused { "▸ " } else { "  " };
    let count = owned_count(state, row);
    let cost = match prompt {
        Some(p) => count_prompt_preview(state, row, &p.digits),
        None => next_cost_text(state, row),
    };

    // Name gets whatever width the fixed parts leave.
    let fixed = cursor.len() + 2 + count.len() + cost.len() + 2;
    let name_w = width.saturating_sub(fixed).max(4);
    let name = fmt::fit(&label, name_w);

    let text = format!("{cursor}{marker} {name:<name_w$} {count} {cost}");
    let mut style = style_for(role);
    if focused {
        // Reverse the terminal's own pair — works in either polarity, and
        // needs no colour of its own. @spec TUI-THEME-007
        style = style.add_modifier(Modifier::REVERSED);
    }
    Line::from(Span::styled(fmt::fit(&text, width), style))
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

    // Each region is drawn by the widget that owns it. This function composes;
    // it decides nothing about what any region contains.
    widgets::chrome::header(f, layout.header, state, view);
    widgets::chrome::tabs(f, layout.tabs, state, view);

    // @spec TUI-RENDER-003, TUI-NAV-027
    if let Some(rect) = layout.banner {
        widgets::overlays::banner(f, rect, state, view);
    }

    // A full-screen overlay takes the body and nothing else — the chrome keeps
    // rendering and the sim keeps ticking behind it, so an overlay is never a
    // demand for attention. @spec TUI-NAV-026, TUI-NAV-025, TUI-NAV-038
    let body_rect = match layout.body {
        Body::TooSmall(r) => r,
        Body::Narrow { sidebar, main } => sidebar.union(main),
        Body::Wide { sidebar, inspector, .. } => sidebar.union(inspector),
    };
    match &view.overlay {
        Some(Overlay::Journal { .. }) => widgets::overlays::journal(f, body_rect, state, view),
        Some(Overlay::Help) => widgets::overlays::help(f, body_rect),
        Some(Overlay::Palette { .. }) => widgets::overlays::palette(f, body_rect, state, view),
        None => match layout.body {
            Body::TooSmall(_) => unreachable!("handled above"),
            Body::Narrow { sidebar, main } => {
                widgets::chrome::sidebar(f, sidebar, state);
                // TUI-NAV-035: swapped in over the list, the inspector still
                // follows the underlying cursor.
                if view.inspector_wanted {
                    widgets::inspector::draw(f, main, state, view);
                } else {
                    draw_list(f, main, state, section, cursor_key, view.prompt.as_ref());
                }
            }
            Body::Wide { sidebar, list, inspector } => {
                widgets::chrome::sidebar(f, sidebar, state);
                draw_list(f, list, state, section, cursor_key, view.prompt.as_ref());
                widgets::inspector::draw(f, inspector, state, view);
            }
        },
    }

    widgets::overlays::ticker(f, layout.ticker, state);
    // A footer note takes the hint bar's row while it lasts: it is the reply
    // to what the player just did, and it belongs where they are already
    // looking. @spec TUI-NAV-014
    match view.footer.as_ref().filter(|(_, until)| *until > std::time::Instant::now()) {
        Some((note, _)) => f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                fmt::fit(note, layout.hints.width as usize),
                style_for(Role::Attention),
            ))),
            layout.hints,
        ),
        None => widgets::chrome::hints(f, layout.hints, state, view),
    }

    layout
}
