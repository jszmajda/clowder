//! Transients — the ticker, the Journal, the find palette, and the returning
//! player's banner — as `render::widgets::overlays` draws them.
//!
//! @spec TUI-NAV-026, TUI-NAV-027, TUI-NAV-038, TUI-NAV-050, TUI-NAV-055,
//! TUI-NAV-056, TUI-NAV-057
//!
//! **What this file is about.** Four of the seven specs here are cozy-tenet
//! requirements rather than mechanism, and the difference between honouring
//! them and subtly breaking them is invisible to a test that only asks whether
//! something was drawn. So the tests are written against the *reasons* the
//! design gives:
//!
//! * the ticker is a one-line surface and the Journal is the lossless one, so
//!   the pair is tested together: an occurrence that has scrolled off the
//!   ticker must still be reachable in the Journal. That is the whole of "no
//!   event shall flash, block, or expire unseen while it is retained";
//! * prominence is the *core's* answer, so the sharpest test uses one event
//!   identity the core classes two ways — the first copy of a structure is a
//!   milestone and every later copy is ordinary — and demands the shell draw
//!   them at different tiers. Nothing about the event's identity changed;
//! * the banner is the first thing a returning player sees, so it must be
//!   *unmoved* by who wandered off. That is stated here as cell-for-cell
//!   equality between a report with wanderers and the same report without,
//!   which no partial-credit rendering can pass;
//! * and because dismissing the banner must lose nothing, the same pair must
//!   make the Journal differ — the detail the banner declined is pinned.
//!
//! **How a rendering is identified without guessing prose.** Events carry data
//! and never prose (`clowder_core::Event`), so the words the shell chooses are
//! its own business and are not asserted. Where a test needs to know that
//! *something specific* was rendered it uses a differential: two states that
//! differ in exactly one entry must not draw the same overlay, and two that
//! differ in nothing the surface is allowed to show must draw the same one.
//! Lexical assertions are made in exactly three places, each one a thing a
//! spec names outright: the resource behind a *gain* (TUI-NAV-055 says the
//! banner carries the largest resource gains), the destination and the return
//! of a wandering (TUI-NAV-056 says the entry names where they went and that
//! they come back), and the vocabulary of loss (TUI-NAV-056 forbids it). A
//! content name is matched by a uniqueness-checked prefix, as the sibling
//! frame tests do, so an ellipsis cannot turn a hit into a miss.
//!
//! **Geometry comes from the layout, not from this file.** The ticker rect is
//! the row `render::layout` gives the ticker and the banner rect is the row it
//! gives the banner, both at the 80×24 floor — which is what makes "one line"
//! a real constraint the curation has to fit inside rather than a number
//! chosen here.
//!
//! **Not tested here.** The help overlay is TUI-NAV-025, which belongs to the
//! discoverability chunk; `overlays::help` is deliberately left alone. The
//! palette's Enter (move to the hit's section and focus it) and the banner's
//! dismissal on Esc or first navigation are input, not drawing: they live in
//! `nav::update` and are that chunk's to test.

use clowder_tui::nav::{Overlay, PendingReport, ViewState};
use clowder_tui::render::layout::{compute, Body, Layout};
use clowder_tui::render::widgets::overlays;
use clowder_tui::theme::Role;

use std::collections::BTreeSet;

use clowder_core::content::structure_def;
use clowder_core::{
    load_string, save_string, Availability, ContentId, Event, EventClass, GameState, IdeaId,
    OfflineReport, ResourceId, Season, StructureId,
};
use ratatui::backend::TestBackend;
use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::{Position, Rect};
use ratatui::style::Modifier;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::{Frame, Terminal};

// ---------------------------------------------------------------------------
// Geometry
// ---------------------------------------------------------------------------

const FLOOR_W: u16 = 80;
const FLOOR_H: u16 = 24;

fn whole(w: u16, h: u16) -> Rect {
    Rect::new(0, 0, w, h)
}

/// The chrome's own geometry at the floor. Every rect below comes from here so
/// that a test about curation is a test against the space the frame really
/// leaves, not against a width invented in a test file.
fn chrome(banner: bool) -> Layout {
    compute(whole(FLOOR_W, FLOOR_H), None, banner)
}

fn ticker_rect() -> Rect {
    chrome(false).ticker
}

fn banner_rect() -> Rect {
    chrome(true).banner.expect("the layout gives a banner row while a report is unread")
}

/// The body: what a non-blocking overlay is drawn over, leaving the header,
/// the tab bar, the ticker and the hint bar to the chrome behind it.
fn overlay_rect() -> Rect {
    let layout = chrome(false);
    let (y, height) = match layout.body {
        Body::Narrow { sidebar, .. } => (sidebar.y, sidebar.height),
        Body::Wide { sidebar, .. } => (sidebar.y, sidebar.height),
        Body::TooSmall(rect) => panic!("the floor is a supported size, not {rect:?}"),
    };
    Rect::new(0, y, FLOOR_W, height)
}

// ---------------------------------------------------------------------------
// Drawing
// ---------------------------------------------------------------------------

fn drawn(render: impl FnOnce(&mut Frame)) -> Buffer {
    let mut term =
        Terminal::new(TestBackend::new(FLOOR_W, FLOOR_H)).expect("TestBackend is infallible");
    term.draw(|frame| render(frame)).expect("TestBackend is infallible");
    term.backend().buffer().clone()
}

fn ticker(state: &GameState) -> Buffer {
    drawn(|f| overlays::ticker(f, ticker_rect(), state))
}

fn journal_view(anchor: usize) -> ViewState {
    ViewState { overlay: Some(Overlay::Journal { anchor }), ..ViewState::default() }
}

fn journal(state: &GameState, anchor: usize) -> Buffer {
    let view = journal_view(anchor);
    drawn(|f| overlays::journal(f, overlay_rect(), state, &view))
}

fn journal_text(state: &GameState, anchor: usize) -> String {
    text_of(&journal(state, anchor), overlay_rect())
}

fn palette(state: &GameState, query: &str) -> Buffer {
    let view = ViewState {
        overlay: Some(Overlay::Palette { query: query.to_string(), selected: 0 }),
        ..ViewState::default()
    };
    drawn(|f| overlays::palette(f, overlay_rect(), state, &view))
}

fn palette_text(state: &GameState, query: &str) -> String {
    text_of(&palette(state, query), overlay_rect())
}

fn banner(state: &GameState, view: &ViewState) -> Buffer {
    drawn(|f| overlays::banner(f, banner_rect(), state, view))
}

fn banner_text(state: &GameState, view: &ViewState) -> String {
    text_of(&banner(state, view), banner_rect())
}

// ---------------------------------------------------------------------------
// Reading a buffer
// ---------------------------------------------------------------------------

fn region_lines(buf: &Buffer, rect: Rect) -> Vec<String> {
    (rect.y..rect.bottom())
        .map(|y| {
            (rect.x..rect.right())
                .map(|x| buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "))
                .collect::<String>()
        })
        .collect()
}

fn text_of(buf: &Buffer, rect: Rect) -> String {
    region_lines(buf, rect).join("\n")
}

fn mentions(text: &str, needle: &str) -> bool {
    occurrences(text, needle) > 0
}

fn occurrences(text: &str, needle: &str) -> usize {
    let (haystack, needle) = (text.to_lowercase(), needle.to_lowercase());
    if needle.is_empty() {
        return 0;
    }
    haystack.matches(&needle).count()
}

/// The words on a surface, lowercased and stripped of punctuation, so a
/// vocabulary check cannot be fooled by "leftovers" containing "left".
fn words(text: &str) -> BTreeSet<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| w.to_lowercase())
        .collect()
}

/// Words that make a departure a bereavement. Nothing in clowder is ever lost
/// (HLD tenet 1), and TUI-NAV-056 says a wandering is never rendered as a
/// loss — so these may not appear anywhere near a report.
const LOSS_WORDS: [&str; 12] = [
    "lost",
    "lose",
    "loses",
    "losing",
    "died",
    "dies",
    "dead",
    "death",
    "missing",
    "vanished",
    "starved",
    "sacrificed",
];

fn loss_vocabulary(text: &str) -> Vec<String> {
    let present = words(text);
    LOSS_WORDS.iter().filter(|w| present.contains(**w)).map(|w| w.to_string()).collect()
}

/// Whether the surface shows a number going down: a minus sign immediately
/// followed by a digit.
fn shows_a_decrease(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    chars.windows(2).any(|pair| pair[0] == '-' && pair[1].is_ascii_digit())
}

fn cells_of(buf: &Buffer, rect: Rect) -> Vec<&Cell> {
    let mut out = Vec::new();
    for y in rect.y..rect.bottom() {
        for x in rect.x..rect.right() {
            out.push(buf.cell((x, y)).expect("every cell of a drawn rect exists"));
        }
    }
    out
}

fn is_blank(cell: &Cell) -> bool {
    cell.symbol().trim().is_empty()
}

/// Quiet, in either of the two channels a dim thing may use: the theme's dim
/// role, or the terminal's own dim attribute. @spec TUI-NAV-026
fn is_quiet(cell: &Cell) -> bool {
    Role::Dim.color() == Some(cell.fg) || cell.modifier.contains(Modifier::DIM)
}

/// The mark a line opens with — the run of symbols before its first letter or
/// digit, and the styles those cells carry.
///
/// This is where an event's *tier* lives: the design opens a flavour line with
/// `·`, a mechanical one with `▪`, and a milestone with `★`. Reading the tier
/// off the line's opening rather than off its whole text is what keeps this
/// independent of whatever words the shell chooses for the event itself.
#[derive(Debug, PartialEq, Eq)]
struct TierMark {
    symbols: String,
    /// The styles those cells carry, as text — neither `Color` nor `Modifier`
    /// is ordered, and only their distinctness matters here.
    styles: BTreeSet<String>,
}

fn tier_mark(buf: &Buffer, rect: Rect) -> TierMark {
    let mut symbols = String::new();
    let mut styles = BTreeSet::new();
    let mut started = false;
    for cell in cells_of(buf, rect) {
        let symbol = cell.symbol();
        if !started && is_blank(cell) {
            continue;
        }
        if symbol.chars().any(|c| c.is_alphanumeric()) {
            break;
        }
        started = true;
        symbols.push_str(symbol);
        if !is_blank(cell) {
            styles.insert(format!("{:?} {:?}", cell.fg, cell.modifier));
        }
    }
    TierMark { symbols: symbols.trim().to_string(), styles }
}

/// The tier the ticker draws for one occurrence, on a line of its own.
fn tier_of(event: Event) -> TierMark {
    let state = journaled(&[(100, event)]);
    let buffer = ticker(&state);
    let mark = tier_mark(&buffer, ticker_rect());
    assert!(
        !mark.symbols.is_empty(),
        "the ticker opened a line with no mark at all, so nothing on it says which tier the \
         occurrence belongs to; the design opens a flavour line with `·`, a mechanical one with \
         `▪`, and a milestone with `★`. The line read as {:?}",
        text_of(&buffer, ticker_rect()).trim_end()
    );
    mark
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// A meadow whose journal holds exactly these occurrences. The core decides
/// which of them roll and which are kept forever; this hands them over and
/// lets it.
fn journaled(entries: &[(u64, Event)]) -> GameState {
    let mut state = GameState::new(11);
    for (tick, event) in entries {
        state.journal.record(*tick, event);
    }
    state
}

/// A run of ordinary occurrences, each distinct from the others, long enough
/// that its oldest entry cannot possibly still be on a one-line ticker.
fn filler(n: usize) -> Vec<(u64, Event)> {
    (0..n)
        .map(|i| {
            (
                100 + i as u64,
                // Not the first of its kind, so the core classes it ordinary.
                Event::StructureBuilt { id: StructureId::ALL[i % StructureId::ALL.len()], count: 2 },
            )
        })
        .collect()
}

/// A meadow whose journal holds these away-reports as pinned entries, oldest
/// first — the one path both an absence and a nap take (SAVE-015).
fn away(reports: &[(u64, OfflineReport)]) -> GameState {
    let mut state = GameState::new(11);
    for (tick, report) in reports {
        state.journal.record_away(*tick, report);
    }
    assert_eq!(
        state.journal.kept.len(),
        reports.len(),
        "fixture: every report must be long enough an absence for the core to pin it"
    );
    state
}

/// One pending report per pinned entry, newest first — the order `ViewState`
/// documents and TUI-NAV-050 requires. Every away entry lives in `kept` and
/// nothing else is in this journal, so an entry's index is the same whichever
/// retained list it is counted against.
fn unread(state: &GameState) -> ViewState {
    let entries = state.journal.kept.len();
    ViewState {
        reports: (0..entries).rev().map(|entry| PendingReport { entry, dismissed: false }).collect(),
        ..ViewState::default()
    }
}

/// The dominant gain in a graded report, and the one nothing could call large.
const DOMINANT: ResourceId = ResourceId::Catnip;
const TINIEST: ResourceId = ResourceId::Hushfelt;

/// An away-report over many resources: one gain that dwarfs everything, a
/// scatter of middling ones, and one so small it has no business on a curated
/// line. Forty resources cannot fit in eighty columns, so the banner has to
/// choose — and TUI-NAV-055 says which way.
fn graded_report(elapsed_seconds: u64) -> OfflineReport {
    let mut report = OfflineReport { elapsed_seconds, ..OfflineReport::default() };
    report.resource_deltas[DOMINANT.ix()] = 1_240_000.0;
    report.resource_deltas[TINIEST.ix()] = 1.0;
    for r in ResourceId::ALL.into_iter().filter(|r| *r != DOMINANT && *r != TINIEST).take(12) {
        report.resource_deltas[r.ix()] = 20.0;
    }
    report
}

/// An away-report that moved exactly one resource, so what the banner names
/// says unambiguously which report it is showing.
fn report_gaining(elapsed_seconds: u64, gain: ResourceId) -> OfflineReport {
    let mut report = OfflineReport { elapsed_seconds, ..OfflineReport::default() };
    report.resource_deltas[gain.ix()] = 4_800.0;
    report
}

fn named(names: &[&str]) -> Vec<(String, u32)> {
    names.iter().map(|n| ((*n).to_string(), 1)).collect()
}

/// How far a scroll anchor has to be swept before "the Journal never shows
/// this" is a fair conclusion: past the end of the entries in either
/// direction, whichever way the overlay orders them.
fn sweep(state: &GameState) -> usize {
    state.journal.iter_chronological().len() * 2 + FLOOR_H as usize
}

/// Whether any scroll position at all tells the two journals apart.
fn journal_differs_somewhere(a: &GameState, b: &GameState) -> bool {
    let far = sweep(a).max(sweep(b));
    (0..=far).any(|anchor| journal(a, anchor) != journal(b, anchor))
}

/// A prefix of a content name, long enough to be unique and short enough to
/// survive TUI-RENDER-014's ellipsis if a column ever gets tight.
fn probe(b: StructureId) -> String {
    let name: String = structure_def(b).name.chars().take(10).collect();
    let clashes = StructureId::ALL
        .into_iter()
        .filter(|o| *o != b && structure_def(*o).name.contains(&name))
        .count();
    assert_eq!(clashes, 0, "fixture: {name:?} does not identify one structure");
    name
}

/// A meadow that has revealed exactly these structures and nothing else.
fn revealing(structures: &[StructureId]) -> GameState {
    let mut state = GameState::new(11);
    state.revealed = structures.iter().map(|s| ContentId::Structure(*s)).collect();
    state
}

// ---------------------------------------------------------------------------
// TUI-NAV-026 — a dim ticker, and a Journal that loses nothing
// ---------------------------------------------------------------------------

/// The ticker is the *latest* events: change the newest thing that happened
/// and the line changes.
/// @spec TUI-NAV-026
#[test]
fn the_ticker_shows_what_just_happened() {
    let mut season = filler(12);
    season.push((900, Event::SeasonChanged { season: Season::Winter }));
    let mut zoomies = filler(12);
    zoomies.push((900, Event::ZoomiesEnded));

    let a = journaled(&season);
    let b = journaled(&zoomies);

    assert_ne!(
        ticker(&a),
        ticker(&b),
        "two meadows whose most recent occurrence differs drew the same ticker; the ticker \
         renders the events the core reported, not a fixed line"
    );
}

/// The ticker is one line and the Journal is the lossless one. An occurrence
/// thirty events back cannot still be on the ticker — and it must still be
/// somewhere in the Journal, because nothing that is retained may expire
/// unseen.
/// @spec TUI-NAV-026
#[test]
fn what_has_scrolled_off_the_ticker_is_still_in_the_journal() {
    let mut earlier = vec![(1, Event::SeasonChanged { season: Season::Winter })];
    earlier.extend(filler(30));
    let mut other = vec![(1, Event::ZoomiesEnded)];
    other.extend(filler(30));

    let a = journaled(&earlier);
    let b = journaled(&other);
    assert_eq!(a.journal.iter_chronological().len(), 31, "fixture: one differing oldest entry");

    assert_eq!(
        ticker(&a),
        ticker(&b),
        "thirty occurrences later, the oldest one was still on a one-line ticker; the ticker \
         carries the latest events and the Journal carries the rest"
    );
    assert!(
        journal_differs_somewhere(&a, &b),
        "an occurrence the core is still retaining could not be found at any scroll position in \
         the Journal — it has scrolled off the ticker and out of the game, which is exactly what \
         'no event expires unseen' forbids"
    );
}

/// The Journal is over the *retained journal*: the rolling window plus every
/// milestone the core kept permanently. A Journal that showed only the recent
/// window would drop the moments worth remembering — the ones it exists for.
/// @spec TUI-NAV-026
#[test]
fn the_journal_reaches_the_milestones_the_core_kept() {
    let researched = Event::IdeaResearched { id: IdeaId::ALL[0] };
    let summoned = Event::WhaleSummoned;
    assert_eq!(researched.class(), EventClass::Milestone, "fixture: the core keeps this forever");
    assert_eq!(summoned.class(), EventClass::Milestone, "fixture: and this one");

    let mut with_idea = vec![(1, researched)];
    with_idea.extend(filler(40));
    let mut with_whale = vec![(1, summoned)];
    with_whale.extend(filler(40));

    let a = journaled(&with_idea);
    let b = journaled(&with_whale);
    assert_eq!(a.journal.kept.len(), 1, "fixture: the milestone is in the kept list");
    assert_eq!(a.journal.recent.len(), 40, "fixture: the rest are in the rolling window");

    assert!(
        journal_differs_somewhere(&a, &b),
        "two journals differing only in the milestone the core kept forever drew the same \
         Journal at every scroll position; the overlay is reading the rolling window alone"
    );

    // ...and it really scrolls: forty-one entries do not fit twenty rows.
    let far = sweep(&a);
    assert_ne!(
        journal(&a, 0),
        journal(&a, far),
        "a journal of {} entries in a {}-row overlay drew the same thing at both ends of its \
         scroll; the anchor must move the window",
        a.journal.iter_chronological().len(),
        overlay_rect().height
    );
}

/// The Journal survives closing and reopening the game, because the journal is
/// saved state and the overlay renders it rather than a buffer of this
/// session's events. That is what makes a report the player never got to read
/// safe.
/// @spec TUI-NAV-026
#[test]
fn the_journal_a_returning_player_opens_is_the_one_the_save_kept() {
    let mut entries = vec![
        (1, Event::IdeaResearched { id: IdeaId::ALL[0] }),
        (2, Event::ElderAppointed { name: "Quillow".to_string() }),
    ];
    entries.extend(filler(20));
    let mut before = journaled(&entries);
    before.journal.record_away(500, &graded_report(2 * 86_400 + 4 * 3_600));

    let saved_at = 1_700_000_000;
    let loaded = load_string(&save_string(&before, saved_at), saved_at)
        .expect("a save just written loads")
        .state;
    assert_eq!(
        loaded.journal, before.journal,
        "fixture: closing and reopening with no time passed keeps the journal verbatim"
    );

    for anchor in 0..=sweep(&before) {
        assert_eq!(
            journal(&before, anchor),
            journal(&loaded, anchor),
            "at scroll position {anchor} the reopened game drew a different Journal from the one \
             it was closed with; the overlay must render the retained journal the save carries"
        );
    }
}

/// The ticker is dim. It is chrome the eye passes over, not something asking
/// to be read — which is only true if it is drawn that way.
/// @spec TUI-NAV-026
#[test]
fn the_ticker_is_drawn_quietly() {
    let state = journaled(&[(100, Event::SeasonChanged { season: Season::Winter })]);
    let buffer = ticker(&state);
    let rect = ticker_rect();
    let line = text_of(&buffer, rect);
    assert!(!line.trim().is_empty(), "fixture: the ticker drew something for an occurrence");

    // The line's opening mark may carry its tier's own colour; everything
    // after it is the event's text, and the text is the part that must recede.
    let cells = cells_of(&buffer, rect);
    let after_mark = cells
        .iter()
        .skip_while(|c| is_blank(c) || !c.symbol().chars().any(|ch| ch.is_alphanumeric()))
        .filter(|c| !is_blank(c));
    for cell in after_mark {
        assert!(
            is_quiet(cell),
            "the ticker drew {:?} at full strength; a dim ticker uses the theme's dim role or the \
             terminal's dim attribute, so that an event never competes with the thing the player \
             is doing. The line read as {:?}",
            cell.symbol(),
            line.trim_end()
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-057 — prominence is the core's answer, not this module's
// ---------------------------------------------------------------------------

/// One event identity, classed two ways by the core: the first copy of a
/// structure is a milestone and every later copy is ordinary. Nothing about
/// the event's identity differs between the two, so a shell that ranked
/// prominence from identity would draw them at the same tier — and a shell
/// that reads the core's classification cannot.
/// @spec TUI-NAV-057
#[test]
fn one_identity_classed_two_ways_is_drawn_at_two_tiers() {
    let id = StructureId::ALL[0];
    let first = Event::StructureBuilt { id, count: 1 };
    let later = Event::StructureBuilt { id, count: 2 };
    assert_eq!(first.class(), EventClass::Milestone, "fixture: the core marks the first copy");
    assert_eq!(later.class(), EventClass::Mechanical, "fixture: and not the ones after it");

    assert_ne!(
        tier_of(first),
        tier_of(later),
        "the same occurrence, classed a milestone by the core and then ordinary, opened its line \
         with the same mark; prominence must be ranked from the classification the core supplies"
    );
}

/// Two occurrences the core classes alike are drawn alike, however different
/// they are as events. Prominence is a property of the tier, and the shell may
/// not decide from an event's identity which tier it belongs to.
/// @spec TUI-NAV-057
#[test]
fn occurrences_of_one_class_are_drawn_at_one_tier() {
    let ordinary = [
        Event::KittenArrived { name: "Zibbly".to_string() },
        Event::SeasonChanged { season: Season::Winter },
    ];
    let remembered =
        [Event::IdeaResearched { id: IdeaId::ALL[0] }, Event::ElderAppointed { name: "Mopsy".to_string() }];

    for pair in [ordinary, remembered] {
        let [a, b] = pair;
        assert_eq!(a.class(), b.class(), "fixture: the core classes these two alike");
        let class = a.class();
        assert_eq!(
            tier_of(a),
            tier_of(b),
            "two occurrences the core both class {class:?} opened their lines with different \
             marks; the mark ranks the tier, and the tier is the core's answer — not this \
             event's identity"
        );
    }
}

/// Three classes, three tiers: flavour is quieter, mechanical is ordinary, and
/// a milestone is marked. A player who cannot tell them apart is reading one
/// undifferentiated stream, which is the thing the classification exists to
/// prevent.
/// @spec TUI-NAV-057
#[test]
fn the_three_classes_are_told_apart_on_the_ticker() {
    let flavor = Event::Flavor { template: 0, kitten: None };
    let mechanical = Event::SeasonChanged { season: Season::Winter };
    let milestone = Event::WhaleSummoned;
    assert_eq!(flavor.class(), EventClass::Flavor, "fixture");
    assert_eq!(mechanical.class(), EventClass::Mechanical, "fixture");
    assert_eq!(milestone.class(), EventClass::Milestone, "fixture");

    let (quiet, ordinary, marked) = (tier_of(flavor), tier_of(mechanical), tier_of(milestone));
    assert_ne!(
        quiet, ordinary,
        "flavour and mechanical occurrences opened their lines identically; flavour is the \
         quieter of the two, and something on the line has to say so"
    );
    assert_ne!(marked, ordinary, "a milestone is marked, and an ordinary occurrence is not");
    assert_ne!(marked, quiet, "a milestone is marked, and flavour is the quietest tier");
}

// ---------------------------------------------------------------------------
// TUI-NAV-038 — the palette, over revealed content only
// ---------------------------------------------------------------------------

/// The palette opens over all revealed content and narrows as characters
/// arrive. Both halves are asserted against names the *query* never contains,
/// so a palette that echoes what was typed cannot pass by accident.
/// @spec TUI-NAV-038
#[test]
fn the_palette_narrows_by_substring_as_characters_are_typed() {
    let (kept, dropped) = (StructureId::ALL[0], StructureId::ALL[1]);
    let (kept_probe, dropped_probe) = (probe(kept), probe(dropped));
    assert!(
        !kept_probe.contains(&dropped_probe) && !dropped_probe.contains(&kept_probe),
        "fixture: the two probes must not match each other"
    );
    let state = revealing(&[kept, dropped]);

    let opened = palette_text(&state, "");
    assert!(
        mentions(&opened, &kept_probe) && mentions(&opened, &dropped_probe),
        "an opened palette is over all revealed content, and one of the two revealed structures \
         was not in it. The palette read as:\n{opened}"
    );

    let narrowed = palette_text(&state, &kept_probe);
    assert!(
        !mentions(&narrowed, &dropped_probe),
        "a query matching only one revealed structure still listed the other; the palette narrows \
         its results by substring as characters are typed. The palette read as:\n{narrowed}"
    );
}

/// The palette is fast travel over what the player has found, never a way to
/// read ahead. Content the core has not revealed is not in it, however
/// precisely it is spelled.
///
/// The claim is made as a count rather than an absence, because a palette may
/// legitimately echo the query itself: revealing the content must make its
/// name appear *more* times than it did while hidden.
/// @spec TUI-NAV-038
#[test]
fn the_palette_reaches_only_what_the_core_has_revealed() {
    let seen = StructureId::ALL[0];
    let unseen = StructureId::ALL[7];
    let unseen_probe = probe(unseen);

    let mut state = revealing(&[seen]);
    assert_eq!(
        state.availability(ContentId::Structure(unseen)),
        Availability::Hidden,
        "fixture: the second structure is not revealed"
    );

    let while_hidden = palette_text(&state, &unseen_probe);
    let hidden_hits = occurrences(&while_hidden, &unseen_probe);

    state.revealed.insert(ContentId::Structure(unseen));
    assert_ne!(
        state.availability(ContentId::Structure(unseen)),
        Availability::Hidden,
        "fixture: now it is revealed"
    );
    let once_revealed = palette_text(&state, &unseen_probe);
    let revealed_hits = occurrences(&once_revealed, &unseen_probe);

    assert!(
        revealed_hits > hidden_hits,
        "the same query found the content {revealed_hits} time(s) once revealed and \
         {hidden_hits} time(s) while hidden; a query that finds nothing new when content is \
         revealed is not searching revealed content at all. Hidden read as:\n{while_hidden}"
    );
    assert!(
        hidden_hits <= 1,
        "unrevealed content was listed in the palette; the palette searches revealed content \
         only, and reading ahead through it would undo progressive reveal. It read as:\n\
         {while_hidden}"
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-027 / 050 / 055 / 056 — the returning player's report
// ---------------------------------------------------------------------------

/// The one line carries the elapsed span, the largest gains, and the arrivals
/// — and, at the floor width, it has to leave the small change out. A banner
/// that named every resource that moved would not be a glance.
/// @spec TUI-NAV-055
#[test]
fn the_banner_carries_the_span_the_largest_gains_and_the_arrivals() {
    let mut report = graded_report(2 * 86_400 + 4 * 3_600);
    report.arrived = named(&["Zibbly", "Quillow", "Mopsy"]);

    let state = away(&[(9_000, report.clone())]);
    let view = unread(&state);
    let line = banner_text(&state, &view);

    assert!(
        mentions(&line, &format!("{DOMINANT:?}")),
        "the banner did not name the resource that grew most over the absence, which is what \
         TUI-NAV-055 puts on the line. It read as {:?}",
        line.trim_end()
    );
    assert!(
        !mentions(&line, &format!("{TINIEST:?}")),
        "the banner spent part of its one line on the smallest gain in the report; the line is \
         the *largest* gains, so it has to choose. It read as {:?}",
        line.trim_end()
    );

    // The span is on the line: two absences of different lengths, alike in
    // everything else, cannot draw the same banner.
    let mut longer = report.clone();
    longer.elapsed_seconds = 40 * 86_400;
    let long_absence = away(&[(9_000, longer)]);
    assert_ne!(
        banner(&state, &view),
        banner(&long_absence, &unread(&long_absence)),
        "a two-day absence and a forty-day one drew the same banner; the elapsed span is part of \
         what fills the line"
    );

    // And so are the arrivals.
    let mut nobody_came = report;
    nobody_came.arrived.clear();
    let quiet = away(&[(9_000, nobody_came)]);
    assert_ne!(
        banner(&state, &view),
        banner(&quiet, &unread(&quiet)),
        "three kittens arrived while the player was away and the banner drew exactly as it did \
         when none had; arrivals belong on the line"
    );
}

/// The first thing a returning player sees leads with what grew. Departures
/// and absences are left to the pinned entry — so the banner must be *unmoved*
/// by who wandered off, cell for cell, and must show no number going down.
/// @spec TUI-NAV-055, TUI-NAV-056
#[test]
fn the_banner_says_nothing_about_who_wandered_off() {
    let mut homebound = graded_report(2 * 86_400);
    homebound.arrived = named(&["Zibbly"]);
    let mut wandering = homebound.clone();
    wandering.wandered = vec![("Quillow".to_string(), 1), ("Mopsy".to_string(), 2)];

    let settled = away(&[(9_000, homebound)]);
    let scattered = away(&[(9_000, wandering)]);

    assert_eq!(
        banner(&settled, &unread(&settled)),
        banner(&scattered, &unread(&scattered)),
        "two kittens wandering off changed the banner; departures and absences belong to the \
         pinned Journal entry, so that the glance a returning player comes back to leads with \
         what grew and never with what left"
    );

    let line = banner_text(&scattered, &unread(&scattered));
    assert!(
        !shows_a_decrease(&line),
        "the banner shows a number going down: {:?}. A departure is never rendered as a loss or \
         a count decrease",
        line.trim_end()
    );
    assert!(
        loss_vocabulary(&line).is_empty(),
        "the banner reads {:?}, which speaks of {:?}; nothing in clowder is ever lost",
        line.trim_end(),
        loss_vocabulary(&line)
    );
}

/// Dismissing the banner loses nothing: exactly what the one line declined to
/// say is in the pinned Journal entry.
/// @spec TUI-NAV-027
#[test]
fn what_the_banner_left_out_is_waiting_in_the_pinned_entry() {
    let mut homebound = graded_report(2 * 86_400);
    homebound.arrived = named(&["Zibbly"]);
    let mut wandering = homebound.clone();
    wandering.wandered = vec![("Quillow".to_string(), 1), ("Mopsy".to_string(), 2)];

    let settled = away(&[(9_000, homebound)]);
    let scattered = away(&[(9_000, wandering)]);

    assert!(
        journal_differs_somewhere(&settled, &scattered),
        "the banner declined to mention the wandering and the pinned entry did not mention it \
         either, so it is gone; the pinned entry carries the full folded detail, which is what \
         makes dismissing the banner cost nothing"
    );
}

/// A wandering is a present absence that reverses itself. The pinned entry
/// names where they went and says they come back — and never speaks of loss.
/// @spec TUI-NAV-056
#[test]
fn an_absence_is_told_as_a_visit_that_reverses_itself() {
    let mut report = graded_report(2 * 86_400);
    report.wandered = vec![("Quillow".to_string(), 1)];
    let state = away(&[(9_000, report)]);

    let entry = journal_text(&state, 0);
    assert!(!entry.trim().is_empty(), "fixture: the pinned entry is on screen at the first anchor");

    assert!(
        mentions(&entry, "neighb"),
        "the pinned entry does not name where the kitten went; the only place a kitten is ever \
         away to is the neighbour's yard, and TUI-NAV-056 says the absence is named. It read as:\n\
         {entry}"
    );
    assert!(
        ["back", "return", "home"].iter().any(|w| mentions(&entry, w)),
        "the pinned entry does not say the kitten comes back; an absence is reported as something \
         that reverses on its own, never as a departure. It read as:\n{entry}"
    );
    assert!(
        loss_vocabulary(&entry).is_empty(),
        "the pinned entry speaks of {:?}; a kitten visiting the neighbours has not been lost, and \
         nothing in clowder ever is. It read as:\n{entry}",
        loss_vocabulary(&entry)
    );
}

/// More than one unread report queues newest-first, and every one is pinned —
/// so a second absence never pushes the first out of the game unread.
/// @spec TUI-NAV-050, TUI-NAV-027
#[test]
fn unread_reports_queue_newest_first_and_every_one_stays_pinned() {
    let (older_gain, newer_gain) = (ResourceId::Feathers, ResourceId::Catnip);
    let older = report_gaining(3 * 86_400, older_gain);
    let newer = report_gaining(2 * 3_600, newer_gain);

    let state = away(&[(1_000, older), (9_000, newer)]);
    let mut view = unread(&state);
    assert_eq!(view.reports.len(), 2, "fixture: two reports are waiting");

    let first = banner_text(&state, &view);
    assert!(
        mentions(&first, &format!("{newer_gain:?}")),
        "the banner is showing something other than the newest unread report. It read as {:?}",
        first.trim_end()
    );
    assert!(
        !mentions(&first, &format!("{older_gain:?}")),
        "both reports are on the one banner line at once; they queue, newest first. It read as \
         {:?}",
        first.trim_end()
    );

    view.reports[0].dismissed = true;
    let next = banner_text(&state, &view);
    assert!(
        mentions(&next, &format!("{older_gain:?}")),
        "dismissing the newest report left no banner for the one behind it, so that absence was \
         displaced unseen. It read as {:?}",
        next.trim_end()
    );

    // ...and both are pinned, whether or not their banners were ever read.
    let newest_only = away(&[(9_000, report_gaining(2 * 3_600, newer_gain))]);
    assert!(
        journal_differs_somewhere(&state, &newest_only),
        "a Journal holding two pinned reports drew exactly as one holding only the newer; every \
         report is pinned as its own entry"
    );
}

/// A nap is the other way bulk time passes, and it takes the same path: one
/// banner, one pinned entry, nothing special-cased for how long the sleep was.
/// @spec TUI-NAV-027
#[test]
fn a_nap_sized_report_takes_the_same_path_as_an_overnight_one() {
    let mut nap = graded_report(5 * 365 * 86_400);
    nap.seasons_passed = 20;
    nap.arrived = named(&["Zibbly", "Mopsy"]);

    let state = away(&[(50_000, nap)]);
    let view = unread(&state);

    let line = banner_text(&state, &view);
    assert!(
        mentions(&line, &format!("{DOMINANT:?}")),
        "a years-long nap's report drew no gain on its banner; a nap reuses the away-report's \
         path rather than a path of its own. It read as {:?}",
        line.trim_end()
    );
    assert!(
        !journal_text(&state, 0).trim().is_empty(),
        "a nap's report is pinned in the Journal like any other bulk-time report"
    );
}

// ---------------------------------------------------------------------------
// Nothing here blocks
// ---------------------------------------------------------------------------

/// Every transient draws inside the area it was given and nowhere else.
///
/// This is the seam that makes "the sim keeps ticking and the chrome keeps
/// rendering behind every one of them" true: an overlay that painted over the
/// header, the ticker or the hint bar would have stopped the world for as long
/// as it was open, and an overlay that stops the world is an obligation.
/// @spec TUI-NAV-026, TUI-NAV-027, TUI-NAV-038
#[test]
fn a_transient_draws_inside_its_own_area_and_leaves_the_chrome_alone() {
    let mut state = journaled(&filler(30));
    state.revealed = StructureId::ALL[..3].iter().map(|s| ContentId::Structure(*s)).collect();
    state.journal.record_away(9_000, &graded_report(2 * 86_400));
    let reports = unread(&state);
    let journal = journal_view(3);
    let palette = ViewState {
        overlay: Some(Overlay::Palette { query: String::new(), selected: 0 }),
        ..ViewState::default()
    };

    assert_confined("the ticker", ticker_rect(), |f| overlays::ticker(f, ticker_rect(), &state));
    assert_confined("the Journal", overlay_rect(), |f| {
        overlays::journal(f, overlay_rect(), &state, &journal)
    });
    assert_confined("the palette", overlay_rect(), |f| {
        overlays::palette(f, overlay_rect(), &state, &palette)
    });
    assert_confined("the banner", banner_rect(), |f| {
        overlays::banner(f, banner_rect(), &state, &reports)
    });
}

/// Draw a sentinel over the whole terminal, then the transient over its rect,
/// and demand every cell outside that rect survive untouched.
fn assert_confined(what: &str, rect: Rect, render: impl FnOnce(&mut Frame)) {
    const SENTINEL: &str = "#";
    let buffer = drawn(|f| {
        let area = f.area();
        let fill = vec![Line::from(SENTINEL.repeat(area.width as usize)); area.height as usize];
        f.render_widget(Paragraph::new(fill), area);
        render(f);
    });

    for y in 0..FLOOR_H {
        for x in 0..FLOOR_W {
            if rect.contains(Position::new(x, y)) {
                continue;
            }
            assert_eq!(
                buffer.cell((x, y)).expect("every cell of the terminal exists").symbol(),
                SENTINEL,
                "{what} drew at ({x}, {y}), outside the {rect:?} it was given; the chrome keeps \
                 rendering behind every transient, so nothing here may paint over it"
            );
        }
    }
}
