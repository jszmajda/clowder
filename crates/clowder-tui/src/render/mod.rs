//! Drawing. A frame is a pure function of (snapshot, view state, size):
//! nothing cached between frames, no game value mirrored shell-side, and no
//! decision depending on what was drawn last. That is what makes layout
//! testable against an in-memory buffer at an exact size, which in turn is
//! what makes the size-dependent navigation specs verifiable rather than
//! hoped-for. @spec TUI-RENDER-001

pub mod fmt;
pub mod widgets;
pub mod layout;

use clowder_core::{content, Command, ContentId, GameState, Location, LockReason, ResourceId};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::nav::rows::{rows, Row, RowKey, RowKind};
use crate::nav::{ArmedRow, CountPrompt, Overlay, Section, ViewState};
use crate::theme::{self, Role};

pub use layout::{Body, Layout};

/// The section a frame draws when the view state has not chosen one yet.
const DEFAULT_SECTION: Section = Section::Meadow;

/// A palette role as a style. `None` means the terminal's own foreground —
/// clowder paints no colour where none is meant. @spec TUI-THEME-001
pub(crate) fn style_for(role: Role) -> Style {
    match role.color() {
        Some(c) => Style::default().fg(c),
        None => Style::default(),
    }
}

/// The glyph-and-role a row wears, taken from the core's answers rather than
/// worked out here — never from a price or a threshold of the shell's own.
/// `None` is a display-only row: an empty marker slot. Glyph first: the
/// marker is legible with every colour stripped.
/// @spec TUI-NAV-009, TUI-NAV-053, TUI-THEME-014
fn marker_for(state: &GameState, row: &Row) -> Option<(&'static str, Role)> {
    use clowder_core::{Availability, LockReason};

    if matches!(row.kind, RowKind::Display) {
        return None;
    }
    let RowKey::Content(id, tag) = &row.key else {
        return Some(("●", Role::Ready));
    };
    // A craft's tinkerer count row carries no price of its own to fall short
    // on — it wears the same marker as any actionable row with no content
    // item, whatever the craft's own lock reason.
    if *tag == "tinker" {
        return Some(("●", Role::Ready));
    }
    let (glyph, role) = match state.lock_reason(*id) {
        Some(LockReason::AlreadyYours) => ("✓", Role::Dim),
        None => ("●", Role::Ready),
        Some(LockReason::ShortBy { .. }) => {
            if state.affordability(*id) > 0.0 {
                ("◐", Role::Partial)
            } else {
                ("○", Role::Dim)
            }
        }
        Some(_) => ("○", Role::Dim),
    };
    // A Teased row is dimmed whatever glyph it wears, `◐` included, and the
    // dim is the availability report's answer, not the lock reason's: a kind
    // without a reveal ladder is never Teased, so its rows keep their glyph's
    // own colour. The core reports a revealed item Available exactly where
    // its lock reason names none (SIM-ENG-023), so the dim never lands on `●`.
    // @spec TUI-NAV-009, TUI-NAV-053
    let role = if state.availability(*id) == Availability::Teased {
        Role::Dim
    } else {
        role
    };
    Some((glyph, role))
}

/// A resource's stores-row spelling — the same name the sidebar and the
/// inspector give it. @spec TUI-RENDER-020, TUI-RENDER-021
fn resource_label(r: ResourceId) -> String {
    humanize(&format!("{r:?}"))
}

/// The widest stores-row name in the resource vocabulary — what the cost
/// field's width (TUI-RENDER-020) is sized against. Computed once: the
/// vocabulary is fixed for the life of the process.
fn widest_resource_name() -> usize {
    use std::sync::OnceLock;
    static WIDEST: OnceLock<usize> = OnceLock::new();
    *WIDEST.get_or_init(|| {
        ResourceId::ALL.iter().map(|r| resource_label(*r).chars().count()).max().unwrap_or(0)
    })
}

/// The cost field's width: six columns of figure, a space, the widest
/// stores-row name in the resource vocabulary, and three columns for `, …`.
/// @spec TUI-RENDER-020
fn cost_field_width() -> usize {
    6 + 1 + widest_resource_name() + 3
}

/// Whether a section's list gives an owned-count field (TUI-NAV-018): the
/// copies of a structure or a rite, the kittens on a job's count row, the
/// tinkerers on a craft's tinkerer row — judged by the section alone, so the
/// column is the same whatever is listed. @spec TUI-RENDER-020
fn section_gives_count(section: Section) -> bool {
    matches!(section, Section::Meadow | Section::Clowder | Section::Crafts | Section::Sunbeams)
}

/// Whether a section's list gives a next-cost field (TUI-NAV-078,
/// TUI-NAV-073), judged by the section alone. @spec TUI-RENDER-020
fn section_gives_cost(section: Section) -> bool {
    !matches!(section, Section::Sky | Section::Legacy)
}

/// The owned count the inspector states for a row, if it states one — the
/// same fact the count field draws (TUI-RENDER-020). @spec TUI-NAV-018
fn stated_count(state: &GameState, row: &Row) -> Option<u64> {
    match &row.key {
        RowKey::Content(ContentId::Structure(b), _) => Some(state.structures[b.ix()] as u64),
        RowKey::Content(ContentId::Rite(r), _) => Some(state.rite_counts[r.ix()] as u64),
        RowKey::Content(ContentId::Job(j), _) if matches!(row.kind, RowKind::Count { .. }) => {
            Some(state.effective_workers(*j) as u64)
        }
        RowKey::Content(ContentId::Craft(c), "tinker") => Some(
            state
                .kittens
                .iter()
                .filter(|k| k.tinkering == Some(*c) && k.location == Location::Home)
                .count() as u64,
        ),
        _ => None,
    }
}

/// The next cost the inspector states for a row, if it states one: the
/// core's price for one more on a row that takes a content item — none on a
/// tinkerer row, where the item is already the player's, or where the price
/// names no component — and the one-unit price on a Pounce or Trade row.
/// @spec TUI-NAV-078, TUI-NAV-073
fn stated_cost(state: &GameState, row: &Row) -> Option<Vec<(ResourceId, f64)>> {
    let cost = match (&row.key, &row.kind) {
        (RowKey::Content(_, "tinker"), _) => return None,
        (RowKey::Content(id, _), _) => {
            if state.lock_reason(*id) == Some(LockReason::AlreadyYours) {
                return None;
            }
            state.cost_of_next(*id, 1)
        }
        (_, RowKind::Act { command, .. }) => state.unit_price(command)?,
        _ => return None,
    };
    (!cost.is_empty()).then_some(cost)
}

/// The count field as TUI-RENDER-020 draws it: `×` joined to a compact
/// figure, the two right-aligned in seven columns, blank for zero or where
/// nothing is stated. @spec TUI-RENDER-020, TUI-RENDER-012
fn count_field_text(count: Option<u64>) -> String {
    match count {
        Some(n) if n > 0 => format!("{:>7}", format!("×{}", fmt::compact(n as f64))),
        _ => " ".repeat(7),
    }
}

/// The cost field as TUI-RENDER-020 draws it, `width` columns wide: each
/// component a compact figure, a space, and its stores-row name, apart by
/// `, ` — the first figure right-aligned in the field's first six columns —
/// and the longest leading run of whole components that fits, `, …` after it
/// where any is left out. @spec TUI-RENDER-020, TUI-RENDER-012
fn cost_field_text(cost: Option<&[(ResourceId, f64)]>, width: usize) -> String {
    let Some(cost) = cost else { return " ".repeat(width) };
    let parts: Vec<String> = cost
        .iter()
        .enumerate()
        .map(|(i, (r, a))| {
            if i == 0 {
                format!("{:>6} {}", fmt::compact(*a), resource_label(*r))
            } else {
                format!("{} {}", fmt::compact(*a), resource_label(*r))
            }
        })
        .collect();
    let mut text = String::new();
    for k in (1..=parts.len()).rev() {
        let run = parts[..k].join(", ");
        let tail = if k < parts.len() { ", …" } else { "" };
        if run.chars().count() + tail.chars().count() <= width {
            text = format!("{run}{tail}");
            break;
        }
    }
    format!("{text:<width$}")
}

/// The components an open count prompt's preview (TUI-NAV-067) prices for
/// `digits`: the core's per-component total for taking the count (a
/// repeat-N row), or N times the core's one-unit price (a Pounce or Trade
/// row) — none for DeepNap, a count row, or an empty or zero count, where the
/// preview is the count alone.
/// @spec TUI-NAV-067
fn prompt_preview_components(state: &GameState, row: &Row, digits: &str) -> Vec<(ResourceId, f64)> {
    use crate::nav::CountSemantics;

    let n: u32 = digits.parse().unwrap_or(0);
    if n == 0 {
        return Vec::new();
    }
    match &row.kind {
        RowKind::Act { semantics: CountSemantics::RepeatN, .. } => match row.key {
            RowKey::Content(id, _) => state.cost_of_next(id, n),
            _ => Vec::new(),
        },
        RowKind::Act { command, semantics: CountSemantics::CountTaking } if !row.heavy => state
            .unit_price(command)
            .map(|unit| unit.into_iter().map(|(r, each)| (r, each * n as f64)).collect())
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// A priced component of an open prompt's preview, in the exact form: what
/// TUI-RENDER-021 requires wherever the prompt's text is drawn.
fn prompt_preview_parts(comps: &[(ResourceId, f64)]) -> Vec<String> {
    comps.iter().map(|(r, a)| format!("{} {}", fmt::exact(*a), resource_label(*r))).collect()
}

/// `×` joined to the digits exactly as typed, then — where the preview prices
/// components — two blank columns and those components, apart by `, `.
/// @spec TUI-NAV-067, TUI-RENDER-021
fn prompt_text(parts: &[String], digits: &str) -> String {
    if parts.is_empty() {
        format!("×{digits}")
    } else {
        format!("×{digits}  {}", parts.join(", "))
    }
}

/// The prompt's text cut to fit `place` columns: whole where it fits; else
/// the count whole and the longest leading run of components that fits,
/// `, …` after it; else the count, two blank columns, and `…`.
/// @spec TUI-RENDER-021
fn prompt_text_cut(parts: &[String], digits: &str, place: usize) -> String {
    let whole = prompt_text(parts, digits);
    if whole.chars().count() <= place {
        return whole;
    }
    for k in (1..parts.len()).rev() {
        let run = format!("×{digits}  {}, …", parts[..k].join(", "));
        if run.chars().count() <= place {
            return run;
        }
    }
    format!("×{digits}  …")
}

/// What an open count prompt on `row` previews, in the row list's cost slot,
/// or in the narrow swap's inspector (TUI-NAV-035): `×` and the digits as
/// typed, then the priced components in the exact form. Shared by both, so
/// neither ever disagrees with the other about what a prompt on the same row
/// shows. @spec TUI-NAV-067, TUI-RENDER-021
pub(crate) fn count_prompt_preview(state: &GameState, row: &Row, digits: &str) -> String {
    let comps = prompt_preview_components(state, row, digits);
    prompt_text(&prompt_preview_parts(&comps), digits)
}

/// A row's stable display name — a later row's own label (TUI-NAV-089) where
/// it has one, the content name where the row is content, or a readable label
/// invented from the row's own fixed key otherwise.
/// @spec TUI-RENDER-001, TUI-NAV-089
fn row_label(state: &GameState, row: &Row) -> String {
    if let Some(label) = crate::nav::rows::later_row_label(&row.key) {
        return label.to_string();
    }
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

/// Whether an armed row is still one the frame should draw as armed: not past
/// its expiry. Whether its row is still listed is answered simply by whether
/// it turns up in the section being iterated, so nothing beyond the clock is
/// checked here. @spec TUI-NAV-076, TUI-NAV-077
pub(crate) fn armed_is_live(armed: &ArmedRow) -> bool {
    std::time::Instant::now() < armed.expires_at
}

/// A figure the core reports, in the exact form — a summary is read, not
/// glanced at, so nothing in it is abbreviated (TUI-RENDER-012) — or, when it
/// is not a finite number, the plain words that say so instead of printing
/// it. A cost or a stock this far past any real one overflows to infinity,
/// and a bound the core has not yet reached (or a value fed by one, such as
/// elder wisdom left non-finite by a test) can come back not a number at
/// all; either prints as gibberish, so both are said in words instead.
/// @spec TUI-NAV-075, TUI-RENDER-012
fn figure(value: f64) -> String {
    fmt::exact(value)
}

/// The consequence summary an armed heavy row expands to: only what the core
/// reports from current state, re-derived every time this is called, and
/// with no consequence worked out by a rule of its own — never a judgement
/// of whether the commit will be accepted, which is the core's to make at
/// the commit. @spec TUI-NAV-075
pub(crate) fn consequence_summary(state: &GameState, armed: &ArmedRow) -> Vec<String> {
    // One fact per line, kept short: the list pane's width can be as narrow
    // as the floor's 60 columns, and a single long sentence would either wrap
    // (this pane never does) or clip a figure mid-digit.
    match armed.command {
        Command::GreatMeadowMove => {
            let kittens = state.kittens.len();
            let basket = figure(state.basket_capacity() as f64);
            let share = figure(state.memory_chest_ratio() * 100.0);
            vec![
                "Move to a new meadow:".to_string(),
                format!("{kittens} kittens come along"),
                format!("room for {basket} in the basket"),
                format!("the chest carries {share}% of stores"),
            ]
        }
        Command::PassItDown => {
            vec![format!("Pass down {} elder wisdom", figure(state.elder_wisdom))]
        }
        Command::AdoptRule(rule) => {
            let sibling = content::rule_def(content::rule_def(rule).blocks).name;
            vec![format!("Closes {sibling} for this meadow")]
        }
        Command::DeepNap(years) => {
            let cost = figure(state.starshard_cost(years));
            let wake = figure((state.year() + years as u64) as f64);
            vec![format!("Nap {years} years"), format!("costs {cost} starshards"), format!("wakes in year {wake}")]
        }
        _ => Vec::new(),
    }
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
/// list) never previews on the wrong line. An armed row's consequence
/// summary, likewise, expands only on the row an arming names, and only
/// while it is still live — never a stale one past its expiry.
/// @spec TUI-RENDER-009, TUI-RENDER-010, TUI-RENDER-011, TUI-RENDER-015,
/// TUI-NAV-067, TUI-NAV-077, TUI-THEME-011
fn draw_list(
    f: &mut Frame,
    area: Rect,
    state: &GameState,
    section: Section,
    cursor_key: Option<&RowKey>,
    prompt: Option<&CountPrompt>,
    armed: Option<&ArmedRow>,
) {
    let all = rows(state, section);
    let cursor = cursor_key
        .and_then(|k| all.iter().position(|r| &r.key == k))
        .unwrap_or(0);
    let prompt_key = prompt.and_then(|p| p.row.as_ref());
    let armed = armed.filter(|a| armed_is_live(a));

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
        lines.push(row_line(state, row, focused, area.width as usize, section, on_this_row));
        // The row an arming holds expands in place, on the lines directly
        // beneath it — never anywhere else, and only while the arming still
        // holds. The expansion carries attention's colour so the row's own
        // line can keep its availability marker's colour undisturbed.
        // @spec TUI-NAV-015, TUI-NAV-077, TUI-THEME-011
        if armed.is_some_and(|a| a.key == row.key) {
            for line in consequence_summary(state, armed.expect("checked Some above")) {
                lines.push(Line::from(Span::styled(
                    fmt::fit(&format!("  {line}"), area.width as usize),
                    style_for(Role::Attention),
                )));
            }
        }
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
/// name, owned count, next cost — the count and cost each in a field of
/// fixed width at the row's right, so the columns stand at one column
/// whatever is listed, whatever the figures (TUI-RENDER-020). While a count
/// prompt is open on this row, its count and exact preview take the cost
/// field's place instead, moving the name (and, where it stands, the count
/// field) left by what the longer text needs (TUI-RENDER-021).
///
/// Colour is the *second* channel throughout — every marker is a distinct
/// glyph before it is a colour, so the row reads the same to someone who
/// cannot tell the colours apart. Focus is the cursor glyph plus reverse
/// video, never a palette colour: the row's colour already says what its
/// state is, and a second colour would be saying something else at the same
/// time. @spec TUI-NAV-009, TUI-NAV-021, TUI-NAV-022, TUI-THEME-006,
/// TUI-THEME-014, TUI-RENDER-020, TUI-RENDER-021
fn row_line(
    state: &GameState,
    row: &Row,
    focused: bool,
    width: usize,
    section: Section,
    prompt: Option<&CountPrompt>,
) -> Line<'static> {
    let label = row_label(state, row);
    if matches!(row.kind, RowKind::Group) {
        return Line::from(Span::styled(
            fmt::fit(&format!("── {label} ──"), width),
            style_for(Role::Dim),
        ));
    }

    let (marker, role) = marker_for(state, row).unwrap_or((" ", Role::Inherited));
    let cursor = if focused { "▸ " } else { "  " };
    let counted = section_gives_count(section);
    let costed = section_gives_cost(section);
    let c = cost_field_width();

    let text = if let Some(p) = prompt {
        let fields = RowFields { width, counted, c };
        prompt_row_text(state, row, cursor, marker, &label, &fields, &p.digits)
    } else {
        let count_text = count_field_text(stated_count(state, row));
        let cost_text = cost_field_text(stated_cost(state, row).as_deref(), c);

        let mut fixed = cursor.chars().count() + 2;
        if counted {
            fixed += 1 + 7;
        }
        if costed {
            fixed += 1 + c;
        }
        let name_w = width.saturating_sub(fixed).max(4);
        let name = fmt::fit(&label, name_w);

        let mut text = format!("{cursor}{marker} {name:<name_w$}");
        if counted {
            text.push(' ');
            text.push_str(&count_text);
        }
        if costed {
            text.push(' ');
            text.push_str(&cost_text);
        }
        text
    };

    let mut style = style_for(role);
    if focused {
        // Reverse the terminal's own pair — works in either polarity, and
        // needs no colour of its own. @spec TUI-THEME-007
        style = style.add_modifier(Modifier::REVERSED);
    }
    Line::from(Span::styled(fmt::fit(&text, width), style))
}

/// A row's fixed-width fields (TUI-RENDER-020): the pane's width, whether its
/// section gives an owned-count field, and the cost field's width — the
/// context `prompt_row_text` needs to work out where a prompt's text begins
/// and how far it may move the name.
struct RowFields {
    width: usize,
    counted: bool,
    c: usize,
}

/// The row an open count prompt is held to, drawn as TUI-RENDER-021: its
/// count and exact preview begun at the left edge of the cost field's place
/// (or, where the section gives none, of the place one would take), the name
/// giving way to it first — down to one character and its ellipsis — and,
/// where the section counts, the count field kept whole but moved left
/// alongside it.
fn prompt_row_text(
    state: &GameState,
    row: &Row,
    cursor: &str,
    marker: &str,
    label: &str,
    fields: &RowFields,
    digits: &str,
) -> String {
    let &RowFields { width, counted, c } = fields;
    let comps = prompt_preview_components(state, row, digits);
    let parts = prompt_preview_parts(&comps);
    let whole = prompt_text(&parts, digits);
    let len = whole.chars().count();

    let reserved = cursor.chars().count() + 2 + if counted { 1 + 7 } else { 0 } + 1;
    let base_name = width.saturating_sub(reserved).saturating_sub(c);

    let (name_w, field) = if len <= c {
        (base_name, format!("{whole:<c$}"))
    } else if base_name >= 2 + (len - c) {
        (base_name - (len - c), whole)
    } else {
        let place = c + base_name.saturating_sub(2);
        (2, format!("{:<place$}", prompt_text_cut(&parts, digits, place)))
    };

    let name = fmt::fit(label, name_w);
    let mut text = format!("{cursor}{marker} {name:<name_w$} ");
    if counted {
        text.push_str(&count_field_text(stated_count(state, row)));
        text.push(' ');
    }
    text.push_str(&field);
    text
}

/// The space a narrow body's single pane leaves once its divider — taken
/// from its edge facing the sidebar — is set aside, so whatever draws there
/// fits its lines to that width rather than being drawn full width and cut.
fn left_of_divider(main: Rect) -> Rect {
    Rect::new(main.x + 1, main.y, main.width.saturating_sub(1), main.height)
}

/// The list's space once both its dividers — taken from its edges facing the
/// sidebar and the inspector — are set aside.
fn left_between_dividers(list: Rect) -> Rect {
    Rect::new(list.x + 1, list.y, list.width.saturating_sub(2), list.height)
}

/// Draw a pane divider — a `│` in the season's accent, and nothing else in
/// its cells — down each named column, the panes' rows only. Taken from the
/// list's space rather than given a column of its own, so the layout algebra
/// is untouched.
fn draw_dividers(f: &mut Frame, columns: &[u16], top: u16, height: u16, season: clowder_core::Season) {
    let accent = theme::seasonal_accent(season);
    let buf = f.buffer_mut();
    for &x in columns {
        for y in top..top + height {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_symbol("│");
                cell.set_style(Style::default().fg(accent));
            }
        }
    }
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
        widgets::overlays::too_small(f, rect);
        return layout;
    }

    let section = view.section.unwrap_or(DEFAULT_SECTION);
    let cursor_key = view.focus.get(section.index()).and_then(|k| k.as_ref());

    // Each region is drawn by the widget that owns it. This function composes;
    // it decides nothing about what any region contains.
    widgets::chrome::header(f, layout.header, state, view);
    // The header rule: the row's bottom edge, drawn in place rather than
    // given a row of its own, in the snapshot's season's accent — the
    // underline alone, so every character above it keeps the colour the
    // header gave it. @spec TUI-RENDER-016, TUI-THEME-008
    f.buffer_mut().set_style(
        layout.header,
        Style::default().add_modifier(Modifier::UNDERLINED).underline_color(theme::seasonal_accent(state.season())),
    );
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
        // @spec TUI-RENDER-016
        None => match layout.body {
            Body::TooSmall(_) => unreachable!("handled above"),
            Body::Narrow { sidebar, main } => {
                widgets::chrome::sidebar(f, sidebar, state);
                let space = left_of_divider(main);
                // TUI-NAV-035: swapped in over the list, the inspector still
                // follows the underlying cursor.
                if view.inspector_wanted {
                    widgets::inspector::draw(f, space, state, view);
                } else {
                    draw_list(f, space, state, section, cursor_key, view.prompt.as_ref(), view.armed.as_ref());
                }
                draw_dividers(f, &[main.x], main.y, main.height, state.season());
            }
            Body::Wide { sidebar, list, inspector } => {
                widgets::chrome::sidebar(f, sidebar, state);
                let space = left_between_dividers(list);
                draw_list(f, space, state, section, cursor_key, view.prompt.as_ref(), view.armed.as_ref());
                // TUI-NAV-035's count-prompt substitution belongs to the
                // narrow-mode swap alone — "while the inspector holds the
                // list's space" — so the persistent pane never loses its next
                // cost or lock reason to an open prompt on the same row.
                let pane_view = ViewState { prompt: None, ..view.clone() };
                widgets::inspector::draw(f, inspector, state, &pane_view);
                draw_dividers(f, &[list.x, list.right() - 1], list.y, list.height, state.season());
            }
        },
    }

    widgets::overlays::ticker(f, layout.ticker, state);
    // A footer note takes the hint bar's row while it lasts: it is the reply
    // to what the player just did, and it belongs where they are already
    // looking. It does not want the player — it is ordinary prose, not one of
    // attention's three wearers — so it is drawn in the inherited foreground,
    // never attention. @spec TUI-NAV-014, TUI-THEME-015
    match view.footer.as_ref().filter(|(_, until)| *until > std::time::Instant::now()) {
        Some((note, _)) => f.render_widget(
            Paragraph::new(Line::from(Span::raw(fmt::fit(note, layout.hints.width as usize)))),
            layout.hints,
        ),
        None => widgets::chrome::hints(f, layout.hints, state, view),
    }

    layout
}
