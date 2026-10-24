//! The persistent furniture: header, tab bar, resource sidebar, hint bar.
//!
//! This is what the two-second glance contract is made of. Its whole job is to
//! be quiet: an empty attention cluster means nothing needs you, and that only
//! works if the eye has learned that something appearing there is information.
//! @spec TUI-NAV-019..025, TUI-NAV-034, TUI-NAV-044, TUI-NAV-045, TUI-NAV-054

use clowder_core::{Command, GameState, Location, MoodTier, ResourceId};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::nav::mnemonics::{self, MnemonicKind};
use crate::nav::rows::{rows, tab_sections, Row, RowKey, RowKind};
use crate::nav::{Overlay, Section, ViewState};
use crate::render::pointer::SidebarRecord;
use crate::render::{fmt, style_for};
use crate::theme::Role;

// ---------------------------------------------------------------------------
// Small shared helpers
// ---------------------------------------------------------------------------

/// Turn an identifier-shaped tag into a spaced label — the same treatment
/// `render::humanize` gives a fixed row, kept local so this leaf does not
/// reach across into `mod.rs` for it.
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

/// Draw a single line of text, clipped to `area` by the Paragraph itself.
fn draw_line(f: &mut Frame, area: Rect, text: String) {
    f.render_widget(Paragraph::new(Line::from(text)), area);
}

fn mood_word(tier: MoodTier) -> &'static str {
    match tier {
        MoodTier::Blissful => "blissful",
        MoodTier::Content => "content",
        MoodTier::Restless => "restless",
        MoodTier::Fussy => "fussy",
        MoodTier::Grumpy => "grumpy",
    }
}

// ---------------------------------------------------------------------------
// Header + attention chips
// ---------------------------------------------------------------------------

/// The tabs currently on the bar that wear TUI-NAV-006's ★ — a tab appeared
/// and has not been visited since. Computed straight from the bar's own
/// membership, never from stored bookkeeping alone, so a tab that leaves the
/// bar stops counting the instant it does, with no key pressed and no frame
/// of per-frame cleanup required first. @spec TUI-NAV-006, TUI-NAV-020
fn badged_tabs(state: &GameState, view: &ViewState) -> Vec<Section> {
    tab_sections(state, view.section)
        .into_iter()
        .filter(|s| !view.seen[s.index()].visited)
        .collect()
}

/// How many tabs in the bar wear ★ right now — what `★N new` counts.
/// @spec TUI-NAV-020
fn new_content_count(state: &GameState, view: &ViewState) -> usize {
    badged_tabs(state, view).len()
}

/// Kittens with no job who are in the meadow to be assigned — the only ones
/// the idle chip may ever count. A kitten visiting the neighbour's yard is
/// just as jobless and not assignable by anybody. @spec TUI-NAV-054
fn idle_here_count(state: &GameState) -> usize {
    state
        .kittens
        .iter()
        .filter(|k| k.job.is_none() && k.location == Location::Home)
        .count()
}

/// Whether a sky event waits to be observed right now — the core's own answer,
/// so a window it has let fade is never drawn as live. @spec TUI-NAV-020
fn sky_live(state: &GameState) -> bool {
    state
        .timers()
        .iter()
        .any(|(t, _)| *t == clowder_core::Timer::SkyEvent)
}

/// The five attention opportunities, in the fixed order idle, sky, new,
/// away, energy, each live or not. Shared by the counted and uncounted chip
/// clusters so the two never drift on which chips are live. The energy chip
/// is live exactly while the core reports the meadow using more energy than
/// it makes. @spec TUI-NAV-020
fn live_opportunities(state: &GameState, view: &ViewState) -> [bool; 5] {
    [
        idle_here_count(state) > 0,
        sky_live(state),
        new_content_count(state, view) > 0,
        view.reports.iter().any(|r| !r.dismissed),
        state.energy_consumption() > state.energy_production(),
    ]
}

/// The live chips at either detail level TUI-NAV-086 steps through, each a
/// whole label — with a count between glyph and word on the two chips that
/// count, or — once the header has shed the count first — glyph directly
/// before word on every chip. Which chips are live never changes between the
/// two; only whether the count is drawn does. Each label is later drawn as
/// one span in `attention` (TUI-THEME-015): the glyph, any count, and the
/// word together, never split.
fn chip_labels(state: &GameState, view: &ViewState, with_counts: bool) -> Vec<String> {
    let live = live_opportunities(state, view);
    let idle = idle_here_count(state);
    let new = new_content_count(state, view);
    let labels = if with_counts {
        [
            format!("◦{idle} idle"),
            "!sky".to_string(),
            format!("★{new} new"),
            "~away".to_string(),
            "↯energy".to_string(),
        ]
    } else {
        [
            "◦idle".to_string(),
            "!sky".to_string(),
            "★new".to_string(),
            "~away".to_string(),
            "↯energy".to_string(),
        ]
    };
    labels
        .into_iter()
        .zip(live)
        .filter(|(_, on)| *on)
        .map(|(label, _)| label)
        .collect()
}

/// The five glance facts in full wording, joined `·`. @spec TUI-NAV-086
fn full_facts(state: &GameState) -> Vec<String> {
    vec![
        format!("Gen {}", state.generation),
        format!(
            "{:?} Day {} Year {}",
            state.season(),
            state.day(),
            state.year()
        ),
        format!("{} kittens", state.kittens.len()),
        mood_word(state.mood_tier()).to_string(),
        format!("×{}", state.speed),
    ]
}

/// The same five facts in compact wording — generation, day, and year
/// shortened to their letter, season to its first three letters, kitten count
/// and mood and speed unchanged. @spec TUI-NAV-086
fn compact_facts(state: &GameState) -> Vec<String> {
    let season: String = format!("{:?}", state.season()).chars().take(3).collect();
    vec![
        format!("G{}", state.generation),
        format!("{season} D{} Y{}", state.day(), state.year()),
        format!("{} kittens", state.kittens.len()),
        mood_word(state.mood_tier()).to_string(),
        format!("×{}", state.speed),
    ]
}

/// Facts and chips together, right-flush chips with at least one blank
/// column before them, or facts alone once the chips are empty. `None` when
/// even this pairing overruns `width`. Returns the facts-and-padding prefix
/// apart from the chips text, so the chips alone can be drawn in attention.
fn fit(facts: &str, chips: &str, width: usize) -> Option<String> {
    let fw = facts.chars().count();
    let cw = chips.chars().count();
    if chips.is_empty() {
        (fw <= width).then(|| format!("{facts}{}", " ".repeat(width - fw)))
    } else {
        (fw + 1 + cw <= width).then(|| format!("{facts}{}", " ".repeat(width - fw - cw)))
    }
}

/// The header line as spans: the facts and padding in the inherited
/// foreground, then each live chip's label whole in `attention` — its glyph,
/// any count, and its word together — with the blank columns between chips
/// left unstyled, so only what wants the player carries the colour.
/// @spec TUI-THEME-015
fn chip_line(prefix: String, labels: &[String]) -> Line<'static> {
    let mut spans = vec![Span::raw(prefix)];
    for (i, label) in labels.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(label.clone(), style_for(Role::Attention)));
    }
    Line::from(spans)
}

/// Clowder generation, date, kitten count, mood as a word, speed, and the
/// right-aligned attention chips — drawn as the first of TUI-NAV-086's steps
/// that fits the pane: the full facts with counted chips; the compact facts
/// with counted chips; the compact facts with the counting chips reduced to
/// glyph and word; the compact facts shed whole — generation, then date, then
/// kitten count — never the mood word or the speed, chips as in the previous
/// step. @spec TUI-NAV-019, TUI-NAV-020, TUI-NAV-021, TUI-NAV-022,
/// TUI-NAV-044, TUI-NAV-054, TUI-NAV-086
pub fn header(f: &mut Frame, area: Rect, state: &GameState, view: &ViewState) {
    let width = area.width as usize;
    let with_labels = chip_labels(state, view, true);
    let without_labels = chip_labels(state, view, false);
    let with_counts = with_labels.join("  ");
    let without_counts = without_labels.join("  ");
    let compact = compact_facts(state);

    let steps: [(Option<String>, &Vec<String>); 6] = [
        (
            fit(&full_facts(state).join(" · "), &with_counts, width),
            &with_labels,
        ),
        (fit(&compact.join(" "), &with_counts, width), &with_labels),
        (
            fit(&compact.join(" "), &without_counts, width),
            &without_labels,
        ),
        (
            fit(&compact[1..].join(" "), &without_counts, width),
            &without_labels,
        ),
        (
            fit(&compact[2..].join(" "), &without_counts, width),
            &without_labels,
        ),
        (
            fit(&compact[3..].join(" "), &without_counts, width),
            &without_labels,
        ),
    ];
    let (prefix, labels) = steps
        .into_iter()
        .find_map(|(prefix, labels)| prefix.map(|p| (p, labels)))
        .unwrap_or_else(|| (compact[3..].join(" "), &without_labels));
    f.render_widget(Paragraph::new(chip_line(prefix, labels)), area);
}

// ---------------------------------------------------------------------------
// Tab bar
// ---------------------------------------------------------------------------

/// Every tab's label beside the separator that joins the run: the reversal is
/// a fill, never a column, so the separators stay in the bar's own plain
/// gaps and only a label's own span carries `Modifier::REVERSED`, never a
/// palette colour, so the terminal's own pair still shows through
/// (TUI-THEME-007). @spec TUI-NAV-092
fn tab_spans(
    sections: &[Section],
    labels: &[String],
    focused: Option<Section>,
    sep: &str,
) -> Vec<Span<'static>> {
    let mut spans = Vec::with_capacity(sections.len() * 2);
    for (i, (section, label)) in sections.iter().zip(labels).enumerate() {
        if i > 0 {
            spans.push(Span::raw(sep.to_string()));
        }
        if focused == Some(*section) {
            spans.push(Span::styled(
                label.clone(),
                Style::default().add_modifier(Modifier::REVERSED),
            ));
        } else {
            spans.push(Span::raw(label.clone()));
        }
    }
    spans
}

/// Sections that list a selectable row, or hold focus, with their digits; a
/// badge on one not yet visited since it last appeared. Width-responsive:
/// full labels where they fit, else a compact digit strip — never a name
/// truncated mid-word, never a tab scrolled out of reach. The section in
/// focus is always the one tab reversed, in either form: the mark of where
/// the player is, following focus alone and never the field or full-screen
/// overlay drawn over it. @spec TUI-NAV-006, TUI-NAV-034, TUI-NAV-085,
/// TUI-NAV-092
pub fn tabs(f: &mut Frame, area: Rect, state: &GameState, view: &ViewState) {
    tabs_recorded(f, area, state, view);
}

/// The tab bar as `tabs` draws it, answering where each tab's label stands:
/// its section and the cells of its label — badge, digit and name as far as
/// the bar draws them — clipped to the bar. @spec TUI-RENDER-025
pub fn tabs_recorded(
    f: &mut Frame,
    area: Rect,
    state: &GameState,
    view: &ViewState,
) -> Vec<(Section, Rect)> {
    let revealed: Vec<Section> = tab_sections(state, view.section);
    let focused = view.section;

    let mark = |s: Section| {
        if !view.seen[s.index()].visited {
            "★"
        } else {
            ""
        }
    };

    // Full form: every revealed tab named beside its digit.
    let full: Vec<String> = revealed
        .iter()
        .map(|s| format!("{}{} {}", mark(*s), s.digit(), s.title()))
        .collect();
    let full_line = full.join("  ");

    if full_line.chars().count() <= area.width as usize {
        let spans = tab_spans(&revealed, &full, focused, "  ");
        f.render_widget(Paragraph::new(Line::from(spans)), area);
        return label_cells(area, &revealed, &full, 2);
    }

    // Compact form: every tab keeps its digit; only the focused tab and any
    // unvisited (badged) tab keep their name. Compacting drops names, never
    // reach — the digit strip never scrolls and never sheds a tab.
    let compact: Vec<String> = revealed
        .iter()
        .map(|s| {
            let badged = !view.seen[s.index()].visited;
            let is_focused = focused == Some(*s);
            if badged || is_focused {
                format!("{}{} {}", mark(*s), s.digit(), s.title())
            } else {
                s.digit().to_string()
            }
        })
        .collect();
    let spans = tab_spans(&revealed, &compact, focused, " ");
    f.render_widget(Paragraph::new(Line::from(spans)), area);
    label_cells(area, &revealed, &compact, 1)
}

/// The cells each label covers on the bar, the labels laid left to right with
/// `gap` blank columns between, the gaps belonging to no tab.
fn label_cells(
    area: Rect,
    sections: &[Section],
    labels: &[String],
    gap: u16,
) -> Vec<(Section, Rect)> {
    let mut x = area.x;
    let mut out = Vec::with_capacity(sections.len());
    for (section, label) in sections.iter().zip(labels) {
        let width = label.chars().count() as u16;
        let from = x.min(area.right());
        let to = x.saturating_add(width).min(area.right());
        if to > from {
            out.push((*section, Rect::new(from, area.y, to - from, 1)));
        }
        x = x.saturating_add(width).saturating_add(gap);
    }
    out
}

// ---------------------------------------------------------------------------
// Resource sidebar
// ---------------------------------------------------------------------------

const FILL_CELLS: usize = 4;
const FILL_FULL: char = '▓';
const FILL_EMPTY: char = '░';

/// A four-cell fill bar for a stock against its cap. An infinite (or zero)
/// cap has nothing to fill toward, so it draws empty.
fn fill_bar(stock: f64, cap: f64) -> String {
    if !cap.is_finite() || cap <= 0.0 {
        return FILL_EMPTY.to_string().repeat(FILL_CELLS);
    }
    let ratio = (stock / cap).clamp(0.0, 1.0);
    let filled = (ratio * FILL_CELLS as f64).round() as usize;
    let filled = filled.min(FILL_CELLS);
    format!(
        "{}{}",
        FILL_FULL.to_string().repeat(filled),
        FILL_EMPTY.to_string().repeat(FILL_CELLS - filled)
    )
}

/// Whether a store wears `full`: a cap above zero, reached. @spec TUI-NAV-022
fn wears_full(state: &GameState, r: ResourceId) -> bool {
    let cap = state.cap(r);
    cap.is_finite() && cap > 0.0 && state.stock(r) >= cap
}

/// The stores the stores group lists, in its order — the same order the row
/// model gives the meadow's stores group, so the sidebar can never disagree
/// with the list about which stores exist or how they are ordered.
/// @spec TUI-NAV-049
fn listed_stores(state: &GameState) -> Vec<ResourceId> {
    rows(state, Section::Meadow)
        .into_iter()
        .filter_map(|row| match row.key {
            RowKey::Resource(r) => Some(r),
            _ => None,
        })
        .collect()
}

/// A store's two lines: its name, stock, `/`, and cap at the right edge; then
/// its fill bar, its rate, and — while capped — the `full` tag. The name
/// wears its coat (TUI-THEME-019); only the `full` tag wears `attention`
/// (TUI-THEME-015); everything else, including the fill bar, stays in the
/// inherited foreground so colour never carries a meaning the glyphs do not.
/// @spec TUI-RENDER-019, TUI-NAV-022, TUI-THEME-019
fn store_lines(
    state: &GameState,
    r: ResourceId,
    name_col: usize,
    width: usize,
) -> [Line<'static>; 2] {
    let stock = state.stock(r);
    let cap = state.cap(r);
    let label = humanize(&format!("{r:?}"));
    let name = fmt::fit(&label, name_col);
    let cap_part = if cap.is_finite() {
        format!("/{:<6}", fmt::compact(cap))
    } else {
        " ".repeat(7)
    };
    // The name wears its coat — a cut name keeping each character's colour by
    // its position in the whole — and the figures beside it wear none.
    // @spec TUI-THEME-019, TUI-THEME-021
    let pad = name_col.saturating_sub(name.chars().count());
    let first = crate::render::line_from_pieces(
        &[
            crate::render::Piece::name(r, name, label.chars().count()),
            crate::render::Piece::plain(format!(
                "{} {:>6}{cap_part}",
                " ".repeat(pad),
                fmt::compact(stock)
            )),
        ],
        ratatui::style::Style::default(),
        true,
        usize::MAX,
    );

    let bar = fill_bar(stock, cap);
    let rate = fmt::rate(state.net_rate(r));
    let body = format!("{bar} {rate:>7}");
    let capped = wears_full(state, r);
    let mut spans = vec![Span::raw(body.clone())];
    let mut drawn_len = body.chars().count();
    if capped {
        spans.push(Span::raw(" "));
        spans.push(Span::styled("full", style_for(Role::Attention)));
        drawn_len += " full".len();
    }
    if drawn_len < width {
        spans.push(Span::raw(" ".repeat(width - drawn_len)));
    }
    [first, Line::from(spans)]
}

/// Whether a sidebar `height` lines tall holds stores out of view: whether
/// the stores the stores group lists outrun its lines at two lines each.
/// @spec TUI-RENDER-022
pub fn stores_out_of_view(state: &GameState, height: u16) -> bool {
    2 * listed_stores(state).len() > height as usize
}

/// Where the window onto `k` stores stands in a sidebar `h` lines tall, the
/// view wanting it to begin at place `want`: its first store's place, how
/// many stores it draws, and the furthest place its first store can stand —
/// the earliest from which every store through the last fits beneath the
/// head note — or none where every store fits.
fn window(k: usize, h: usize, want: usize) -> (usize, usize, Option<usize>) {
    if 2 * k <= h {
        return (0, k, None);
    }
    let max_start = k - h.saturating_sub(1) / 2;
    let start = want.min(max_start);
    let avail = h.saturating_sub(usize::from(start > 0));
    let drawn = if 2 * (k - start) <= avail {
        k - start
    } else {
        avail.saturating_sub(1) / 2
    };
    (start, drawn, Some(max_start))
}

/// A note counting `n` stores out of view on one side, and how many of them
/// wear `full`: `▾ 6 more · 2 full`, in `dim` save the full count, which wears
/// `attention`. Where the sidebar is narrower than the whole note, its
/// ` more` gives way. @spec TUI-RENDER-027, TUI-THEME-015
fn sidebar_note(glyph: char, n: usize, full: usize, width: usize) -> Line<'static> {
    let count = format!("{glyph} {n}");
    let full_part = (full > 0).then(|| format!("{full} full"));
    let whole = count.chars().count()
        + " more".len()
        + full_part
            .as_ref()
            .map_or(0, |f| " · ".chars().count() + f.chars().count());
    let mut spans = vec![Span::styled(count, style_for(Role::Dim))];
    if whole <= width {
        spans.push(Span::styled(" more", style_for(Role::Dim)));
    }
    if let Some(f) = full_part {
        spans.push(Span::styled(" · ", style_for(Role::Dim)));
        spans.push(Span::styled(f, style_for(Role::Attention)));
    }
    Line::from(spans)
}

/// The stores the stores group lists, each drawn whole (TUI-RENDER-019) or
/// not at all, in the stores group's order. Where the sidebar's lines cannot
/// hold every one, it draws a window onto them from the store the view is
/// scrolled to — the top where the view holds none, or one no longer listed
/// — never running past the last store, with a note at its head and its foot
/// counting the stores out of view on that side. Returns the window drawn, for
/// the frame's pointer record. Display-only — the complete table is a list in
/// the home section.
/// @spec TUI-NAV-019, TUI-NAV-022, TUI-NAV-049, TUI-NAV-140, TUI-RENDER-019,
/// TUI-RENDER-022, TUI-RENDER-027, TUI-THEME-015
pub fn sidebar(f: &mut Frame, area: Rect, state: &GameState, view: &ViewState) -> SidebarRecord {
    let width = area.width as usize;
    let height = area.height as usize;
    let name_col = width.saturating_sub(14).max(1);

    let listed = listed_stores(state);
    let k = listed.len();
    let want = view
        .sidebar_top
        .and_then(|top| listed.iter().position(|r| *r == top))
        .unwrap_or(0);
    let (start, drawn, max_start) = window(k, height, want);
    let full_in = |range: std::ops::Range<usize>| {
        listed[range]
            .iter()
            .filter(|r| wears_full(state, **r))
            .count()
    };

    let mut lines: Vec<Line<'static>> = Vec::with_capacity(height);
    if start > 0 {
        lines.push(sidebar_note('▴', start, full_in(0..start), width));
    }
    for r in &listed[start..start + drawn] {
        lines.extend(store_lines(state, *r, name_col, width));
    }
    let below = k - start - drawn;
    if below > 0 {
        while lines.len() + 1 < height {
            lines.push(Line::from(""));
        }
        lines.push(sidebar_note('▾', below, full_in(start + drawn..k), width));
    }
    f.render_widget(Paragraph::new(lines), area);
    SidebarRecord {
        region: area,
        stores: listed,
        start,
        drawn,
        max_start,
    }
}

// ---------------------------------------------------------------------------
// Hint bar
// ---------------------------------------------------------------------------

/// The focused row, if any row is focused in the current section.
fn focused_row(state: &GameState, view: &ViewState) -> Option<Row> {
    let section = view.section.unwrap_or(Section::Meadow);
    let key = view.focus[section.index()].as_ref()?;
    rows(state, section).into_iter().find(|r| &r.key == key)
}

/// Context keys, most-relevant-first, always ending with the help key. Drops
/// the least relevant first when the width runs out.
///
/// A field (the count prompt, or the palette) holds the keyboard, so it owns
/// this line too — in place of *any* other context's, an armed row's beneath
/// it included, since the field is what the next Enter actually reaches. A
/// full-screen overlay holds it the same way, and the armed keys stop
/// showing the moment the arming itself ends, whether by expiry or by its
/// row leaving every list. @spec TUI-NAV-023, TUI-NAV-024, TUI-NAV-045,
/// TUI-NAV-046, TUI-NAV-066, TUI-NAV-074, TUI-NAV-076, TUI-NAV-077
pub fn hints(f: &mut Frame, area: Rect, state: &GameState, view: &ViewState, stores_hidden: bool) {
    const HELP: &str = "? help";

    // A full-screen overlay (Journal or help) holds the keyboard too, the
    // same as a field does — the armed row's keys beneath it are not what
    // the next Enter reaches while it is open. @spec TUI-NAV-077
    let armed_and_live = view.overlay.is_none()
        && view.armed.as_ref().is_some_and(|a| {
            std::time::Instant::now() < a.expires_at && crate::nav::row_still_listed(state, &a.key)
        });

    let items: Vec<String> = if view.prompt.is_some() {
        strs(&["digits amount", "Enter go", "Esc never mind"])
    } else if matches!(view.overlay, Some(Overlay::Palette { .. })) {
        strs(&["type to find", "Enter jump", "Esc close"])
    } else if armed_and_live {
        strs(&["Enter yes, truly", "Esc not today"])
    } else {
        // x opens the count prompt, and X runs to the maximum affordable,
        // exactly on the rows TUI-NAV-011 and TUI-NAV-012 name: a row tagged
        // count-taking or repeat-N, or a count row — X withheld on a heavy
        // one, since it never commits — and neither key named at all once
        // the core reports the row's item as already the player's.
        // @spec TUI-NAV-023, TUI-NAV-074
        let plain = vec!["↑↓ choose", "Enter do", "Space details", "Tab next"];
        let base: Vec<&str> = match focused_row(state, view) {
            Some(row) => {
                let owned = match &row.key {
                    RowKey::Content(id, _) => {
                        state.lock_reason(*id) == Some(clowder_core::LockReason::AlreadyYours)
                    }
                    _ => false,
                };
                if owned {
                    plain
                } else {
                    match &row.kind {
                        RowKind::Act { semantics, .. } => {
                            use crate::nav::CountSemantics;
                            match semantics {
                                CountSemantics::Single => plain,
                                CountSemantics::RepeatN if row.heavy => {
                                    vec![
                                        "Enter build",
                                        "x amount",
                                        "Space details",
                                        "↑↓ choose",
                                        "Tab next",
                                    ]
                                }
                                CountSemantics::RepeatN => {
                                    vec![
                                        "Enter build",
                                        "x amount",
                                        "X max",
                                        "Space details",
                                        "↑↓ choose",
                                        "Tab next",
                                    ]
                                }
                                CountSemantics::CountTaking if row.heavy => {
                                    vec![
                                        "Enter do",
                                        "x amount",
                                        "Space details",
                                        "↑↓ choose",
                                        "Tab next",
                                    ]
                                }
                                CountSemantics::CountTaking => {
                                    vec![
                                        "Enter do",
                                        "x amount",
                                        "X max",
                                        "Space details",
                                        "↑↓ choose",
                                        "Tab next",
                                    ]
                                }
                            }
                        }
                        RowKind::Count { .. } => {
                            vec![
                                "+ assign",
                                "- unassign",
                                "x amount",
                                "X max",
                                "↑↓ choose",
                                "Space details",
                                "Tab next",
                            ]
                        }
                        RowKind::Display | RowKind::Group => plain,
                    }
                }
            }
            None => plain,
        };

        // Every live mnemonic but the focused row's own — TUI-NAV-082. A
        // focused-item letter is the other act on the item Enter acts on, so
        // it takes the slot right after the key that names Enter; a fixed
        // letter acts on a row the cursor is not on, so the fixed letters are
        // named last, in their rows' list order, just before the help key.
        let mut items: Vec<String> = strs(&base);
        add_mnemonics(&mut items, state, view);
        add_appoint_elder(&mut items, state, view);
        add_done_key(&mut items, state, view);
        // The sidebar's scroll keys come last, so they are the first given up
        // when the width runs short: the sidebar's own notes already say there
        // is more. @spec TUI-NAV-146
        if stores_hidden {
            items.push("{} stores".to_string());
        }
        items
    };

    let width = area.width as usize;
    let mut line = HELP.to_string();
    for take in (0..=items.len()).rev() {
        let mut parts: Vec<&str> = items[..take].iter().map(String::as_str).collect();
        parts.push(HELP);
        let candidate = parts.join(" · ");
        if candidate.chars().count() <= width {
            line = candidate;
            break;
        }
    }
    draw_line(f, area, line);
}

fn strs(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

/// The word a mnemonic's item names it by, matching the design's own hint-bar
/// phrasing for each row it can stand for — a later row's own label
/// (TUI-NAV-089, TUI-NAV-090) where it stands for one.
fn mnemonic_label(key: &RowKey) -> &'static str {
    if let Some(label) = crate::nav::rows::later_row_label(key) {
        return label;
    }
    match key {
        RowKey::Fixed("GatherCatnip") => "gather",
        RowKey::Fixed("Forage") => "forage",
        RowKey::Fixed("Rummage") => "rummage",
        RowKey::Kitten(_, "promote") => "promote",
        _ => "",
    }
}

/// Every live mnemonic on the focused screen but the one whose row is the
/// focused row — Enter already names that act. A focused-item letter is
/// inserted directly after the item naming Enter, among that item's own
/// keys; the fixed letters are appended last, in their rows' list order,
/// after every other context key. @spec TUI-NAV-082, TUI-NAV-090
fn add_mnemonics(items: &mut Vec<String>, state: &GameState, view: &ViewState) {
    // A full-screen overlay suspends every mnemonic the same way it suspends
    // the rest of the global table (`keys::resolve` leaves every letter
    // unbound there), so the bar must not name one as live. @spec TUI-NAV-046
    if view.overlay.is_some() {
        return;
    }
    let Some(section) = view.section else { return };
    let focus = view.focus[section.index()].clone();

    let mut fixed: Vec<String> = Vec::new();
    let mut focused_item: Vec<String> = Vec::new();
    for (c, target, kind) in mnemonics::live(state, section, focus.as_ref()) {
        if Some(&target) == focus.as_ref() {
            continue;
        }
        let entry = format!("{c} {}", mnemonic_label(&target));
        match kind {
            MnemonicKind::Fixed => fixed.push(entry),
            MnemonicKind::FocusedItem => focused_item.push(entry),
        }
    }

    if !focused_item.is_empty() {
        let after_enter = items
            .iter()
            .position(|i| i.split_whitespace().next() == Some("Enter"));
        match after_enter {
            Some(at) => {
                for (offset, entry) in focused_item.into_iter().enumerate() {
                    items.insert(at + 1 + offset, entry);
                }
            }
            None => items.extend(focused_item),
        }
    }
    items.extend(fixed);
}

/// Names the row-bound `n` key directly after the key that names Enter,
/// ranked as a focused-item mnemonic is — so it outlasts every context key
/// after it as the bar sheds width — while the cursor sits on a kitten's row
/// whose AppointElder gate the core reports open. Not itself a mnemonic
/// (TUI-NAV-046), so it is named here rather than by `add_mnemonics`.
/// @spec TUI-NAV-096
fn add_appoint_elder(items: &mut Vec<String>, state: &GameState, view: &ViewState) {
    if view.overlay.is_some() {
        return;
    }
    if view.section != Some(Section::Clowder) {
        return;
    }
    let Some(RowKey::Kitten(ix, _)) = &view.focus[Section::Clowder.index()] else {
        return;
    };
    if state.elder == Some(*ix) || !state.command_gate_open(&Command::AppointElder(*ix)) {
        return;
    }

    let entry = "n appoint Elder".to_string();
    let after_enter = items
        .iter()
        .position(|i| i.split_whitespace().next() == Some("Enter"));
    match after_enter {
        Some(at) => items.insert(at + 1, entry),
        None => items.push(entry),
    }
}

/// Names the Ideas screen's done key last, just before the help key, while it
/// is live: `d show done` while the group is folded, `d fold done` while open.
/// Named last, it is the first key the bar sheds for width.
/// @spec TUI-NAV-122
fn add_done_key(items: &mut Vec<String>, state: &GameState, view: &ViewState) {
    if view.overlay.is_some() || view.section != Some(Section::Ideas) {
        return;
    }
    if crate::nav::rows::done_count(state) == 0 {
        return;
    }
    items.push(
        if view.done_open {
            "d fold done"
        } else {
            "d show done"
        }
        .to_string(),
    );
}
