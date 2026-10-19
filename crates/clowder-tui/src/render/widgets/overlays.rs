//! Transients: the event ticker, the Journal, help, the find palette, and the
//! returning player's report banner.
//!
//! Nothing here blocks. The sim keeps ticking and the chrome keeps rendering
//! behind every one of them — an overlay that stopped the world would be an
//! obligation, and this game does not issue those.
//! @spec TUI-NAV-026, TUI-NAV-027, TUI-NAV-038, TUI-NAV-050, TUI-NAV-055, TUI-NAV-056, TUI-NAV-057

use clowder_core::{Event, EventClass, GameState, JournalItem, OfflineReport, ResourceId, Timer};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::{layout::Rect, Frame};

use crate::nav::rows::RowKey;
use crate::nav::{Overlay, ViewState};
use crate::render::{fmt, line_from_pieces, Piece};
use crate::theme::Role;

/// The mark a line opens with, and the class it belongs to. Prominence is the
/// core's classification (`Event::class`) alone — this only chooses the glyph
/// for a class, never decides which class an event's identity belongs to.
/// @spec TUI-NAV-057
fn tier_mark(class: EventClass) -> &'static str {
    match class {
        EventClass::Flavor => "\u{b7}",       // ·
        EventClass::Mechanical => "\u{25aa}", // ▪
        EventClass::Milestone => "\u{2605}",  // ★
    }
}

/// One occurrence's line: its tier mark and its text. Events carry data, never
/// prose (`clowder_core::Event`) — the words here are the shell's own choice
/// and are not part of the contract. Where the event carries what an act
/// brought home, the line names each store, in its coat, with its amount.
/// @spec TUI-NAV-147, TUI-THEME-024
pub(crate) fn event_line(event: &Event) -> Vec<Piece> {
    let mut out = vec![Piece::plain(format!("{} ", tier_mark(event.class())))];
    match event {
        Event::PounceReturned {
            feathers,
            bells,
            shinies,
            unicorns,
            ..
        } => {
            let haul = pounce_haul(*feathers, *bells, *shinies, *unicorns);
            if haul.is_empty() {
                out.push(Piece::plain("the pounce came home empty-pawed"));
            } else {
                out.push(Piece::plain("the pounce came home with "));
                out.extend(goods_pieces(&haul));
            }
        }
        Event::TradeReturned {
            colony,
            napped,
            goods,
            ..
        } => {
            let from = clowder_core::content::colony_def(*colony).name;
            if goods.is_empty() {
                out.push(Piece::plain(format!("caravans came home from the {from}")));
            } else {
                out.push(Piece::plain(format!(
                    "caravans came home from the {from} with "
                )));
                out.extend(goods_pieces(goods));
            }
            if *napped > 0 {
                out.push(Piece::plain(format!("; {napped} napped on the way")));
            }
        }
        other => out.push(Piece::plain(event_words(other))),
    }
    out
}

/// A pounce's haul as stores and amounts, in the core's resource order, each
/// store the haul raised and none it left at nothing. @spec TUI-NAV-147
pub(crate) fn pounce_haul(
    feathers: f64,
    bells: f64,
    shinies: f64,
    unicorns: u32,
) -> Vec<(ResourceId, f64)> {
    let mut haul: Vec<(ResourceId, f64)> = [
        (ResourceId::Feathers, feathers),
        (ResourceId::Bells, bells),
        (ResourceId::Shinies, shinies),
        (ResourceId::Unicorns, f64::from(unicorns)),
    ]
    .into_iter()
    .filter(|(_, a)| *a > 0.0)
    .collect();
    haul.sort_by_key(|(r, _)| r.ix());
    haul
}

/// Stores and amounts as a sentence names them: `+12 Twigs, +3 Catnip`, each
/// name in its coat, in the order given.
fn goods_pieces(goods: &[(ResourceId, f64)]) -> Vec<Piece> {
    let mut out = Vec::new();
    for (i, (r, amount)) in goods.iter().enumerate() {
        if i > 0 {
            out.push(Piece::plain(", "));
        }
        out.push(Piece::plain(format!("+{} ", fmt::compact(*amount))));
        out.push(resource_piece(*r));
    }
    out
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
            format!(
                "{} raised (×{count})",
                clowder_core::content::structure_def(*id).name
            )
        }
        E::IdeaResearched { id } => {
            format!("figured out {}", clowder_core::content::idea_def(*id).name)
        }
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
            let head = d
                .split([' ', '{', '('])
                .next()
                .unwrap_or("something")
                .to_string();
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

/// A resource named the way its stores row names it — the same humanized
/// spelling every other surface uses, never the bare identifier's own case.
fn resource_label(r: ResourceId) -> String {
    let tag = format!("{r:?}");
    let mut out = String::with_capacity(tag.len() + 4);
    for (i, c) in tag.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

/// A resource named in a sentence: its stores-row name, in its coat.
fn resource_piece(r: ResourceId) -> Piece {
    let name = resource_label(r);
    let chars = name.chars().count();
    Piece::name(r, name, chars)
}

/// Pieces drawn as one line in `base`, each resource's name in its coat and
/// at full strength (TUI-THEME-023), never fitted: the region clips it.
fn line_of(pieces: &[Piece], base: Style) -> Line<'static> {
    line_of_width(pieces, base, usize::MAX)
}

/// As `line_of`, fitted to `width` with an ellipsis (TUI-RENDER-014).
fn line_of_width(pieces: &[Piece], base: Style, width: usize) -> Line<'static> {
    line_from_pieces(pieces, base, true, width)
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

/// How much time passed, humanized — stated as *more than* that span, with a
/// gentle note that the meadow stopped counting there, where the core's own
/// replay hit its cap (`truncated`). A floor is not the absence, so the
/// wording says so rather than reading as an exact figure it is not.
/// @spec TUI-NAV-027, TUI-NAV-055
fn span_clause(report: &OfflineReport) -> String {
    let span = format_span(report.elapsed_seconds);
    if report.truncated {
        format!("more than {span}")
    } else {
        span
    }
}

/// The gentle note appended where the core stopped counting — never on its
/// own line, since it belongs to the span it qualifies. @spec TUI-NAV-027
fn truncated_note(report: &OfflineReport) -> Option<&'static str> {
    report
        .truncated
        .then_some(" \u{2014} the meadow stopped counting there")
}

/// The largest positive resource gains in a report, curated to a handful so a
/// one-line banner has room for them. A gain below the display floor reads as
/// `0` (TUI-RENDER-017), which is nothing gained, so it is left off rather
/// than drawn as zero. @spec TUI-NAV-055, TUI-RENDER-017
fn largest_gains(report: &OfflineReport) -> Vec<(ResourceId, f64)> {
    let mut gains: Vec<(ResourceId, f64)> = ResourceId::ALL
        .into_iter()
        .map(|r| (r, report.resource_deltas[r.ix()]))
        .filter(|(_, delta)| *delta >= fmt::DISPLAY_FLOOR)
        .collect();
    gains.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    gains.truncate(3);
    gains
}

/// The curated gains, each resource in its coat. Never a negative number and
/// never the small change — TUI-NAV-055 says the *largest* gains, so it has
/// to choose. @spec TUI-THEME-024
fn gains_pieces(report: &OfflineReport) -> Vec<Piece> {
    goods_pieces(&largest_gains(report))
}

/// Every resource whose stock changed at all during the report's span,
/// signed — gains before decreases, and each group largest amount first (a
/// decrease's size is how far the stock fell). The pinned entry tells the
/// whole report, unlike the banner's curated three. @spec TUI-NAV-027
fn changed_resources(report: &OfflineReport) -> Vec<(ResourceId, f64)> {
    let mut changed: Vec<(ResourceId, f64)> = ResourceId::ALL
        .into_iter()
        .map(|r| (r, report.resource_deltas[r.ix()]))
        .filter(|(_, delta)| *delta != 0.0)
        .collect();
    changed.sort_by(|a, b| {
        let group = |d: f64| if d > 0.0 { 0 } else { 1 };
        group(a.1).cmp(&group(b.1)).then(
            b.1.abs()
                .partial_cmp(&a.1.abs())
                .unwrap_or(std::cmp::Ordering::Equal),
        )
    });
    changed
}

/// The too-small card's one line: calm, says nothing is wrong, and names the
/// size the game needs. @spec TUI-NAV-087
const TOO_SMALL_LINE: &str = "this pane is too cozy \u{2014} 80\u{d7}24 needed";

/// The too-small card: below the floor, this line drawn whole in place of the
/// whole frame — nothing else — wherever the pane is at least as wide as the
/// line and at least one row tall. Too narrow for the line whole, or of no
/// height at all, draws nothing rather than a clipped or wrapped fragment.
/// @spec TUI-NAV-087
pub fn too_small(f: &mut Frame, area: Rect) {
    let line_w = TOO_SMALL_LINE.chars().count() as u16;
    if area.width < line_w || area.height < 1 {
        return;
    }
    let y = area.y + (area.height - 1) / 2;
    let row = Rect::new(area.x, y, area.width, 1);
    f.render_widget(Paragraph::new(Line::raw(TOO_SMALL_LINE)), row);
}

/// The ticker's line: the newest entry on the left, drawn plainly — flavour
/// in `dim`, everything else in the inherited foreground, no faint attribute —
/// and the lights right-aligned on the right, each a timed state the core
/// lists with its clock. The entry takes what the lights leave, less two
/// blank columns, shortened with an ellipsis where it is longer; no light is
/// ever cut. Prominence comes from the core's classification, and what runs
/// on a clock from the core's timers, never from this module.
/// @spec TUI-NAV-026, TUI-NAV-057, TUI-NAV-150, TUI-THEME-024, TUI-THEME-025
pub fn ticker(f: &mut Frame, area: Rect, state: &GameState) {
    let entries = state.journal.iter_chronological();
    let (pieces, base): (Vec<Piece>, Style) = match entries.last().map(|e| &e.item) {
        Some(JournalItem::Happened(event)) => {
            let base = if event.class() == EventClass::Flavor {
                crate::render::style_for(Role::Dim)
            } else {
                Style::default()
            };
            (event_line(event), base)
        }
        Some(JournalItem::Away(report)) => {
            let mut out = vec![Piece::plain(format!(
                "\u{2605} while you were away ({}): ",
                format_span(report.elapsed_seconds)
            ))];
            out.extend(gains_pieces(report));
            (out, Style::default())
        }
        None => (Vec::new(), Style::default()),
    };

    const GAP: usize = 2;
    let lights = lights(state);
    let width = area.width as usize;
    let cluster: usize =
        lights.iter().map(Line::width).sum::<usize>() + GAP * lights.len().saturating_sub(1);
    let room = if lights.is_empty() {
        width
    } else {
        width.saturating_sub(cluster + GAP)
    };

    let left = Rect::new(area.x, area.y, room as u16, area.height.min(1));
    f.render_widget(Paragraph::new(line_of_width(&pieces, base, room)), left);

    let mut spans: Vec<Span<'static>> = Vec::new();
    for (i, light) in lights.into_iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(" ".repeat(GAP)));
        }
        spans.extend(light.spans);
    }
    let drawn = cluster.min(width) as u16;
    let right = Rect::new(area.right() - drawn, area.y, drawn, area.height.min(1));
    f.render_widget(Paragraph::new(Line::from(spans)), right);
}

/// One light for each timed state the core lists that a glance can use, in
/// the core's order: the sky's as its chip's glyph and word, its clock, and
/// its key, all in `attention`; the others as `◇`, their word, and their
/// clock, the clock in `dim`. A déjà-vu day lasts too short a time to read,
/// so it has none.
/// @spec TUI-NAV-151, TUI-NAV-152, TUI-THEME-015, TUI-THEME-026
fn lights(state: &GameState) -> Vec<Line<'static>> {
    let running = |word: &str, clock: String| {
        Line::from(vec![
            Span::raw(format!("\u{25c7} {word} ")),
            Span::styled(clock, crate::render::style_for(Role::Dim)),
        ])
    };
    state
        .timers()
        .into_iter()
        .filter_map(|(timer, left)| {
            let clock = clock(left, state.speed);
            match timer {
                Timer::SkyEvent => Some(Line::from(Span::styled(
                    format!("!sky {clock} (o)"),
                    crate::render::style_for(Role::Attention),
                ))),
                Timer::WhaleVisit => Some(running("whale", clock)),
                Timer::Jamboree => Some(running("jamboree", clock)),
                Timer::Zoomies => Some(running("zoomies", clock)),
                Timer::DejaVuDay => None,
            }
        })
        .collect()
}

/// How long `ticks` last in real time at `speed`, rounded down to a coarse
/// step — `<10s`, tens of seconds, whole minutes, whole hours — so a clock
/// never promises more time than there is and stays still between glances.
/// @spec TUI-NAV-153
fn clock(ticks: u64, speed: f64) -> String {
    let seconds = ticks as f64 / (clowder_core::content::TICKS_PER_SECOND as f64 * speed);
    if seconds < 10.0 {
        "<10s".to_string()
    } else if seconds < 60.0 {
        format!("{}s", (seconds / 10.0).floor() as u64 * 10)
    } else if seconds < 3600.0 {
        format!("{}m", (seconds / 60.0).floor() as u64)
    } else {
        format!("{}h", (seconds / 3600.0).floor() as u64)
    }
}

/// Full scrollback over the retained journal, grouped by day — with every
/// unread report pinned above it, newest first, so the Journal opens onto the
/// tale rather than making the player scroll to find it.
/// @spec TUI-NAV-026, TUI-NAV-027, TUI-NAV-050
pub fn journal(f: &mut Frame, area: Rect, state: &GameState, view: &ViewState) {
    let anchor = match &view.overlay {
        Some(Overlay::Journal { anchor }) => *anchor,
        _ => 0,
    };

    let lines = journal_lines(state, view);
    let height = area.height as usize;
    let start = anchor.min(lines.len());
    let end = (start + height).min(lines.len());
    let visible = lines[start..end].to_vec();
    f.render_widget(Paragraph::new(visible), area);
}

/// The scroll at which the Journal's last line stands at the foot of a region
/// `height` lines tall, or none where every line fits. What the wheel may
/// scroll it to, recorded with the frame. @spec TUI-RENDER-025
pub fn journal_max_scroll(state: &GameState, view: &ViewState, height: u16) -> Option<usize> {
    let total = journal_lines(state, view).len();
    (total > height as usize).then(|| total - height as usize)
}

/// Every line of the Journal, top to bottom, before any scroll.
fn journal_lines(state: &GameState, view: &ViewState) -> Vec<Line<'static>> {
    let entries = state.journal.iter_chronological();

    // While this Journal is open, what is pinned is frozen to what was
    // unread the moment it opened (`ViewState::journal_pinned`) — reading it
    // must not walk an entry back out of the place the player opened onto.
    // With no Journal open at all, pinning simply reads what is unread now.
    // Indices are positions in `state.journal.kept` — append-only, so they
    // stay put however many ordinary occurrences land in `recent` afterward
    // — never a position in the two merged and re-sorted, which an event
    // stamped earlier than a pinned report would shift out from under it.
    // @spec TUI-NAV-027, TUI-NAV-050
    let pinned: Vec<usize> = if matches!(view.overlay, Some(Overlay::Journal { .. })) {
        view.journal_pinned.clone()
    } else {
        view.reports
            .iter()
            .filter(|r| !r.dismissed)
            .map(|r| r.entry)
            .collect()
    };
    // Identified by their place in memory, since that is what a `kept` index
    // and a reference out of the merged, re-sorted list can be compared by in
    // common. @spec TUI-NAV-027, TUI-NAV-050
    let pinned_ptrs: Vec<*const clowder_core::JournalEntry> = pinned
        .iter()
        .filter_map(|&idx| state.journal.kept.get(idx).map(|e| e as *const _))
        .collect();

    let mut lines: Vec<Line<'static>> = Vec::new();
    for &idx in &pinned {
        if let Some(JournalItem::Away(report)) = state.journal.kept.get(idx).map(|e| &e.item) {
            lines.extend(away_report_lines(report));
        }
    }

    // Newest first, so scrolling forward from the top reads like the ticker
    // it complements — every pinned entry left out, since it is already
    // told above. @spec TUI-NAV-026
    for entry in entries.into_iter().rev() {
        if pinned_ptrs.contains(&(entry as *const _)) {
            continue;
        }
        match &entry.item {
            JournalItem::Happened(event) => {
                lines.push(line_of(&event_line(event), Style::default()))
            }
            JournalItem::Away(report) => lines.extend(away_report_lines(report)),
        }
    }
    lines
}

/// The pinned entry's full folded detail: the span (stated as more than
/// itself, with a gentle note, where the core stopped counting); every
/// resource whose stock changed, signed, gains before decreases, each group
/// largest first; each kitten who arrived, wandered, or came home, by name;
/// and the seasons passed. Unlike the banner, this tells everything the
/// report holds — dismissing the banner must lose nothing. Each changed
/// store is named in its coat.
/// @spec TUI-NAV-027, TUI-NAV-056, TUI-THEME-024
fn away_report_lines(report: &OfflineReport) -> Vec<Line<'static>> {
    let mut header = format!("\u{2605} away for {}", span_clause(report));
    if let Some(note) = truncated_note(report) {
        header.push_str(note);
    }
    header.push('.');
    let mut lines = vec![Line::raw(header)];

    // Every changed stock, one to a line — never joined onto the header's
    // own line, which a report touching every resource would run well past
    // any pane's width; the Journal scrolls, so a line each costs nothing.
    // @spec TUI-NAV-027
    for (r, d) in changed_resources(report) {
        let figure = Piece::plain(format!(
            "  {}{} ",
            if d > 0.0 { "+" } else { "-" },
            fmt::compact(d.abs())
        ));
        lines.push(line_of(&[figure, resource_piece(r)], Style::default()));
    }

    // Occurrences, never whereabouts (TUI-NAV-056): each list counts what
    // happened during the span, by name, with its count where the report
    // counts more than one — never where anyone ended up.
    for (name, count) in &report.arrived {
        lines.push(Line::raw(if *count > 1 {
            format!("  {name} arrived {count} times!")
        } else {
            format!("  {name} arrived!")
        }));
    }
    for (name, count) in &report.wandered {
        // A visit to the neighbour's yard that ends back home — never a loss
        // or a departure. @spec TUI-NAV-056
        lines.push(Line::raw(if *count > 1 {
            format!("  {name} visited the neighbours' yard {count} times and came back home.")
        } else {
            format!("  {name} visited the neighbours' yard and came back home.")
        }));
    }
    for (name, count) in &report.returned {
        lines.push(Line::raw(if *count > 1 {
            format!("  {name} came home {count} times.")
        } else {
            format!("  {name} came home.")
        }));
    }
    if report.seasons_passed > 0 {
        lines.push(Line::raw(format!(
            "  {} seasons passed.",
            report.seasons_passed
        )));
    }
    lines
}

/// The complete key map and glyph legend. There is no manual; this and the
/// hint bar are the manual. @spec TUI-NAV-025, TUI-NAV-090, TUI-NAV-146
pub fn help(f: &mut Frame, area: Rect) {
    const KEYS: &[(&str, &str)] = &[
        ("1-9, 0", "Jump to that section"),
        ("Tab / BackTab", "Next / previous revealed section"),
        ("Arrows / hjkl", "Move the cursor, cycle sections"),
        ("[ / ]", "Previous / next group header"),
        ("{ / }", "Scroll the stores sidebar up / down"),
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
    // The design's per-screen mnemonic table (TUI-NAV-046), listed here in
    // full: a mnemonic is live only on its own screen, so it never earns a
    // permanent line in the hint bar, and the help overlay is the only place
    // every one of them is named at once, whatever screen the player is on.
    // @spec TUI-NAV-025, TUI-NAV-046, TUI-NAV-109
    // A focused-item mnemonic's act is worded by its row's label (TUI-NAV-089)
    // followed by the item it acts on, in parentheses (TUI-NAV-090) — the same
    // wording the hint bar gives it.
    const MNEMONICS: &[(&str, &str)] = &[
        ("Meadow", "g gather / f forage / r rummage"),
        ("Clowder", "p promote (kitten) / n appoint Elder (kitten)"),
        ("Friends", "p send a pen-pal (colony)"),
        // The Ideas screen's done key is no mnemonic (TUI-NAV-046) but is
        // listed beside them as a per-screen key. @spec TUI-NAV-122
        ("Ideas", "d show or fold done"),
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
        ("\u{25c6}", "the Elder"),
        ("~", "unread offline report"),
        ("!", "open sky-event window"),
        ("\u{25c7}", "running on a clock, with its time left"),
    ];

    let width = area.width as usize;
    let keys_whole: Vec<String> = KEYS.iter().map(|(k, a)| format!("{k:<14} {a}")).collect();
    let keys_packed = pack(
        &KEYS
            .iter()
            .map(|(k, a)| format!("{k} {a}"))
            .collect::<Vec<_>>(),
        width,
    );
    let screens: Vec<String> = MNEMONICS
        .iter()
        .map(|(s, k)| format!("{s:<10} {k}"))
        .collect();
    let legend_whole: Vec<String> = LEGEND.iter().map(|(g, m)| format!("{g}  {m}")).collect();
    let legend_packed = pack(
        &LEGEND
            .iter()
            .map(|(g, m)| format!("{g} {m}"))
            .collect::<Vec<_>>(),
        width,
    );

    // Pack as little as fits, so nothing is cut at any supported size: the
    // legend before the key map, and at each step the blank lines between the
    // lists kept while the rows still hold them. The screens' keys are never
    // packed. @spec TUI-NAV-124
    let height = area.height as usize;
    let steps = [
        (&keys_whole, &legend_whole),
        (&keys_whole, &legend_packed),
        (&keys_packed, &legend_packed),
    ];
    let (keys, legend, gaps) = steps
        .iter()
        .flat_map(|&(keys, legend)| [(keys, legend, true), (keys, legend, false)])
        .find(|&(keys, legend, gaps)| {
            keys.len() + screens.len() + legend.len() + if gaps { 2 } else { 0 } <= height
        })
        .unwrap_or((&keys_packed, &legend_packed, false));

    let mut lines: Vec<Line<'static>> = Vec::new();
    for (i, list) in [keys, &screens, legend].into_iter().enumerate() {
        if i > 0 && gaps {
            lines.push(Line::raw(String::new()));
        }
        lines.extend(list.iter().cloned().map(Line::raw));
    }
    f.render_widget(Paragraph::new(lines), area);
}

/// Entries set side by side, three spaces apart so each entry's own inner
/// space never reads as a boundary, filling each line of `width` columns
/// before starting the next; an entry is never split.
fn pack(entries: &[String], width: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for e in entries {
        match out.last_mut() {
            Some(line) if line.chars().count() + 3 + e.chars().count() <= width => {
                line.push_str("   ");
                line.push_str(e);
            }
            _ => out.push(e.clone()),
        }
    }
    out
}

/// Find-anything over the sections' own lists, narrowing by substring as
/// characters arrive, drawn from the top so the result Enter takes is always
/// the first line and always on screen. A store's result names it in its
/// coat. @spec TUI-NAV-038, TUI-NAV-062, TUI-NAV-063, TUI-THEME-019
pub fn palette(f: &mut Frame, area: Rect, state: &GameState, view: &ViewState) {
    let query = match &view.overlay {
        Some(Overlay::Palette { query, .. }) => query.as_str(),
        _ => "",
    };

    let mut lines = vec![Line::raw(format!("Find: {query}"))];
    lines.extend(
        crate::nav::palette::results(state, query)
            .into_iter()
            .map(|r| {
                let digit = Piece::plain(format!("[{}] ", r.section.digit()));
                let name = match r.key {
                    RowKey::Resource(res) => {
                        let chars = r.name.chars().count();
                        Piece::name(res, r.name, chars)
                    }
                    _ => Piece::plain(r.name),
                };
                line_of(&[digit, name], Style::default())
            }),
    );
    let height = area.height as usize;
    lines.truncate(height);
    f.render_widget(Paragraph::new(lines), area);
}

/// The unread report the banner is currently showing — the first entry in
/// `view.reports` that has not been dismissed, so a queue of several drains
/// oldest-visible-first while newer ones keep their place ahead of it — and
/// the journal-entry index it sits at, which is the one thing the nap route
/// hangs its own length in years off (`ViewState::nap_years`), since the
/// core's retained report does not say whether a nap or an absence folded it.
/// @spec TUI-NAV-050
fn active_report<'a>(state: &'a GameState, view: &ViewState) -> Option<(usize, &'a OfflineReport)> {
    let pending = view.reports.iter().find(|r| !r.dismissed)?;
    match &state.journal.kept.get(pending.entry)?.item {
        JournalItem::Away(report) => Some((pending.entry, report)),
        JournalItem::Happened(_) => None,
    }
}

/// How much time passed, as the banner's opening clause: the core's own span
/// for an absence, or — for a nap, which the core folds through the very same
/// report — the nap's length in years, the one fact the retained report
/// itself cannot carry (Decisions & Alternatives: how much time a nap's
/// banner says passed). @spec TUI-NAV-055
fn banner_time_clause(report: &OfflineReport, nap_years: Option<u32>) -> String {
    match nap_years {
        Some(years) => format!(
            "~ after a nap of {years} year{}",
            if years == 1 { "" } else { "s" }
        ),
        None => format!("~ while you were away ({})", span_clause(report)),
    }
}

/// The returning player's one-line banner, assembled clause by clause so a
/// pane too narrow for all of it drops whole clauses — never cuts one — in
/// the fixed order TUI-NAV-055 gives, and never drops the span.
/// @spec TUI-NAV-027, TUI-NAV-050, TUI-NAV-055, TUI-NAV-056
pub fn banner(f: &mut Frame, area: Rect, state: &GameState, view: &ViewState) {
    let Some((entry, report)) = active_report(state, view) else {
        return;
    };
    let nap_years = view
        .nap_years
        .iter()
        .find(|(e, _)| *e == entry)
        .map(|(_, y)| *y);
    let line = banner_line(report, nap_years, area.width as usize);
    f.render_widget(
        Paragraph::new(Line::styled(
            line,
            Style::default().add_modifier(Modifier::REVERSED),
        )),
        area,
    );
}

/// Assemble the banner's line for a pane `width` columns wide. Nothing here
/// reads `report.wandered` or `report.returned` — departures and absences are
/// the pinned entry's to tell. @spec TUI-NAV-055, TUI-NAV-056
fn banner_line(report: &OfflineReport, nap_years: Option<u32>, width: usize) -> String {
    let time = banner_time_clause(report, nap_years);
    let gains: Vec<String> = largest_gains(report)
        .into_iter()
        .map(|(r, d)| format!("+{} {}", fmt::compact(d), resource_label(r)))
        .collect();
    let arrivals_sum: u32 = report.arrived.iter().map(|(_, c)| *c).sum();
    let arrivals = (arrivals_sum > 0).then(|| {
        format!(
            "{arrivals_sum} kitten{} arrived",
            if arrivals_sum == 1 { "" } else { "s" }
        )
    });
    let quiet_note =
        (gains.is_empty() && arrivals.is_none()).then(|| "the meadow kept quiet".to_string());

    // Priority order — lowest dropped first — exactly TUI-NAV-055's list:
    // the dismiss hint, the third gain, the second gain, the tale hint, the
    // arrivals, then the largest gain (or the quiet note standing in for
    // it). Absent clauses are simply never candidates to drop, and the
    // numbering below is `assemble_banner_line`'s own vocabulary of which
    // clause each priority names.
    let mut droppable: Vec<u8> = vec![0]; // the dismiss hint is always a candidate
    if gains.len() > 2 {
        droppable.push(1); // the third gain
    }
    if gains.len() > 1 {
        droppable.push(2); // the second gain
    }
    droppable.push(3); // the tale hint
    if arrivals.is_some() {
        droppable.push(4);
    }
    if quiet_note.is_some() || !gains.is_empty() {
        droppable.push(5); // the largest gain, or the quiet note in its place
    }

    for dropped in 0..=droppable.len() {
        let kept: std::collections::BTreeSet<u8> = droppable[dropped..].iter().copied().collect();
        let line = assemble_banner_line(
            &time,
            &gains,
            arrivals.as_deref(),
            quiet_note.as_deref(),
            &kept,
        );
        if line.chars().count() <= width || dropped == droppable.len() {
            return line;
        }
    }
    time
}

/// Reconstruct the line in the banner's own display order — span, then the
/// gains (or the quiet note in their place), the arrivals, and the two hints
/// — including a clause only where its priority is still in `kept`.
fn assemble_banner_line(
    time: &str,
    gains: &[String],
    arrivals: Option<&str>,
    quiet_note: Option<&str>,
    kept: &std::collections::BTreeSet<u8>,
) -> String {
    let mut middle: Vec<&str> = Vec::new();
    if kept.contains(&5) {
        if let Some(note) = quiet_note {
            middle.push(note);
        } else if let Some(t) = gains.first() {
            middle.push(t);
        }
    }
    if kept.contains(&2) {
        if let Some(t) = gains.get(1) {
            middle.push(t);
        }
    }
    if kept.contains(&1) {
        if let Some(t) = gains.get(2) {
            middle.push(t);
        }
    }
    if kept.contains(&4) {
        if let Some(t) = arrivals {
            middle.push(t);
        }
    }

    let mut line = time.to_string();
    if !middle.is_empty() {
        line.push_str(": ");
        line.push_str(&middle.join(", "));
    }
    if kept.contains(&3) {
        line.push_str(" \u{b7} e for the tale");
    }
    if kept.contains(&0) {
        line.push_str(" \u{b7} Esc dismiss");
    }
    line
}
