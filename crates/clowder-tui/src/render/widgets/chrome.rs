//! The persistent furniture: header, tab bar, resource sidebar, hint bar.
//!
//! This is what the two-second glance contract is made of. Its whole job is to
//! be quiet: an empty attention cluster means nothing needs you, and that only
//! works if the eye has learned that something appearing there is information.
//! @spec TUI-NAV-019..025, TUI-NAV-034, TUI-NAV-044, TUI-NAV-045, TUI-NAV-054

use clowder_core::{GameState, Location, MoodTier, ResourceId};
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::nav::mnemonics::{self, MnemonicKind};
use crate::nav::rows::{rows, tab_sections, Row, RowKey, RowKind};
use crate::nav::{Overlay, Section, ViewState};
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
    tab_sections(state, view.section).into_iter().filter(|s| !view.seen[s.index()].visited).collect()
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
    state.kittens.iter().filter(|k| k.job.is_none() && k.location == Location::Home).count()
}

/// Whether the sky-observation window is open right now — the core's own
/// notion of "live", not merely "was set": a faded window is `Locked`.
/// @spec TUI-NAV-020
fn sky_live(state: &GameState) -> bool {
    state.pending_sky_event.is_some_and(|expiry| state.ticks < expiry)
}

/// The four attention opportunities, in the fixed order idle, sky, new, away,
/// each live or not. Shared by the counted and uncounted chip clusters so the
/// two never drift on which chips are live.
fn live_opportunities(state: &GameState, view: &ViewState) -> [bool; 4] {
    [
        idle_here_count(state) > 0,
        sky_live(state),
        new_content_count(state, view) > 0,
        view.reports.iter().any(|r| !r.dismissed),
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
        [format!("◦{idle} idle"), "!sky".to_string(), format!("★{new} new"), "~away".to_string()]
    } else {
        ["◦idle".to_string(), "!sky".to_string(), "★new".to_string(), "~away".to_string()]
    };
    labels.into_iter().zip(live).filter(|(_, on)| *on).map(|(label, _)| label).collect()
}

/// The five glance facts in full wording, joined `·`. @spec TUI-NAV-086
fn full_facts(state: &GameState) -> Vec<String> {
    vec![
        format!("Gen {}", state.generation),
        format!("{:?} Day {} Year {}", state.season(), state.day(), state.year()),
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
        (fit(&full_facts(state).join(" · "), &with_counts, width), &with_labels),
        (fit(&compact.join(" "), &with_counts, width), &with_labels),
        (fit(&compact.join(" "), &without_counts, width), &without_labels),
        (fit(&compact[1..].join(" "), &without_counts, width), &without_labels),
        (fit(&compact[2..].join(" "), &without_counts, width), &without_labels),
        (fit(&compact[3..].join(" "), &without_counts, width), &without_labels),
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

/// Sections that list a selectable row, or hold focus, with their digits; a
/// badge on one not yet visited since it last appeared. Width-responsive:
/// full labels where they fit, else a compact digit strip — never a name
/// truncated mid-word, never a tab scrolled out of reach.
/// @spec TUI-NAV-006, TUI-NAV-034, TUI-NAV-085
pub fn tabs(f: &mut Frame, area: Rect, state: &GameState, view: &ViewState) {
    let revealed: Vec<Section> = tab_sections(state, view.section);
    let focused = view.section;

    let mark = |s: Section| if !view.seen[s.index()].visited { "★" } else { "" };

    // Full form: every revealed tab named beside its digit.
    let full: Vec<String> =
        revealed.iter().map(|s| format!("{}{} {}", mark(*s), s.digit(), s.title())).collect();
    let full_line = full.join("  ");

    if full_line.chars().count() <= area.width as usize {
        draw_line(f, area, full_line);
        return;
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
    draw_line(f, area, compact.join(" "));
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
    format!("{}{}", FILL_FULL.to_string().repeat(filled), FILL_EMPTY.to_string().repeat(FILL_CELLS - filled))
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

/// As many of `listed`'s stores as `capacity` two-line slots hold, chosen
/// first from those wearing `full` and then from the rest, each in the
/// stores group's order — then handed back in that same order, so a store
/// chosen for room is still drawn where the group places it.
/// @spec TUI-RENDER-022
fn choose_stores(state: &GameState, listed: &[ResourceId], capacity: usize) -> Vec<ResourceId> {
    let mut chosen: Vec<ResourceId> =
        listed.iter().copied().filter(|r| wears_full(state, *r)).take(capacity).collect();
    let room = capacity - chosen.len();
    chosen.extend(listed.iter().copied().filter(|r| !wears_full(state, *r)).take(room));
    listed.iter().copied().filter(|r| chosen.contains(r)).collect()
}

/// A store's two lines: its name, stock, `/`, and cap at the right edge; then
/// its fill bar, its rate, and — while capped — the `full` tag. Only the
/// `full` tag wears `attention` (TUI-THEME-015); everything else, including
/// the fill bar, stays in the inherited foreground so colour never carries a
/// meaning the glyphs do not. @spec TUI-RENDER-019, TUI-NAV-022
fn store_lines(state: &GameState, r: ResourceId, name_col: usize, width: usize) -> [Line<'static>; 2] {
    let stock = state.stock(r);
    let cap = state.cap(r);
    let name = fmt::fit(&humanize(&format!("{r:?}")), name_col);
    let cap_part = if cap.is_finite() { format!("/{:<6}", fmt::compact(cap)) } else { " ".repeat(7) };
    let first = format!("{name:<name_col$} {:>6}{cap_part}", fmt::compact(stock));

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
    [Line::from(first), Line::from(spans)]
}

/// A curated glance: the stores the stores group lists, each drawn whole
/// (TUI-RENDER-019) or not at all, standing in the stores group's order. Where
/// the sidebar's lines cannot hold every listed store, the last line is a
/// `dim` note naming how many are left out, and the stores drawn are chosen
/// first from those wearing `full`. Display-only — the complete table is a
/// list in the home section.
/// @spec TUI-NAV-019, TUI-NAV-022, TUI-NAV-049, TUI-RENDER-019, TUI-RENDER-022,
/// TUI-THEME-015
pub fn sidebar(f: &mut Frame, area: Rect, state: &GameState) {
    let width = area.width as usize;
    let height = area.height as usize;
    let name_col = width.saturating_sub(14).max(1);

    let listed = listed_stores(state);
    let k = listed.len();
    let capacity_rows = 2 * k;
    let (drawn_n, note) = if capacity_rows <= height {
        (k, false)
    } else {
        (height.saturating_sub(1) / 2, true)
    };
    let chosen = choose_stores(state, &listed, drawn_n);

    let mut lines: Vec<Line<'static>> = Vec::with_capacity(height);
    for r in &chosen {
        lines.extend(store_lines(state, *r, name_col, width));
    }
    if note {
        while lines.len() + 1 < height {
            lines.push(Line::from(""));
        }
        let left = k - chosen.len();
        lines.push(Line::styled(format!("+{left} more"), style_for(Role::Dim)));
    }
    f.render_widget(Paragraph::new(lines), area);
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
pub fn hints(f: &mut Frame, area: Rect, state: &GameState, view: &ViewState) {
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
                    RowKey::Content(id, _) => state.lock_reason(*id) == Some(clowder_core::LockReason::AlreadyYours),
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
                                    vec!["Enter build", "x amount", "Space details", "↑↓ choose", "Tab next"]
                                }
                                CountSemantics::RepeatN => {
                                    vec!["Enter build", "x amount", "X max", "Space details", "↑↓ choose", "Tab next"]
                                }
                                CountSemantics::CountTaking if row.heavy => {
                                    vec!["Enter do", "x amount", "Space details", "↑↓ choose", "Tab next"]
                                }
                                CountSemantics::CountTaking => {
                                    vec!["Enter do", "x amount", "X max", "Space details", "↑↓ choose", "Tab next"]
                                }
                            }
                        }
                        RowKind::Count { .. } => {
                            vec!["+ assign", "- unassign", "x amount", "X max", "↑↓ choose", "Space details", "Tab next"]
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
/// phrasing for each row it can stand for.
fn mnemonic_label(key: &RowKey) -> &'static str {
    match key {
        RowKey::Fixed("GatherCatnip") => "gather",
        RowKey::Fixed("Forage") => "forage",
        RowKey::Fixed("Rummage") => "rummage",
        RowKey::Kitten(_, "promote") => "promote",
        RowKey::Kitten(_, "elder") => "Elder",
        RowKey::Colony(_, "penpal") => "pen-pal",
        _ => "",
    }
}

/// Every live mnemonic on the focused screen but the one whose row is the
/// focused row — Enter already names that act. A focused-item letter is
/// inserted directly after the item naming Enter, among that item's own
/// keys; the fixed letters are appended last, in their rows' list order,
/// after every other context key. @spec TUI-NAV-082
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
        let after_enter = items.iter().position(|i| i.split_whitespace().next() == Some("Enter"));
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
