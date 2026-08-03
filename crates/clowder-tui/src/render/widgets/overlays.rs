//! Transients: the event ticker, the Journal, help, the find palette, and the
//! returning player's report banner.
//!
//! Nothing here blocks. The sim keeps ticking and the chrome keeps rendering
//! behind every one of them — an overlay that stopped the world would be an
//! obligation, and this game does not issue those.
//! @spec TUI-NAV-026, TUI-NAV-027, TUI-NAV-038, TUI-NAV-050, TUI-NAV-055, TUI-NAV-056, TUI-NAV-057

use clowder_core::{
    Availability, ArchId, ContentId, CraftId, DreamLoomId, Event, EventClass, GameState, IdeaId,
    JobId, JournalItem, OfflineReport, RefinementId, ResourceId, RiteId, RuleId, StructureId,
    TraditionId,
};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::{layout::Rect, Frame};

use crate::nav::{Overlay, ViewState};
use crate::render::fmt;
use crate::theme::Role;

/// The mark a line opens with, and the class it belongs to. Prominence is the
/// core's classification (`Event::class`) alone — this only chooses the glyph
/// for a class, never decides which class an event's identity belongs to.
/// @spec TUI-NAV-057
fn tier_mark(class: EventClass) -> &'static str {
    match class {
        EventClass::Flavor => "\u{b7}",   // ·
        EventClass::Mechanical => "\u{25aa}", // ▪
        EventClass::Milestone => "\u{2605}",  // ★
    }
}

/// One occurrence's line: its tier mark and its text. Events carry data, never
/// prose (`clowder_core::Event`) — the words here are the shell's own choice
/// and are not part of the contract.
fn event_line(event: &Event) -> String {
    format!("{} {}", tier_mark(event.class()), event_words(event))
}

/// An occurrence in words. The core hands over typed data precisely so the
/// shell can choose how to say it; showing the type's Debug form puts internal
/// spellings and struct punctuation in front of the player, which is neither
/// the plain truth nor any kind of delight.
fn event_words(event: &Event) -> String {
    use Event as E;
    match event {
        E::KittenArrived { name } => format!("{name} wandered in"),
        E::KittenWanderedOff { name } => format!("{name} is visiting the neighbour's yard"),
        E::KittenReturned { name } => format!("{name} came home"),
        E::JobAssigned { name, .. } => format!("{name} took up new work"),
        E::JobUnassigned { name, .. } => format!("{name} went back to loafing"),
        E::SeasonChanged { season } => format!("{} arrived", format!("{season:?}").to_lowercase()),
        E::NewYear { year } => format!("a new year — {year}"),
        E::StructureBuilt { id, count } => {
            format!("{} raised (×{count})", clowder_core::content::structure_def(*id).name)
        }
        E::IdeaResearched { id } => format!("figured out {}", clowder_core::content::idea_def(*id).name),
        E::ZoomiesStarted { .. } => "the zoomies!".into(),
        E::ZoomiesEnded => "the zoomies pass".into(),
        E::KittenMood { tier } => format!("the clowder is {}", format!("{tier:?}").to_lowercase()),
        E::Revealed { .. } => "something new looks possible".into(),
        E::SkyEventVisible => "something bright crossed the sky".into(),
        E::SpeedChanged { speed } => format!("speed ×{speed}"),
        E::ElderAppointed { name } => format!("{name} is the Elder now"),
        E::KittenPromoted { name, .. } => format!("{name} was promoted"),
        E::ColonyDiscovered { .. } => "a new neighbour found".into(),
        E::MeadowMoved { generation } => format!("the clowder moved on — generation {generation}"),
        E::DistantDawn => "a distant dawn".into(),
        // Everything else says its kind without unpacking its payload: better a
        // plain noun than a struct dump, and better than inventing prose for
        // occurrences whose wording nobody has settled.
        other => {
            let d = format!("{other:?}");
            let head = d.split([' ', '{', '(']).next().unwrap_or("something").to_string();
            spaced_words(&head)
        }
    }
}

/// "PromiseQuieted" -> "promise quieted".
fn spaced_words(camel: &str) -> String {
    let mut out = String::new();
    for (i, c) in camel.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            out.push(' ');
        }
        out.extend(c.to_lowercase());
    }
    out
}

/// A wall-clock span, humanized coarsely enough that two spans differing by
/// weeks never read the same.
fn format_span(elapsed_seconds: u64) -> String {
    let days = elapsed_seconds / 86_400;
    let hours = (elapsed_seconds % 86_400) / 3_600;
    if days > 0 && hours > 0 {
        format!("{days}d {hours}h")
    } else if days > 0 {
        format!("{days}d")
    } else if hours > 0 {
        format!("{hours}h")
    } else {
        format!("{}m", (elapsed_seconds % 3_600) / 60)
    }
}

/// The largest positive resource gains in a report, curated to a handful so a
/// one-line banner has room for them. @spec TUI-NAV-055
fn largest_gains(report: &OfflineReport) -> Vec<(ResourceId, f64)> {
    let mut gains: Vec<(ResourceId, f64)> = ResourceId::ALL
        .into_iter()
        .map(|r| (r, report.resource_deltas[r.ix()]))
        .filter(|(_, delta)| *delta > 0.0)
        .collect();
    gains.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    gains.truncate(3);
    gains
}

/// The curated gains, rendered as text. Never a negative number and never the
/// small change — TUI-NAV-055 says the *largest* gains, so it has to choose.
fn gains_text(report: &OfflineReport) -> String {
    largest_gains(report)
        .into_iter()
        .map(|(r, d)| format!("+{} {}", fmt::compact(d), format!("{r:?}").to_lowercase()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The dim one-line ticker. Flavour is quieter than mechanical; milestones are
/// marked. Prominence comes from the core's classification, never from this
/// module deciding which events matter. @spec TUI-NAV-026, TUI-NAV-057
pub fn ticker(f: &mut Frame, area: Rect, state: &GameState) {
    let entries = state.journal.iter_chronological();
    let text = match entries.last().map(|e| &e.item) {
        Some(JournalItem::Happened(event)) => event_line(event),
        Some(JournalItem::Away(report)) => {
            format!("\u{2605} while you were away ({}): {}", format_span(report.elapsed_seconds), gains_text(report))
        }
        None => String::new(),
    };
    let style = Style::default().fg(Role::Dim.color().expect("Dim carries a colour")).add_modifier(Modifier::DIM);
    f.render_widget(Paragraph::new(Line::styled(text, style)), area);
}

/// Full scrollback over the retained journal, grouped by day.
/// @spec TUI-NAV-026
pub fn journal(f: &mut Frame, area: Rect, state: &GameState, view: &ViewState) {
    let anchor = match &view.overlay {
        Some(Overlay::Journal { anchor }) => *anchor,
        _ => 0,
    };

    let mut lines: Vec<Line<'static>> = Vec::new();
    // Newest first, so scrolling forward from the top reads like the ticker
    // it complements. @spec TUI-NAV-026
    for entry in state.journal.iter_chronological().into_iter().rev() {
        match &entry.item {
            JournalItem::Happened(event) => lines.push(Line::raw(event_line(event))),
            JournalItem::Away(report) => lines.extend(away_report_lines(report)),
        }
    }

    let height = area.height as usize;
    let start = anchor.min(lines.len());
    let end = (start + height).min(lines.len());
    let visible = lines[start..end].to_vec();
    f.render_widget(Paragraph::new(visible), area);
}

/// The pinned entry's full folded detail: the elapsed span and gains a banner
/// would also show, and — unlike the banner — every kitten who wandered,
/// named as a visit that reverses on its own, and every arrival.
/// @spec TUI-NAV-027, TUI-NAV-056
fn away_report_lines(report: &OfflineReport) -> Vec<Line<'static>> {
    let mut lines = vec![Line::raw(format!(
        "\u{2605} away for {}: {}",
        format_span(report.elapsed_seconds),
        gains_text(report)
    ))];
    for (name, _) in &report.wandered {
        // A present absence that reverses on its own — never a loss or a
        // count decrease. @spec TUI-NAV-056
        lines.push(Line::raw(format!(
            "  {name} is off visiting the neighbours' yard \u{2014} home again soon."
        )));
    }
    for (name, _) in &report.arrived {
        lines.push(Line::raw(format!("  {name} arrived!")));
    }
    lines
}

/// The complete key map and glyph legend. There is no manual; this and the
/// hint bar are the manual. @spec TUI-NAV-025
pub fn help(f: &mut Frame, area: Rect) {
    const KEYS: &[(&str, &str)] = &[
        ("1-9, 0", "Jump to that section"),
        ("Tab / BackTab", "Next / previous revealed section"),
        ("Arrows / hjkl", "Move the cursor, cycle sections"),
        ("[ / ]", "Previous / next group header"),
        ("Enter", "Row's primary action; commits an armed row"),
        ("Space / i", "Toggle the inspector"),
        ("x", "Open the count prompt; Enter applies xN"),
        ("X", "Row's action x max affordable"),
        ("+ / -", "Increment / decrement a count row"),
        ("/", "Find-anything palette over revealed content"),
        ("e", "Journal: full scrollback + pinned reports"),
        ("o", "Observe the sky"),
        ("< / >", "Game speed down / up"),
        ("?", "This help overlay"),
        ("q", "Quit (saves first)"),
        ("Esc", "Close the topmost transient"),
        ("Ctrl-C", "Emergency quit (saves first)"),
    ];
    const LEGEND: &[(&str, &str)] = &[
        ("\u{25cf}", "affordable now"),
        ("\u{25d0}", "partially affordable"),
        ("\u{25cb}", "revealed, not yet affordable"),
        ("\u{2713}", "owned / done"),
        ("\u{25b8}", "cursor"),
        ("\u{2605}", "milestone / new content"),
        ("\u{b7}", "flavor event"),
        ("\u{25aa}", "mechanical event"),
        ("\u{25e6}", "idle kitten"),
        ("~", "unread offline report"),
        ("!", "open sky-event window"),
    ];

    let mut lines: Vec<Line<'static>> = Vec::new();
    for (key, action) in KEYS {
        lines.push(Line::raw(format!("{key:<14} {action}")));
    }
    lines.push(Line::raw(String::new()));
    for (glyph, meaning) in LEGEND {
        lines.push(Line::raw(format!("{glyph}  {meaning}")));
    }
    f.render_widget(Paragraph::new(lines), area);
}

/// Every kind of content the palette can find, whether or not it happens to
/// be revealed right now — filtered to revealed content below.
/// @spec TUI-NAV-038
fn all_content_ids() -> Vec<ContentId> {
    let mut ids = Vec::new();
    ids.extend(StructureId::ALL.into_iter().map(ContentId::Structure));
    ids.extend(IdeaId::ALL.into_iter().map(ContentId::Idea));
    ids.extend(CraftId::ALL.into_iter().map(ContentId::Craft));
    ids.extend(JobId::ALL.into_iter().map(ContentId::Job));
    ids.extend(RiteId::ALL.into_iter().map(ContentId::Rite));
    ids.extend(ArchId::ALL.into_iter().map(ContentId::Arch));
    ids.extend(RefinementId::ALL.into_iter().map(ContentId::Refinement));
    ids.extend(RuleId::ALL.into_iter().map(ContentId::Rule));
    ids.extend(TraditionId::ALL.into_iter().map(ContentId::Tradition));
    ids.extend(DreamLoomId::ALL.into_iter().map(ContentId::DreamLoom));
    ids
}

/// Find-anything over revealed content, narrowing by substring as characters
/// arrive. @spec TUI-NAV-038
pub fn palette(f: &mut Frame, area: Rect, state: &GameState, view: &ViewState) {
    let query = match &view.overlay {
        Some(Overlay::Palette { query, .. }) => query.clone(),
        _ => String::new(),
    };
    let needle = query.to_lowercase();

    let mut names: Vec<String> = all_content_ids()
        .into_iter()
        .filter(|id| state.availability(*id) != Availability::Hidden)
        .map(|id| crate::render::content_name(id))
        .filter(|name| needle.is_empty() || name.to_lowercase().contains(&needle))
        .collect();
    names.sort();
    names.dedup();

    let mut lines = vec![Line::raw(format!("Find: {query}"))];
    lines.extend(names.into_iter().map(Line::raw));
    let height = area.height as usize;
    lines.truncate(height);
    f.render_widget(Paragraph::new(lines), area);
}

/// The unread report the banner is currently showing: the first entry in
/// `view.reports` that has not been dismissed, so a queue of several drains
/// oldest-visible-first while newer ones keep their place ahead of it.
/// @spec TUI-NAV-050
fn active_report<'a>(state: &'a GameState, view: &ViewState) -> Option<&'a OfflineReport> {
    let pending = view.reports.iter().find(|r| !r.dismissed)?;
    let entries = state.journal.iter_chronological();
    match &entries.get(pending.entry)?.item {
        JournalItem::Away(report) => Some(report),
        JournalItem::Happened(_) => None,
    }
}

/// The returning player's one-line banner: elapsed span, the largest gains, and
/// arrivals. Departures belong to the pinned entry — the first thing seen on
/// return leads with what grew.
/// @spec TUI-NAV-027, TUI-NAV-050, TUI-NAV-055, TUI-NAV-056
pub fn banner(f: &mut Frame, area: Rect, state: &GameState, view: &ViewState) {
    let Some(report) = active_report(state, view) else {
        return;
    };

    let mut parts = vec![gains_text(report)];
    if !report.arrived.is_empty() {
        let n = report.arrived.len();
        parts.push(format!("{n} kitten{} arrived", if n == 1 { "" } else { "s" }));
    }

    // Nothing here reads `report.wandered` or `report.returned` — departures
    // and absences are the pinned entry's to tell. @spec TUI-NAV-055, TUI-NAV-056
    let line = format!(
        "~ while you were away ({}): {} \u{b7} e for the tale \u{b7} Esc dismiss",
        format_span(report.elapsed_seconds),
        parts.join(", "),
    );
    f.render_widget(Paragraph::new(Line::styled(line, Style::default().add_modifier(Modifier::REVERSED))), area);
}
