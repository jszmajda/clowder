//! The persistent furniture: header, tab bar, resource sidebar, hint bar.
//!
//! This is what the two-second glance contract is made of. Its whole job is to
//! be quiet: an empty attention cluster means nothing needs you, and that only
//! works if the eye has learned that something appearing there is information.
//! @spec TUI-NAV-019..025, TUI-NAV-034, TUI-NAV-044, TUI-NAV-045, TUI-NAV-054

use clowder_core::{GameState, Location, MoodTier, ResourceId};
use ratatui::{layout::Rect, text::Line, widgets::Paragraph, Frame};

use crate::nav::rows::{rows, RowKind};
use crate::nav::{Overlay, Section, ViewState};
use crate::render::fmt;

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

/// Left-flush text on one side and right-flush text on the other, within a
/// character budget. When there is not room for both, they simply run
/// together — the terminal-facing Paragraph clips whatever does not fit
/// rather than this function inventing its own truncation.
fn split_line(left: &str, right: &str, width: usize) -> String {
    if right.is_empty() {
        return left.to_string();
    }
    let left_w = left.chars().count();
    let right_w = right.chars().count();
    if left_w + 1 + right_w >= width {
        return format!("{left} {right}");
    }
    let pad = width - left_w - right_w;
    format!("{left}{}{right}", " ".repeat(pad))
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

/// How many rows have appeared, across every section, since the player last
/// looked — what `★N new` counts. @spec TUI-NAV-020
fn new_content_count(state: &GameState, view: &ViewState) -> usize {
    Section::ALL
        .iter()
        .map(|s| {
            let total = rows(state, *s).len();
            total.saturating_sub(view.seen[s.index()].revealed_at_last_visit)
        })
        .sum()
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

/// The right-aligned attention cluster: glyph + count + word, each shown only
/// while its opportunity is live, empty otherwise. @spec TUI-NAV-020,
/// TUI-NAV-022, TUI-NAV-044, TUI-NAV-054
fn chips(state: &GameState, view: &ViewState) -> String {
    let mut out: Vec<String> = Vec::new();

    let idle = idle_here_count(state);
    if idle > 0 {
        out.push(format!("◦{idle} idle"));
    }
    if sky_live(state) {
        out.push("!sky".to_string());
    }
    let new = new_content_count(state, view);
    if new > 0 {
        out.push(format!("★{new} new"));
    }
    if view.reports.iter().any(|r| !r.dismissed) {
        out.push("~away".to_string());
    }

    out.join("  ")
}

/// Clowder generation, date, kitten count, mood as a word, speed, and the
/// right-aligned attention chips. @spec TUI-NAV-019, TUI-NAV-020, TUI-NAV-021,
/// TUI-NAV-022, TUI-NAV-044, TUI-NAV-054
pub fn header(f: &mut Frame, area: Rect, state: &GameState, view: &ViewState) {
    let left = format!(
        "Gen {} · {:?} Day {} Year {} · {} kittens · {} · ×{}",
        state.generation,
        state.season(),
        state.day(),
        state.year(),
        state.kittens.len(),
        mood_word(state.mood_tier()),
        fmt::compact(state.speed),
    );
    let right = chips(state, view);
    draw_line(f, area, split_line(&left, &right, area.width as usize));
}

// ---------------------------------------------------------------------------
// Tab bar
// ---------------------------------------------------------------------------

/// Revealed sections with their digits; a badge on one not yet visited.
/// Width-responsive: full labels where they fit, else a compact digit strip —
/// never a name truncated mid-word, never a tab scrolled out of reach.
/// @spec TUI-NAV-006, TUI-NAV-034
pub fn tabs(f: &mut Frame, area: Rect, state: &GameState, view: &ViewState) {
    let revealed: Vec<Section> = Section::ALL.into_iter().filter(|s| !rows(state, *s).is_empty()).collect();
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

/// A curated glance: revealed resources with stock, a fill bar, and a signed
/// rate. Display-only — the complete table is a list in the home section.
/// @spec TUI-NAV-019, TUI-NAV-022, TUI-NAV-049
pub fn sidebar(f: &mut Frame, area: Rect, state: &GameState) {
    let mut lines: Vec<Line<'static>> = Vec::new();
    for r in ResourceId::ALL {
        let stock = state.stock(r);
        if stock <= 0.0 {
            continue;
        }
        let cap = state.cap(r);
        let name = humanize(&format!("{r:?}"));
        let bar = fill_bar(stock, cap);
        let rate = fmt::rate(state.net_rate(r));
        let full_tag = if cap.is_finite() && cap > 0.0 && stock >= cap { " full" } else { "" };
        // Two lines per resource — a name and cap are wordy together, and the
        // sidebar's fixed width has no room to spare for both on one line
        // without risking the `full` tag (TUI-NAV-022's non-colour channel)
        // being the part that gets clipped.
        lines.push(Line::from(name));
        let numbers = format!("{} {bar} {rate}{full_tag}", fmt::compact(stock));
        lines.push(Line::from(numbers));
    }
    f.render_widget(Paragraph::new(lines), area);
}

// ---------------------------------------------------------------------------
// Hint bar
// ---------------------------------------------------------------------------

/// The focused row's kind, if any row is focused in the current section.
fn focused_kind(state: &GameState, view: &ViewState) -> Option<RowKind> {
    let section = view.section.unwrap_or(Section::Meadow);
    let key = view.focus[section.index()].as_ref()?;
    rows(state, section).into_iter().find(|r| &r.key == key).map(|r| r.kind)
}

/// Context keys, most-relevant-first, always ending with the help key. Drops
/// the least relevant first when the width runs out.
///
/// A field (the count prompt, or the palette) holds the keyboard, so it owns
/// this line too — in place of *any* other context's, an armed row's beneath
/// it included, since the field is what the next Enter actually reaches.
/// @spec TUI-NAV-023, TUI-NAV-024, TUI-NAV-045, TUI-NAV-046, TUI-NAV-066
pub fn hints(f: &mut Frame, area: Rect, state: &GameState, view: &ViewState) {
    const HELP: &str = "? help";

    let items: Vec<&str> = if view.prompt.is_some() {
        vec!["digits amount", "Enter go", "Esc never mind"]
    } else if matches!(view.overlay, Some(Overlay::Palette { .. })) {
        vec!["type to find", "Enter jump", "Esc close"]
    } else if view.armed.is_some() {
        vec!["Enter yes, truly", "Esc not today"]
    } else {
        match focused_kind(state, view) {
            Some(RowKind::Act { semantics, .. }) => {
                use crate::nav::CountSemantics;
                match semantics {
                    CountSemantics::Single => {
                        vec!["↑↓ choose", "Enter do", "Space details", "Tab next"]
                    }
                    CountSemantics::RepeatN => {
                        vec!["Enter build", "x amount", "X max", "Space details", "↑↓ choose", "Tab next"]
                    }
                    CountSemantics::CountTaking => {
                        vec!["Enter do", "x amount", "X max", "Space details", "↑↓ choose", "Tab next"]
                    }
                }
            }
            Some(RowKind::Count { .. }) => {
                vec!["+ assign", "- unassign", "↑↓ choose", "Space details", "Tab next"]
            }
            _ => vec!["↑↓ choose", "Enter do", "Space details", "Tab next"],
        }
    };

    let width = area.width as usize;
    let mut line = HELP.to_string();
    for take in (0..=items.len()).rev() {
        let mut parts: Vec<&str> = items[..take].to_vec();
        parts.push(HELP);
        let candidate = parts.join(" · ");
        if candidate.chars().count() <= width {
            line = candidate;
            break;
        }
    }
    draw_line(f, area, line);
}
