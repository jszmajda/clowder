//! Drawing. A frame is a pure function of (snapshot, view state, size):
//! nothing cached between frames, no game value mirrored shell-side, and no
//! decision depending on what was drawn last. That is what makes layout
//! testable against an in-memory buffer at an exact size, which in turn is
//! what makes the size-dependent navigation specs verifiable rather than
//! hoped-for. @spec TUI-RENDER-001

pub mod fmt;
pub mod layout;
pub mod pointer;
pub(crate) mod portrait;
pub mod widgets;

use clowder_core::{content, Command, ContentId, GameState, Location, LockReason, ResourceId};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::nav::rows::{Row, RowKey, RowKind};
use crate::nav::{ArmedRow, CountPrompt, Overlay, Section, ViewState};
use crate::theme::{self, Role};

pub use layout::{Body, Layout};
use pointer::PointerRecord;

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

/// A run of text on a line: plain, or a resource's name, which wears its coat
/// where the line lets it. A name piece carries the whole name's character
/// count, so a name cut short (TUI-RENDER-014) keeps each character's colour
/// by its position in the whole name.
#[derive(Debug, Clone)]
pub(crate) struct Piece {
    text: String,
    coat: Option<(ResourceId, usize)>,
    /// A state colour worn by this run alone, as the `short` tag wears
    /// attention (TUI-THEME-015) — on a line that lets colour show.
    role: Option<Role>,
}

impl Piece {
    pub(crate) fn plain(text: impl Into<String>) -> Self {
        Piece {
            text: text.into(),
            coat: None,
            role: None,
        }
    }

    /// Words wearing one state colour.
    pub(crate) fn role(text: impl Into<String>, role: Role) -> Self {
        Piece {
            text: text.into(),
            coat: None,
            role: Some(role),
        }
    }

    /// A resource's name as drawn — whole, or cut with its ellipsis — and the
    /// character count of the whole name.
    pub(crate) fn name(r: ResourceId, drawn: impl Into<String>, full_chars: usize) -> Self {
        Piece {
            text: drawn.into(),
            coat: Some((r, full_chars)),
            role: None,
        }
    }
}

/// Text whose resource names are marked as a blurb marks them
/// (`content::blurb_parts`), as pieces: each marked name in its coat across
/// its words as written, the rest plain. @spec TUI-THEME-023
pub(crate) fn marked_pieces(text: &str) -> Vec<Piece> {
    content::blurb_parts(text)
        .into_iter()
        .map(|part| match part {
            content::BlurbPart::Text(t) => Piece::plain(t),
            content::BlurbPart::Resource(r, t) => Piece::name(r, t, t.chars().count()),
        })
        .collect()
}

/// The pieces' text, run together.
pub(crate) fn pieces_text(pieces: &[Piece]) -> String {
    pieces.iter().map(|p| p.text.as_str()).collect()
}

/// One line from its pieces, fitted to `width` exactly as `fmt::fit` fits a
/// string, every character in `base` — and, where `coats` is on, each
/// character of a resource's name in its coat's colour by its position in the
/// whole name, at full strength whatever faintness `base` carries, the
/// figures, separators, and blanks beside it keeping `base`. An ellipsis that
/// cuts a name wears the colour of the character at its position. A line whose
/// colour says one thing whole — a dim row, attention, a reversal — passes
/// `coats` off, and every character keeps `base`.
/// @spec TUI-THEME-019, TUI-THEME-020, TUI-THEME-021, TUI-THEME-023
pub(crate) fn line_from_pieces(
    pieces: &[Piece],
    base: Style,
    coats: bool,
    width: usize,
) -> Line<'static> {
    let mut cells: Vec<(char, Style)> = Vec::new();
    for piece in pieces {
        match piece.coat {
            Some((r, full)) if coats => {
                let colours = crate::theme::coat_colours(crate::theme::coat(r), full);
                for (i, c) in piece.text.chars().enumerate() {
                    let style = colours.get(i).map_or(base, |colour| {
                        base.fg(*colour).remove_modifier(Modifier::DIM)
                    });
                    cells.push((c, style));
                }
            }
            _ => match piece.role.and_then(|r| r.color()) {
                Some(colour) if coats => {
                    let style = base.fg(colour).remove_modifier(Modifier::DIM);
                    cells.extend(piece.text.chars().map(|c| (c, style)));
                }
                _ => cells.extend(piece.text.chars().map(|c| (c, base))),
            },
        }
    }

    let total: usize = cells.iter().map(|(c, _)| fmt::char_width(*c)).sum();
    if total > width {
        let ellipsis = '\u{2026}';
        let ellipsis_width = fmt::char_width(ellipsis);
        let budget = width.saturating_sub(ellipsis_width);
        let (mut kept, mut used) = (0usize, 0usize);
        for (c, _) in &cells {
            let cw = fmt::char_width(*c);
            if used + cw > budget {
                break;
            }
            kept += 1;
            used += cw;
        }
        if kept > 0 && used + ellipsis_width <= width {
            let replaced = cells[kept].1;
            cells.truncate(kept);
            cells.push((ellipsis, replaced));
        } else {
            let (mut kept, mut used) = (0usize, 0usize);
            for (c, _) in &cells {
                let cw = fmt::char_width(*c);
                if used + cw > width {
                    break;
                }
                kept += 1;
                used += cw;
            }
            cells.truncate(kept);
        }
    }

    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut run = String::new();
    let mut run_style: Option<Style> = None;
    for (c, style) in cells {
        if run_style != Some(style) {
            if let Some(s) = run_style {
                spans.push(Span::styled(std::mem::take(&mut run), s));
            }
            run_style = Some(style);
        }
        run.push(c);
    }
    if let Some(s) = run_style {
        spans.push(Span::styled(run, s));
    }
    Line::from(spans)
}

/// A row's name as a piece: a stores row's name wears its coat, any other
/// row's name is plain. @spec TUI-THEME-019
fn name_piece(row: &Row, drawn: &str, full: &str) -> Piece {
    match row.key {
        RowKey::Resource(r) => Piece::name(r, drawn, full.chars().count()),
        _ => Piece::plain(drawn),
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
    // A craft's tinkerer count row binds rather than makes: its marker is the
    // bind's, whatever the craft's own lock reason, and a bind has no price
    // to cover in part. @spec TUI-NAV-053
    if *tag == "tinker" {
        let ContentId::Craft(c) = id else {
            return Some(("●", Role::Ready));
        };
        return Some(match state.tinkerer_bind_reason(*c) {
            None => ("●", Role::Ready),
            Some(_) => ("○", Role::Dim),
        });
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
        ResourceId::ALL
            .iter()
            .map(|r| resource_label(*r).chars().count())
            .max()
            .unwrap_or(0)
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
    matches!(
        section,
        Section::Meadow | Section::Clowder | Section::Crafts | Section::Sunbeams
    )
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
        // A letter's price is the core's Pen Pal query's. @spec TUI-NAV-171
        (RowKey::Colony(c, "penpal"), _) => vec![(ResourceId::Charm, state.pen_pal(*c)?.price)],
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

/// The cost field's pieces: the price's components in the core's order, each
/// a compact figure, a space, and the resource's stores-row name — the name
/// a piece of its own so it can wear its coat — apart by `, `, the first
/// figure right-aligned in six columns; the longest leading run of components
/// that fits the field, with `, …` after it where any is left out; blank to
/// the field's width. @spec TUI-RENDER-020, TUI-THEME-019
fn cost_field_pieces(cost: Option<&[(ResourceId, f64)]>, width: usize) -> Vec<Piece> {
    let Some(cost) = cost else {
        return vec![Piece::plain(" ".repeat(width))];
    };
    let components: Vec<[Piece; 2]> = cost
        .iter()
        .enumerate()
        .map(|(i, (r, a))| {
            let figure = if i == 0 {
                format!("{:>6} ", fmt::compact(*a))
            } else {
                format!("{} ", fmt::compact(*a))
            };
            let name = resource_label(*r);
            let chars = name.chars().count();
            [Piece::plain(figure), Piece::name(*r, name, chars)]
        })
        .collect();
    let run_len = |k: usize| -> usize {
        components[..k]
            .iter()
            .map(|c| c[0].text.chars().count() + c[1].text.chars().count())
            .sum::<usize>()
            + 2 * k.saturating_sub(1)
    };
    let mut out: Vec<Piece> = Vec::new();
    for k in (1..=components.len()).rev() {
        let tail = if k < components.len() { ", …" } else { "" };
        if run_len(k) + tail.chars().count() <= width {
            for (i, component) in components.iter().take(k).enumerate() {
                if i > 0 {
                    out.push(Piece::plain(", "));
                }
                out.push(component[0].clone());
                out.push(component[1].clone());
            }
            if !tail.is_empty() {
                out.push(Piece::plain(tail));
            }
            break;
        }
    }
    let used: usize = out.iter().map(|p| p.text.chars().count()).sum();
    if used < width {
        out.push(Piece::plain(" ".repeat(width - used)));
    }
    out
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
        RowKind::Act {
            semantics: CountSemantics::RepeatN,
            ..
        } => match row.key {
            RowKey::Content(id, _) => state.cost_of_next(id, n),
            _ => Vec::new(),
        },
        RowKind::Act {
            command,
            semantics: CountSemantics::CountTaking,
        } if !row.heavy => state
            .unit_price(command)
            .map(|unit| {
                unit.into_iter()
                    .map(|(r, each)| (r, each * n as f64))
                    .collect()
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// The components a preview prices, each as its figure in the exact form and
/// the resource it names. @spec TUI-NAV-067
fn prompt_preview_parts(comps: &[(ResourceId, f64)]) -> Vec<(String, ResourceId)> {
    comps.iter().map(|(r, a)| (fmt::exact(*a), *r)).collect()
}

/// The prompt's pieces: `×` joined to the digits exactly as typed; then,
/// where the preview prices components, two blank columns and the first
/// `keep` of them (all, with `None`), each its figure, a space, and the
/// resource's name as a piece of its own, apart by `, `, with `, …` after
/// them where any is left out — or `…` alone after the blank columns where
/// none is kept. @spec TUI-NAV-067, TUI-RENDER-021, TUI-THEME-019
fn prompt_pieces(parts: &[(String, ResourceId)], digits: &str, keep: Option<usize>) -> Vec<Piece> {
    let mut out = vec![Piece::plain(format!("×{digits}"))];
    if parts.is_empty() {
        return out;
    }
    let k = keep.unwrap_or(parts.len());
    out.push(Piece::plain("  "));
    for (i, (figure, r)) in parts.iter().take(k).enumerate() {
        if i > 0 {
            out.push(Piece::plain(", "));
        }
        out.push(Piece::plain(format!("{figure} ")));
        let name = resource_label(*r);
        let chars = name.chars().count();
        out.push(Piece::name(*r, name, chars));
    }
    if k < parts.len() {
        out.push(Piece::plain(if k == 0 { "…" } else { ", …" }));
    }
    out
}

/// The prompt's whole text, as `prompt_pieces` lays it out.
fn prompt_text(parts: &[(String, ResourceId)], digits: &str) -> String {
    pieces_text(&prompt_pieces(parts, digits, None))
}

/// The prompt's pieces cut to fit `place` columns: whole where it fits; else
/// the count whole and the longest leading run of components that fits,
/// `, …` after it; else the count, two blank columns, and `…`.
/// @spec TUI-RENDER-021
fn prompt_pieces_cut(parts: &[(String, ResourceId)], digits: &str, place: usize) -> Vec<Piece> {
    let whole = prompt_pieces(parts, digits, None);
    if pieces_text(&whole).chars().count() <= place {
        return whole;
    }
    for k in (1..parts.len()).rev() {
        let run = prompt_pieces(parts, digits, Some(k));
        if pieces_text(&run).chars().count() <= place {
            return run;
        }
    }
    prompt_pieces(parts, digits, Some(0))
}

/// The preview as pieces, each component's name a piece of its own so it can
/// wear its coat wherever the preview is drawn. @spec TUI-NAV-067, TUI-THEME-019
pub(crate) fn count_prompt_preview_pieces(
    state: &GameState,
    row: &Row,
    digits: &str,
) -> Vec<Piece> {
    let comps = prompt_preview_components(state, row, digits);
    prompt_pieces(&prompt_preview_parts(&comps), digits, None)
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
            let name = state
                .kittens
                .get(*ix)
                .map(|k| k.name.as_str())
                .unwrap_or("");
            if name.is_empty() {
                format!("Kitten {}", ix + 1)
            } else {
                name.to_string()
            }
        }
        RowKey::Bench => crate::nav::rows::BENCH_NAME.to_string(),
        RowKey::Energy => crate::nav::rows::ENERGY_NAME.to_string(),
        RowKey::Mess => crate::nav::rows::MESS_NAME.to_string(),
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
            vec![format!(
                "Pass down {} elder wisdom",
                figure(state.elder_wisdom)
            )]
        }
        Command::AdoptRule(rule) => {
            let sibling = content::rule_def(content::rule_def(rule).blocks).name;
            vec![format!("Closes {sibling} for this meadow")]
        }
        Command::DeepNap(years) => {
            let cost = figure(state.starshard_cost(years));
            let wake = figure((state.year() + years as u64) as f64);
            vec![
                format!("Nap {years} years"),
                format!("costs {cost} starshards"),
                format!("wakes in year {wake}"),
            ]
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
        return Window {
            start: 0,
            len: 0,
            hidden: total,
        };
    }
    if total <= capacity {
        return Window {
            start: 0,
            len: total,
            hidden: 0,
        };
    }

    // One row of the pane is reserved for the overflow note, so the rows
    // themselves get `capacity - 1`. @spec TUI-RENDER-011
    let usable = capacity.saturating_sub(1).max(1);
    const MARGIN: usize = 3;

    let max_start = total - usable;
    let start = cursor.saturating_sub(MARGIN).min(max_start);
    let hidden = total - (start + usable);
    Window {
        start,
        len: usable,
        hidden,
    }
}

/// Render one section's rows into a pane, deriving the scroll window from the
/// cursor. An open count prompt's preview draws on the exact row it was
/// opened on, by that row's identity, never by list position — so a cursor
/// that has since drifted (or an open prompt whose own row has left the
/// list) never previews on the wrong line. An armed row's consequence
/// summary, likewise, expands only on the row an arming names, and only
/// while it is still live — never a stale one past its expiry.
/// @spec TUI-RENDER-009, TUI-RENDER-010, TUI-RENDER-011, TUI-RENDER-015, TUI-RENDER-025,
/// TUI-NAV-067, TUI-NAV-077, TUI-THEME-011
fn draw_list(
    f: &mut Frame,
    area: Rect,
    state: &GameState,
    section: Section,
    view: &ViewState,
) -> Vec<(Rect, RowKey)> {
    let cursor_key = view.focus.get(section.index()).and_then(|k| k.as_ref());
    let prompt = view.prompt.as_ref();
    let armed = view.armed.as_ref();
    let all = crate::nav::rows::drawn_rows(state, section, view);
    // The list never highlights a header: where the remembered row has left
    // the list, it falls back to the first selectable row, never index 0
    // outright. @spec TUI-NAV-032, TUI-NAV-099
    let cursor = crate::nav::rows::cursor_index(&all, cursor_key).unwrap_or(0);
    let prompt_key = prompt.and_then(|p| p.row.as_ref());
    let armed = armed.filter(|a| armed_is_live(a));

    let capacity = area.height as usize;
    let win = window_for(all.len(), cursor, capacity);

    let mut lines: Vec<Line<'static>> = Vec::with_capacity(win.len + 2);
    // Where each row drew, whole line, for the frame's pointer record.
    // @spec TUI-RENDER-025
    let mut drawn_at: Vec<(Rect, RowKey)> = Vec::new();
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
        if crate::nav::rows::is_done_header(row) {
            // @spec TUI-NAV-111
            lines.push(Line::from(Span::styled(
                fmt::done_header_line(
                    crate::nav::rows::done_count(state),
                    view.done_open,
                    area.width as usize,
                ),
                style_for(Role::Dim),
            )));
            continue;
        }
        let y = area.y as usize + lines.len();
        if row.selectable() && y < area.bottom() as usize {
            drawn_at.push((Rect::new(area.x, y as u16, area.width, 1), row.key.clone()));
        }
        lines.push(row_line(
            state,
            row,
            focused,
            area.width as usize,
            section,
            on_this_row,
        ));
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
    drawn_at
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
/// time. A kitten's row carries neither heading field; the place they would
/// take instead names the kitten's one state (TUI-RENDER-024).
/// @spec TUI-NAV-009, TUI-NAV-021, TUI-NAV-022, TUI-THEME-006,
/// TUI-THEME-014, TUI-RENDER-020, TUI-RENDER-021, TUI-RENDER-024
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
        // @spec TUI-RENDER-023
        return Line::from(Span::styled(
            fmt::group_header_line(&label, width),
            style_for(Role::Dim),
        ));
    }

    let (marker, role) = marker_for(state, row).unwrap_or((" ", Role::Inherited));
    let cursor = if focused { "▸ " } else { "  " };
    let counted = section_gives_count(section);
    let costed = section_gives_cost(section);
    let c = cost_field_width();

    let mut style = style_for(role);
    if focused {
        // Reverse the terminal's own pair — works in either polarity, and
        // needs no colour of its own. @spec TUI-THEME-007
        style = style.add_modifier(Modifier::REVERSED);
    }
    // A coat is worn where the line is inherited, ready, or partial; a dim or
    // reversed line says one thing with its whole colour, and no name on it
    // wears a coat. @spec TUI-THEME-020
    let coats = !focused && matches!(role, Role::Inherited | Role::Ready | Role::Partial);

    if let Some(p) = prompt {
        let fields = RowFields { width, counted, c };
        let pieces = prompt_row_pieces(state, row, cursor, marker, &label, &fields, &p.digits);
        return line_from_pieces(&pieces, style, coats, width);
    }

    let mut fixed = cursor.chars().count() + 2;
    if counted {
        fixed += 1 + 7;
    }
    if costed {
        fixed += 1 + c;
    }
    let name_w = width.saturating_sub(fixed).max(4);

    // A kitten's row states neither heading; the place its count and cost
    // fields would take instead names the kitten's one state
    // (TUI-NAV-101, TUI-RENDER-024).
    let mut suffix: Vec<Piece> = Vec::new();
    if let RowKey::Kitten(ix, _) = row.key {
        let total = fixed - (cursor.chars().count() + 2);
        let place = total.saturating_sub(1);
        suffix.push(Piece::plain(format!(
            " {:<place$}",
            kitten_state_word(state, ix)
        )));
    } else if matches!(row.key, RowKey::Energy | RowKey::Mess) {
        // The readouts state no count or cost either; their totals stand in
        // their place, as the bench's seats do. @spec TUI-RENDER-030
        let total = fixed - (cursor.chars().count() + 2);
        let place = total.saturating_sub(1);
        suffix.push(Piece::plain(" "));
        suffix.extend(readout_pieces(state, &row.key, place));
    } else if row.key == RowKey::Bench {
        // The bench states no count or cost either; its seats stand in their
        // place, as a kitten's word does. @spec TUI-RENDER-028
        let total = fixed - (cursor.chars().count() + 2);
        let place = total.saturating_sub(1);
        suffix.push(Piece::plain(format!(
            " {:<place$}",
            bench_summary(state, place)
        )));
    } else {
        if counted {
            suffix.push(Piece::plain(format!(
                " {}",
                count_field_text(stated_count(state, row))
            )));
        }
        if costed {
            suffix.push(Piece::plain(" "));
            suffix.extend(cost_field_pieces(stated_cost(state, row).as_deref(), c));
        }
    }

    // A kitten's row carries no label (TUI-NAV-093), but where the name's
    // column has room for the whole name, a blank, and the whole hint, it
    // draws that hint in the dim role — a colour the row's own focus reverse
    // never overrides — and never in part. @spec TUI-NAV-095
    let name_len = label.chars().count();
    let hint =
        kitten_elder_hint(state, row).filter(|hint| name_len + 1 + hint.chars().count() <= name_w);

    // The Elder's own row draws the mark in the row's own style instead,
    // stepping down to the glyph alone, then to nothing, by what the name's
    // column holds beside the whole name. @spec TUI-NAV-104, TUI-NAV-105, TUI-NAV-106
    if let Some(mark) = elder_mark(state, row)
        .into_iter()
        .find(|mark| name_len + 1 + mark.chars().count() <= name_w)
    {
        let trailing = name_w - name_len - 1 - mark.chars().count();
        let mut text = format!("{cursor}{marker} {label} {mark}");
        text.push_str(&" ".repeat(trailing));
        text.push_str(&pieces_text(&suffix));
        return Line::from(Span::styled(fmt::fit(&text, width), style));
    }

    let Some(hint) = hint else {
        let name = fmt::fit(&label, name_w);
        let pad = name_w.saturating_sub(name.chars().count());
        let mut pieces = vec![
            Piece::plain(format!("{cursor}{marker} ")),
            name_piece(row, &name, &label),
            Piece::plain(" ".repeat(pad)),
        ];
        pieces.extend(suffix);
        return line_from_pieces(&pieces, style, coats, width);
    };

    let hint_w = hint.chars().count();
    let trailing = name_w - name_len - 1 - hint_w;
    let mut hint_style = style_for(Role::Dim);
    if focused {
        hint_style = hint_style.add_modifier(Modifier::REVERSED);
    }
    let mut spans = vec![
        Span::styled(format!("{cursor}{marker} {label} "), style),
        Span::styled(hint.to_string(), hint_style),
    ];
    if trailing > 0 {
        spans.push(Span::styled(" ".repeat(trailing), style));
    }
    if !suffix.is_empty() {
        spans.push(Span::styled(pieces_text(&suffix), style));
    }
    Line::from(spans)
}

/// The `(n: Elder)` hint a listed kitten row carries after its name while the
/// core reports the command gate of AppointElder for that kitten open — never
/// drawn at all where the gate is shut. Whether the row's own column has room
/// for it is the caller's to judge (TUI-NAV-095); this states only whether the
/// row earns the hint at all.
/// @spec TUI-NAV-095
fn kitten_elder_hint(state: &GameState, row: &Row) -> Option<&'static str> {
    let RowKey::Kitten(ix, _) = row.key else {
        return None;
    };
    if state.elder == Some(ix) {
        return None;
    }
    state
        .command_gate_open(&Command::AppointElder(ix))
        .then_some(ELDER_HINT)
}

/// The forms of the Elder's mark the row of the kitten the core holds as the
/// Elder may draw, fullest first; empty on every other row.
/// @spec TUI-NAV-104, TUI-NAV-106
fn elder_mark(state: &GameState, row: &Row) -> Vec<&'static str> {
    match row.key {
        RowKey::Kitten(ix, _) if state.elder == Some(ix) => vec![ELDER_MARK, ELDER_GLYPH],
        _ => Vec::new(),
    }
}

/// The Elder's mark: glyph, one blank column, word. @spec TUI-NAV-105
pub(crate) const ELDER_MARK: &str = "◆ Elder";
/// The mark's glyph alone, its narrower form. @spec TUI-NAV-106
const ELDER_GLYPH: &str = "◆";

/// The row-bound hint a kitten's row carries after its name, whole or not at
/// all. @spec TUI-NAV-095
const ELDER_HINT: &str = "(n: Elder)";

/// The word a kitten away from home reads, whatever job it holds.
const AWAY_WORD: &str = "visiting";
/// The word a kitten at home with no job reads.
const IDLE_WORD: &str = "idle";

/// The one state a listed kitten's row names, read from the kitten's location
/// and job as the core holds them and no rule of the shell's own: away from
/// home wherever it is anywhere but home, whatever job it holds; at home with
/// a job, that job's name as its definition spells it; at home with none, the
/// idle word. Says nothing of the Elder, whose own hint (TUI-NAV-095) stands
/// in the name's column, not here. @spec TUI-NAV-101
fn kitten_state_word(state: &GameState, ix: usize) -> String {
    let kitten = &state.kittens[ix];
    if kitten.location != Location::Home {
        AWAY_WORD.to_string()
    } else if let Some(j) = kitten.job {
        content::job_def(j).name.to_string()
    } else {
        IDLE_WORD.to_string()
    }
}

/// The tinkerers' bench at a glance: the bindings, the seats, and — where any
/// Tinkerer waits — how many, as the core's bench query reports them. Where
/// the place cannot hold it all, the waiting clause gives way whole, the
/// inspector naming every waiting kitten; the seats are cut only as a name is.
/// @spec TUI-NAV-157, TUI-RENDER-028
fn bench_summary(state: &GameState, place: usize) -> String {
    let bench = state.bench();
    let seats = format!("{}/{} seats taken", bench.seated.len(), bench.seats);
    let full = if bench.waiting.is_empty() {
        seats.clone()
    } else {
        format!("{seats} · {} waiting", bench.waiting.len())
    };
    if full.chars().count() <= place {
        full
    } else {
        fmt::fit(&seats, place)
    }
}

/// The text a readout row draws in its fields' place, fitted to `place`:
/// the energy made and used, and ` · short` with `short` in attention while
/// the meadow uses more than it makes, the clause kept and the figures cut
/// where the place is short; or the mess level, `tidy` with none.
/// @spec TUI-NAV-168, TUI-RENDER-030, TUI-THEME-015
fn readout_pieces(state: &GameState, key: &RowKey, place: usize) -> Vec<Piece> {
    let pad = |used: usize| Piece::plain(" ".repeat(place.saturating_sub(used)));
    if *key == RowKey::Mess {
        let text = match state.mess_report().level {
            None => "tidy".to_string(),
            Some(l) => format!("level {l}"),
        };
        let text = fmt::fit(&text, place);
        let used = text.chars().count();
        return vec![Piece::plain(text), pad(used)];
    }
    let (made, used) = (state.energy_production(), state.energy_consumption());
    let figures = format!("{} made · {} used", fmt::compact(made), fmt::compact(used));
    if used > made {
        let clause = " · short";
        let room = place.saturating_sub(clause.chars().count());
        let figures = fmt::fit(&figures, room);
        let n = figures.chars().count() + clause.chars().count();
        vec![
            Piece::plain(figures),
            Piece::plain(" · "),
            Piece::role("short", Role::Attention),
            pad(n),
        ]
    } else {
        let figures = fmt::fit(&figures, place);
        let n = figures.chars().count();
        vec![Piece::plain(figures), pad(n)]
    }
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

/// The row's pieces while a count prompt is open on it: cursor, marker, the
/// name fitted to what the prompt's text leaves it, the count field where
/// the list gives one, and the prompt's count and preview in the cost field's
/// place (TUI-RENDER-021). @spec TUI-RENDER-021
fn prompt_row_pieces(
    state: &GameState,
    row: &Row,
    cursor: &str,
    marker: &str,
    label: &str,
    fields: &RowFields,
    digits: &str,
) -> Vec<Piece> {
    let &RowFields { width, counted, c } = fields;
    let comps = prompt_preview_components(state, row, digits);
    let parts = prompt_preview_parts(&comps);
    let whole = prompt_text(&parts, digits);
    let len = whole.chars().count();

    let reserved = cursor.chars().count() + 2 + if counted { 1 + 7 } else { 0 } + 1;
    let base_name = width.saturating_sub(reserved).saturating_sub(c);

    let (name_w, mut field) = if len <= c {
        let mut field = prompt_pieces(&parts, digits, None);
        field.push(Piece::plain(" ".repeat(c - len)));
        (base_name, field)
    } else if base_name >= 2 + (len - c) {
        (base_name - (len - c), prompt_pieces(&parts, digits, None))
    } else {
        let place = c + base_name.saturating_sub(2);
        let mut field = prompt_pieces_cut(&parts, digits, place);
        let used = pieces_text(&field).chars().count();
        field.push(Piece::plain(" ".repeat(place.saturating_sub(used))));
        (2, field)
    };

    let name = fmt::fit(label, name_w);
    let pad = name_w.saturating_sub(name.chars().count());
    let mut out = vec![
        Piece::plain(format!("{cursor}{marker} ")),
        name_piece(row, &name, label),
    ];
    out.push(Piece::plain(format!("{} ", " ".repeat(pad))));
    if counted {
        out.push(Piece::plain(count_field_text(stated_count(state, row))));
        out.push(Piece::plain(" "));
    }
    out.append(&mut field);
    out
}

/// The space a narrow body's single pane leaves once its divider — taken
/// from its edge facing the sidebar — is set aside, so whatever draws there
/// fits its lines to that width rather than being drawn full width and cut.
fn left_of_divider(main: Rect) -> Rect {
    Rect::new(
        main.x + 1,
        main.y,
        main.width.saturating_sub(1),
        main.height,
    )
}

/// The list's space once both its dividers — taken from its edges facing the
/// sidebar and the inspector — are set aside.
fn left_between_dividers(list: Rect) -> Rect {
    Rect::new(
        list.x + 1,
        list.y,
        list.width.saturating_sub(2),
        list.height,
    )
}

/// Draw a pane divider — a `│` in the season's accent, and nothing else in
/// its cells — down each named column, the panes' rows only. Taken from the
/// list's space rather than given a column of its own, so the layout algebra
/// is untouched.
fn draw_dividers(
    f: &mut Frame,
    columns: &[u16],
    top: u16,
    height: u16,
    season: clowder_core::Season,
) {
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

/// The waking screen: three lines in place of the whole frame, drawn from the
/// catch-up's total and remaining counts and the terminal's size alone —
/// nothing about the absence itself. Below the floor, the too-small card
/// stands in, as it does for the game's own frame, and the replay carries on
/// beneath it regardless of which is drawn.
/// @spec TUI-NAV-102
pub fn waking(f: &mut Frame, total_ticks: u64, remaining_ticks: u64) {
    waking_recorded(f, total_ticks, remaining_ticks);
}

/// The waking screen as `waking` draws it, with its pointer record: it draws
/// nothing a pointer can address, so the record names no target.
/// @spec TUI-RENDER-026
pub fn waking_recorded(f: &mut Frame, total_ticks: u64, remaining_ticks: u64) -> PointerRecord {
    let area = f.area();
    let record = PointerRecord::blank((area.width, area.height));
    if let Body::TooSmall(rect) = layout::compute(area, None, false).body {
        widgets::overlays::too_small(f, rect);
        return record;
    }

    // Both round down, so neither reads full until waking ends.
    let replayed = total_ticks.saturating_sub(remaining_ticks);
    let total = total_ticks.max(1);
    let filled = (40 * replayed / total) as usize;
    let percent = 100 * replayed / total;
    let bar: String = "\u{2588}".repeat(filled) + &"\u{2591}".repeat(40 - filled);

    let calm = "the clowder is stretching awake\u{2026}";
    let bar_line = format!("{bar}   {percent}%");
    let hint = "q save & quit \u{b7} the meadow finishes waking next time";
    let lines = [
        (calm, Role::Dim),
        (bar_line.as_str(), Role::Inherited),
        (hint, Role::Dim),
    ];

    // Centred in the pane, one blank row between each. @spec TUI-NAV-102
    let block_h = 5u16;
    let top = area.y + area.height.saturating_sub(block_h) / 2;
    for (i, (text, role)) in lines.iter().enumerate() {
        let y = top + i as u16 * 2;
        if y >= area.y + area.height {
            continue;
        }
        let w = (text.chars().count() as u16).min(area.width);
        let x = area.x + (area.width.saturating_sub(w)) / 2;
        let rect = Rect::new(x, y, w, 1);
        f.render_widget(
            Paragraph::new(Line::styled(text.to_string(), style_for(*role))),
            rect,
        );
    }
    record
}

/// Draw one frame, returning the layout it used.
///
/// The layout must escape: the breakpoints' dead-band is a function of the
/// *previous* frame's layout, so a frame that computed one and dropped it
/// would leave the next frame unable to tell which side of a boundary it was
/// already on. @spec TUI-RENDER-001, TUI-RENDER-002, TUI-RENDER-008
pub fn frame(f: &mut Frame, state: &GameState, view: &ViewState, prev: Option<&Layout>) -> Layout {
    frame_recorded(f, state, view, prev).0
}

/// Draw one frame as `frame` does, and with it the record of where it drew
/// what a pointer can address. The record is a second output, never an input:
/// drawing consults none of an earlier frame. @spec TUI-RENDER-025,
/// TUI-RENDER-026
pub fn frame_recorded(
    f: &mut Frame,
    state: &GameState,
    view: &ViewState,
    prev: Option<&Layout>,
) -> (Layout, PointerRecord) {
    let area = f.area();
    let mut record = PointerRecord::blank((area.width, area.height));
    // TUI-NAV-050: an unread report is what puts the banner on screen.
    let banner = view.reports.iter().any(|r| !r.dismissed);
    let layout = layout::compute(area, prev, banner);

    // @spec TUI-RENDER-004
    if let Body::TooSmall(rect) = layout.body {
        widgets::overlays::too_small(f, rect);
        return (layout, record);
    }

    let section = view.section.unwrap_or(DEFAULT_SECTION);

    // Each region is drawn by the widget that owns it. This function composes;
    // it decides nothing about what any region contains.
    widgets::chrome::header(f, layout.header, state, view);
    // The header rule: the row's bottom edge, drawn in place rather than
    // given a row of its own, in the snapshot's season's accent — the
    // underline alone, so every character above it keeps the colour the
    // header gave it. @spec TUI-RENDER-016, TUI-THEME-008
    f.buffer_mut().set_style(
        layout.header,
        Style::default()
            .add_modifier(Modifier::UNDERLINED)
            .underline_color(theme::seasonal_accent(state.season())),
    );
    record.record_tabs(widgets::chrome::tabs_recorded(f, layout.tabs, state, view));

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
        Body::Wide {
            sidebar, inspector, ..
        } => sidebar.union(inspector),
    };
    match &view.overlay {
        Some(Overlay::Journal { .. }) => {
            widgets::overlays::journal(f, body_rect, state, view);
            record.record_journal(
                body_rect,
                widgets::overlays::journal_max_scroll(state, view, body_rect.height),
            );
        }
        // The scrapbook scrolls as the Journal does, so the frame records its
        // region and reach in the same place. @spec TUI-NAV-174
        Some(Overlay::Achievements { .. }) => {
            widgets::overlays::achievements(f, body_rect, state, view);
            record.record_journal(
                body_rect,
                widgets::overlays::achievements_max_scroll(state, view, body_rect),
            );
        }
        Some(Overlay::Help) => widgets::overlays::help(f, body_rect),
        Some(Overlay::Palette { .. }) => widgets::overlays::palette(f, body_rect, state, view),
        // @spec TUI-RENDER-016
        None => match layout.body {
            Body::TooSmall(_) => unreachable!("handled above"),
            Body::Narrow { sidebar, main } => {
                record.record_sidebar(widgets::chrome::sidebar(f, sidebar, state, view));
                let space = left_of_divider(main);
                // TUI-NAV-035: swapped in over the list, the inspector still
                // follows the underlying cursor.
                if view.inspector_wanted {
                    widgets::inspector::draw(f, space, state, view);
                } else {
                    let drawn = draw_list(f, space, state, section, view);
                    record.record_list(space, section, drawn);
                }
                draw_dividers(f, &[main.x], main.y, main.height, state.season());
            }
            Body::Wide {
                sidebar,
                list,
                inspector,
            } => {
                record.record_sidebar(widgets::chrome::sidebar(f, sidebar, state, view));
                let space = left_between_dividers(list);
                let drawn = draw_list(f, space, state, section, view);
                record.record_list(space, section, drawn);
                // TUI-NAV-035's count-prompt substitution belongs to the
                // narrow-mode swap alone — "while the inspector holds the
                // list's space" — so the persistent pane never loses its next
                // cost or lock reason to an open prompt on the same row.
                let pane_view = ViewState {
                    prompt: None,
                    ..view.clone()
                };
                widgets::inspector::draw(f, inspector, state, &pane_view);
                draw_dividers(
                    f,
                    &[list.x, list.right() - 1],
                    list.y,
                    list.height,
                    state.season(),
                );
            }
        },
    }

    widgets::overlays::ticker(f, layout.ticker, state);
    // A footer note takes the hint bar's row while it lasts: it is the reply
    // to what the player just did, and it belongs where they are already
    // looking. It does not want the player — it is ordinary prose, not one of
    // attention's three wearers — so it is drawn in the inherited foreground,
    // never attention, and a store it names wears its coat, read from the mark
    // the note carries (`nav::plain_error`). @spec TUI-NAV-014, TUI-THEME-015,
    // TUI-THEME-024
    match view
        .footer
        .as_ref()
        .filter(|(_, until)| *until > std::time::Instant::now())
    {
        Some((note, _)) => f.render_widget(
            Paragraph::new(line_from_pieces(
                &marked_pieces(note),
                Style::default(),
                true,
                layout.hints.width as usize,
            )),
            layout.hints,
        ),
        // The hint bar names the sidebar's scroll keys while this frame's
        // sidebar holds stores out of view. @spec TUI-NAV-146
        None => widgets::chrome::hints(
            f,
            layout.hints,
            state,
            view,
            record.sidebar().is_some_and(|s| s.max_start.is_some()),
        ),
    }

    (layout, record)
}
