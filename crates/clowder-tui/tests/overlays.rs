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
//! Lexical assertions are made only where a spec or the design names the
//! thing outright: the resource behind a *gain*, by the name its stores row
//! displays (TUI-NAV-055); the banner's two hints, by the words the design
//! gives them (`e for the tale`, `Esc dismiss`); a count, as the number it
//! must read; a truncated span, as *more than* it; the destination and the
//! return of a wandering (TUI-NAV-056 says the entry names where they went and
//! that they come back); and two forbidden vocabularies — loss, and words that
//! say where a kitten is *now* (TUI-NAV-056 forbids both). A content name is
//! matched by a uniqueness-checked prefix or as a whole word, so an ellipsis
//! or a longer name containing it cannot turn a hit into a miss or a miss
//! into a hit.
//!
//! **Geometry comes from the layout, not from this file.** The ticker rect is
//! the row `render::layout` gives the ticker and the banner rect is the row it
//! gives the banner, both at the 80×24 floor — which is what makes "one line"
//! a real constraint the curation has to fit inside rather than a number
//! chosen here.
//!
//! **Not tested here.** The help overlay is TUI-NAV-025, which belongs to the
//! discoverability chunk; `overlays::help` is deliberately left alone. The
//! palette's Enter (move to the hit's section and focus it) is input, not
//! drawing, and lives with `nav::update`'s tests. Which reports queue a
//! banner, what retires or keeps the one showing, and where a pinned entry
//! sits before and after its report is read are a session's doing, tested
//! through the loop in `report_queue.rs`.

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
    chrome(true)
        .banner
        .expect("the layout gives a banner row while a report is unread")
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
    term.draw(|frame| render(frame))
        .expect("TestBackend is infallible");
    term.backend().buffer().clone()
}

fn ticker(state: &GameState) -> Buffer {
    drawn(|f| overlays::ticker(f, ticker_rect(), state))
}

fn journal_view(anchor: usize) -> ViewState {
    ViewState {
        overlay: Some(Overlay::Journal { anchor }),
        ..ViewState::default()
    }
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
        overlay: Some(Overlay::Palette {
            query: query.to_string(),
            selected: 0,
        }),
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
    LOSS_WORDS
        .iter()
        .filter(|w| present.contains(**w))
        .map(|w| w.to_string())
        .collect()
}

/// Whether the surface shows a number going down: a minus sign immediately
/// followed by a digit.
fn shows_a_decrease(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    chars
        .windows(2)
        .any(|pair| pair[0] == '-' && pair[1].is_ascii_digit())
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
    TierMark {
        symbols: symbols.trim().to_string(),
        styles,
    }
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
                Event::StructureBuilt {
                    id: StructureId::ALL[i % StructureId::ALL.len()],
                    count: 2,
                },
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
        reports: (0..entries)
            .rev()
            .map(|entry| PendingReport {
                entry,
                dismissed: false,
            })
            .collect(),
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
    let mut report = OfflineReport {
        elapsed_seconds,
        ..OfflineReport::default()
    };
    report.resource_deltas[DOMINANT.ix()] = 1_240_000.0;
    report.resource_deltas[TINIEST.ix()] = 1.0;
    for r in ResourceId::ALL
        .into_iter()
        .filter(|r| *r != DOMINANT && *r != TINIEST)
        .take(12)
    {
        report.resource_deltas[r.ix()] = 20.0;
    }
    report
}

/// An away-report that moved exactly one resource, so what the banner names
/// says unambiguously which report it is showing.
fn report_gaining(elapsed_seconds: u64, gain: ResourceId) -> OfflineReport {
    let mut report = OfflineReport {
        elapsed_seconds,
        ..OfflineReport::default()
    };
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
    assert_eq!(
        clashes, 0,
        "fixture: {name:?} does not identify one structure"
    );
    name
}

/// A meadow that has revealed exactly these structures and nothing else.
fn revealing(structures: &[StructureId]) -> GameState {
    let mut state = GameState::new(11);
    state.revealed = structures
        .iter()
        .map(|s| ContentId::Structure(*s))
        .collect();
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
    season.push((
        900,
        Event::SeasonChanged {
            season: Season::Winter,
        },
    ));
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
    let mut earlier = vec![(
        1,
        Event::SeasonChanged {
            season: Season::Winter,
        },
    )];
    earlier.extend(filler(30));
    let mut other = vec![(1, Event::ZoomiesEnded)];
    other.extend(filler(30));

    let a = journaled(&earlier);
    let b = journaled(&other);
    assert_eq!(
        a.journal.iter_chronological().len(),
        31,
        "fixture: one differing oldest entry"
    );

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
    assert_eq!(
        researched.class(),
        EventClass::Milestone,
        "fixture: the core keeps this forever"
    );
    assert_eq!(
        summoned.class(),
        EventClass::Milestone,
        "fixture: and this one"
    );

    let mut with_idea = vec![(1, researched)];
    with_idea.extend(filler(40));
    let mut with_whale = vec![(1, summoned)];
    with_whale.extend(filler(40));

    let a = journaled(&with_idea);
    let b = journaled(&with_whale);
    assert_eq!(
        a.journal.kept.len(),
        1,
        "fixture: the milestone is in the kept list"
    );
    assert_eq!(
        a.journal.recent.len(),
        40,
        "fixture: the rest are in the rolling window"
    );

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
        (
            2,
            Event::ElderAppointed {
                name: "Quillow".to_string(),
            },
        ),
    ];
    entries.extend(filler(20));
    let mut before = journaled(&entries);
    before
        .journal
        .record_away(500, &graded_report(2 * 86_400 + 4 * 3_600));

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
    let state = journaled(&[(
        100,
        Event::SeasonChanged {
            season: Season::Winter,
        },
    )]);
    let buffer = ticker(&state);
    let rect = ticker_rect();
    let line = text_of(&buffer, rect);
    assert!(
        !line.trim().is_empty(),
        "fixture: the ticker drew something for an occurrence"
    );

    // The line's opening mark may carry its tier's own colour; everything
    // after it is the event's text, and the text is the part that must recede.
    // (A resource a line names wears its coat instead, TUI-THEME-023; this
    // occurrence names none.)
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
    assert_eq!(
        first.class(),
        EventClass::Milestone,
        "fixture: the core marks the first copy"
    );
    assert_eq!(
        later.class(),
        EventClass::Mechanical,
        "fixture: and not the ones after it"
    );

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
        Event::KittenArrived {
            name: "Zibbly".to_string(),
        },
        Event::SeasonChanged {
            season: Season::Winter,
        },
    ];
    let remembered = [
        Event::IdeaResearched { id: IdeaId::ALL[0] },
        Event::KittenPromoted {
            name: "Mopsy".to_string(),
            rank: 1,
        },
    ];

    for pair in [ordinary, remembered] {
        let [a, b] = pair;
        assert_eq!(
            a.class(),
            b.class(),
            "fixture: the core classes these two alike"
        );
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
    let flavor = Event::Flavor {
        template: 0,
        kitten: None,
    };
    let mechanical = Event::SeasonChanged {
        season: Season::Winter,
    };
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
    assert_ne!(
        marked, ordinary,
        "a milestone is marked, and an ordinary occurrence is not"
    );
    assert_ne!(
        marked, quiet,
        "a milestone is marked, and flavour is the quietest tier"
    );
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
    assert!(
        !entry.trim().is_empty(),
        "fixture: the pinned entry is on screen at the first anchor"
    );

    assert!(
        mentions(&entry, "neighb"),
        "the pinned entry does not name where the kitten went; the only place a kitten is ever \
         away to is the neighbour's yard, and TUI-NAV-056 says the absence is named. It read as:\n\
         {entry}"
    );
    assert!(
        ["back", "return", "home"]
            .iter()
            .any(|w| mentions(&entry, w)),
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
    state.revealed = StructureId::ALL[..3]
        .iter()
        .map(|s| ContentId::Structure(*s))
        .collect();
    state.journal.record_away(9_000, &graded_report(2 * 86_400));
    let reports = unread(&state);
    let journal = journal_view(3);
    let palette = ViewState {
        overlay: Some(Overlay::Palette {
            query: String::new(),
            selected: 0,
        }),
        ..ViewState::default()
    };

    assert_confined("the ticker", ticker_rect(), |f| {
        overlays::ticker(f, ticker_rect(), &state)
    });
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
                buffer
                    .cell((x, y))
                    .expect("every cell of the terminal exists")
                    .symbol(),
                SENTINEL,
                "{what} drew at ({x}, {y}), outside the {rect:?} it was given; the chrome keeps \
                 rendering behind every transient, so nothing here may paint over it"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-055 — what the report banner's one line holds, at every width
//
// The line is read at a width wide enough for everything (`WIDE`) unless a
// test is about narrowing it. Clauses are located the way the spec names
// them: a gain by its resource's name, the arrivals by their count, the hints
// by the words the design gives them (`e for the tale`, `Esc dismiss`), and the
// span by a differential — two reports alike but for their span first differ
// where the span is drawn.
// ---------------------------------------------------------------------------

/// Wide enough for the whole line of any report these tests build.
const WIDE: u16 = 240;

/// The banner drawn into a one-row pane `width` columns wide, trailing blanks
/// dropped. Every UI symbol is single-width (TUI-NAV-021), so the line's
/// character count is the number of columns it takes.
fn banner_at(state: &GameState, view: &ViewState, width: u16) -> String {
    let rect = Rect::new(0, 0, width, 1);
    let mut term = Terminal::new(TestBackend::new(width, 1)).expect("TestBackend is infallible");
    term.draw(|f| overlays::banner(f, rect, state, view))
        .expect("TestBackend is infallible");
    region_lines(term.backend().buffer(), rect)
        .join("")
        .trim_end()
        .to_string()
}

fn columns(line: &str) -> usize {
    line.chars().count()
}

/// The whole line for a single report, at `WIDE`.
fn full_line(report: &OfflineReport) -> String {
    let state = away(&[(9_000, report.clone())]);
    banner_at(&state, &unread(&state), WIDE)
}

/// Where `word` first stands on its own in `text` — case ignored, and never
/// inside a longer word, so `Dew` is not found in `Dewdrop`.
fn find_word(text: &str, word: &str) -> Option<usize> {
    let hay = text.to_lowercase();
    let needle = word.to_lowercase();
    if needle.is_empty() {
        return None;
    }
    let bytes = hay.as_bytes();
    let mut from = 0;
    while let Some(off) = hay[from..].find(&needle) {
        let at = from + off;
        let end = at + needle.len();
        let before_ok = at == 0 || !(bytes[at - 1] as char).is_ascii_alphanumeric();
        let after_ok = end >= bytes.len() || !(bytes[end] as char).is_ascii_alphanumeric();
        if before_ok && after_ok {
            return Some(at);
        }
        from = at + 1;
        while from < hay.len() && !hay.is_char_boundary(from) {
            from += 1;
        }
    }
    None
}

/// Where a run of digits reading exactly `digits` first stands in `text` — a
/// whole number, never part of a longer one or of a decimal.
fn find_number(text: &str, digits: &str) -> Option<usize> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].1.is_ascii_digit() {
            let start = i;
            while i < chars.len() && chars[i].1.is_ascii_digit() {
                i += 1;
            }
            let run: String = chars[start..i].iter().map(|(_, c)| *c).collect();
            let glued_before = start > 0 && chars[start - 1].1 == '.';
            let glued_after = i < chars.len()
                && chars[i].1 == '.'
                && i + 1 < chars.len()
                && chars[i + 1].1.is_ascii_digit();
            if run == digits && !glued_before && !glued_after {
                return Some(chars[start].0);
            }
        } else {
            i += 1;
        }
    }
    None
}

/// The runs of digits in `text`, in order.
fn digit_runs(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_ascii_digit())
        .filter(|r| !r.is_empty())
        .map(String::from)
        .collect()
}

/// The name a resource's stores row displays — the oracle for "each resource
/// named as its stores row names it". Read from the palette, whose results
/// carry the name each item's rows display (TUI-NAV-063), for a meadow that
/// holds some of `r`, so its stores row is listed at all.
fn stores_name(r: ResourceId) -> String {
    use clowder_tui::nav::RowKey;
    let mut s = GameState::new(11);
    s.resources[r.ix()] = 1.0;
    clowder_tui::nav::palette::results(&s, "")
        .into_iter()
        .find(|p| p.key == RowKey::Resource(r))
        .map(|p| p.name)
        .unwrap_or_else(|| panic!("fixture: a meadow holding {r:?} lists its stores row"))
}

/// Where a resource is first named in `text`, by its stores-row name or its
/// identifier's spelling, whichever comes first — for surfaces the spec does
/// not tie to the stores row's spelling.
fn find_resource(text: &str, r: ResourceId) -> Option<usize> {
    let debug = format!("{r:?}");
    [stores_name(r), debug]
        .iter()
        .filter_map(|n| find_word(text, n))
        .min()
}

/// The first column at which two lines differ.
fn first_difference(a: &str, b: &str) -> Option<usize> {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    (0..a.len().max(b.len())).find(|i| a.get(*i) != b.get(*i))
}

/// A report whose line has every clause: a span, three gains of distinct
/// sizes listed out of the resources' own order, and arrivals counted more
/// than once per name. No number on it collides with the arrivals' sum.
fn full_report() -> OfflineReport {
    let mut report = OfflineReport {
        elapsed_seconds: 3 * 86_400 + 5 * 3_600,
        ..OfflineReport::default()
    };
    report.resource_deltas[ResourceId::Twigs.ix()] = 9_600.0;
    report.resource_deltas[ResourceId::Feathers.ix()] = 4_800.0;
    report.resource_deltas[ResourceId::Catnip.ix()] = 1_500.0;
    report.arrived = vec![("Zibbly".to_string(), 6), ("Quillow".to_string(), 8)];
    report
}

/// The arrivals' sum in `full_report`.
const ARRIVALS_SUM: &str = "14";

/// The line runs, in order: how much time passed, the gains largest first,
/// the arrivals, then the hint for the tale and the hint for dismissing.
/// @spec TUI-NAV-055
#[test]
fn the_banner_line_runs_span_then_gains_then_arrivals_then_the_hints() {
    let report = full_report();
    let line = full_line(&report);

    let mut longer = report.clone();
    longer.elapsed_seconds = 40 * 86_400;
    let span_at = first_difference(&line, &full_line(&longer)).unwrap_or_else(|| {
        panic!("the line does not change with the span, so it does not say how much time passed: {line:?}")
    });
    let span_at = line
        .char_indices()
        .nth(span_at)
        .map(|(b, _)| b)
        .unwrap_or(line.len());

    let at = |what: &str, found: Option<usize>| {
        found.unwrap_or_else(|| panic!("the banner has no {what} on it. It read as {line:?}"))
    };
    let order = [
        ("span", span_at),
        (
            "largest gain",
            at("largest gain", find_word(&line, "twigs")),
        ),
        (
            "second gain",
            at("second gain", find_word(&line, "feathers")),
        ),
        ("third gain", at("third gain", find_word(&line, "catnip"))),
        (
            "arrivals count",
            at("arrivals count", find_number(&line, ARRIVALS_SUM)),
        ),
        (
            "hint for the tale",
            at("hint for the tale", find_word(&line, "tale")),
        ),
        (
            "hint for dismissing",
            at("hint for dismissing", find_word(&line, "esc")),
        ),
    ];
    for pair in order.windows(2) {
        assert!(
            pair[0].1 < pair[1].1,
            "the {} comes after the {} on the banner; the line holds, in order, how much time \
             passed, the largest gains, the arrivals, and the hints. It read as {line:?}",
            pair[0].0,
            pair[1].0
        );
    }
}

/// Each gain is named as the player already knows the resource — by the name
/// its stores row displays — for every resource there is.
/// @spec TUI-NAV-055
#[test]
fn every_gain_on_the_banner_is_named_as_its_stores_row_names_it() {
    for r in ResourceId::ALL {
        let mut report = OfflineReport {
            elapsed_seconds: 2 * 86_400,
            ..OfflineReport::default()
        };
        report.resource_deltas[r.ix()] = 4_800.0;
        let line = full_line(&report);
        let name = stores_name(r);
        assert!(
            line.contains(&name),
            "the banner named the only gain in the report some other way than its stores row \
             does ({name:?}); the line names each resource as its stores row names it. It read \
             as {line:?}"
        );
    }
}

/// At most three gains, the largest first by amount, and only stocks that grew
/// — never one that fell, and never the fourth-largest gain.
/// @spec TUI-NAV-055
#[test]
fn the_banner_shows_at_most_three_gains_largest_first_and_only_positive_ones() {
    // Amounts deliberately out of the resources' own order, so "largest
    // first" cannot be satisfied by listing in declaration order.
    let mut report = OfflineReport {
        elapsed_seconds: 2 * 86_400,
        ..OfflineReport::default()
    };
    let gains = [
        (ResourceId::Pebbles, 900.0),
        (ResourceId::Twigs, 700.0),
        (ResourceId::Catnip, 500.0),
        (ResourceId::Fluff, 300.0),
        (ResourceId::Feathers, 100.0),
    ];
    for (r, d) in gains {
        report.resource_deltas[r.ix()] = d;
    }
    // Falls far larger than any gain: largest by size is not what earns a place.
    report.resource_deltas[ResourceId::Buttons.ix()] = -50_000.0;
    report.resource_deltas[ResourceId::Embers.ix()] = -20_000.0;

    let line = full_line(&report);
    let shown: Vec<usize> = gains[..3]
        .iter()
        .map(|(r, _)| {
            find_resource(&line, *r).unwrap_or_else(|| {
                panic!("the banner left out {r:?}, one of the three largest gains. It read as {line:?}")
            })
        })
        .collect();
    assert!(
        shown.windows(2).all(|p| p[0] < p[1]),
        "the three largest gains are not listed largest first. It read as {line:?}"
    );
    for (r, _) in &gains[3..] {
        assert!(
            find_resource(&line, *r).is_none(),
            "the banner lists {r:?}, the fourth or fifth largest gain; it shows at most three. It \
             read as {line:?}"
        );
    }
    for r in [ResourceId::Buttons, ResourceId::Embers] {
        assert!(
            find_resource(&line, r).is_none(),
            "the banner names {r:?}, a stock that fell; only positive net changes are gains. It \
             read as {line:?}"
        );
    }
    assert!(
        !shows_a_decrease(&line),
        "the banner shows a number going down: {line:?}"
    );
}

/// No gain on the line reads as nothing: whatever amount a gain is given, it
/// shows as more than zero, or the gain is left off the line.
/// @spec TUI-NAV-055
#[test]
fn no_gain_on_the_banner_reads_as_zero() {
    let tiny = [
        (ResourceId::Twigs, 1.0e-9),
        (ResourceId::Pebbles, 1.0e-300),
        (ResourceId::Fluff, f64::MIN_POSITIVE),
        (ResourceId::Buttons, 4.0e-5),
    ];
    for (r, d) in tiny {
        let mut report = OfflineReport {
            elapsed_seconds: 2 * 86_400,
            ..OfflineReport::default()
        };
        report.resource_deltas[ResourceId::Catnip.ix()] = 4_800.0;
        report.resource_deltas[r.ix()] = d;
        let line = full_line(&report);
        let Some(at) = find_resource(&line, r) else {
            continue;
        };
        // The clause the name stands in: between the separators around it.
        let start = line[..at]
            .rfind([',', '\u{b7}', ':', ';'])
            .map(|i| i + 1)
            .unwrap_or(0);
        let end = line[at..]
            .find([',', '\u{b7}', ';'])
            .map(|i| at + i)
            .unwrap_or(line.len());
        let clause = &line[start..end];
        assert!(
            clause.chars().any(|c| ('1'..='9').contains(&c)),
            "a gain of {d:e} {r:?} is on the banner as {clause:?}, which reads as nothing gained; \
             a gain too small to show as more than zero stays off the line. It read as {line:?}"
        );
    }
}

/// The arrivals are counted as the sum of the report's per-name counts — a
/// kitten who arrived twice is two arrivals — never as the number of names.
/// @spec TUI-NAV-055
#[test]
fn the_banner_counts_arrivals_as_the_sum_of_the_per_name_counts() {
    let mut report = OfflineReport {
        elapsed_seconds: 3 * 86_400 + 5 * 3_600,
        ..OfflineReport::default()
    };
    report.resource_deltas[ResourceId::Catnip.ix()] = 4_800.0;
    report.arrived = vec![("Zibbly".to_string(), 6), ("Quillow".to_string(), 8)];
    let line = full_line(&report);

    assert!(
        find_number(&line, ARRIVALS_SUM).is_some(),
        "two names arrived six and eight times and the banner does not say fourteen kittens \
         arrived; the arrivals are the sum of the per-name counts. It read as {line:?}"
    );
    assert!(
        find_number(&line, "2").is_none(),
        "the banner counts the arrivals' names (two) rather than the arrivals. It read as {line:?}"
    );
    for name in ["Zibbly", "Quillow"] {
        assert!(
            !mentions(&line, name),
            "the banner names an arrival; the line carries the number who arrived and leaves the \
             names to the pinned entry. It read as {line:?}"
        );
    }
}

/// A truncated absence is a floor, not the absence: the line says it was more
/// than the span the core reports, and otherwise reads the same.
/// @spec TUI-NAV-055
#[test]
fn a_truncated_absence_reads_as_more_than_its_span() {
    let mut exact = OfflineReport {
        elapsed_seconds: 3 * 86_400,
        ..OfflineReport::default()
    };
    exact.resource_deltas[ResourceId::Catnip.ix()] = 4_800.0;
    let mut floor = exact.clone();
    floor.truncated = true;

    let (exact_line, floor_line) = (full_line(&exact), full_line(&floor));
    assert_ne!(
        exact_line, floor_line,
        "a truncated report's banner reads exactly as an exact one's; where the core stopped \
         counting, the span is stated as more than that span"
    );
    assert!(
        ["more than", "over", ">"]
            .iter()
            .any(|m| floor_line.to_lowercase().contains(m)),
        "the truncated report's banner does not say the absence was more than its span. It read \
         as {floor_line:?}"
    );
    let kept: BTreeSet<String> = words(&exact_line);
    let missing: Vec<&String> = kept
        .iter()
        .filter(|w| !words(&floor_line).contains(*w))
        .collect();
    assert!(
        missing.is_empty(),
        "stating the span as more than it lost {missing:?} from the line; the span itself, the \
         gains and the hints all stay. It read as {floor_line:?}"
    );
}

/// With no stock grown and no kitten arrived, a quiet note stands where the
/// gains would be, and the line still points at the tale. Nothing that fell or
/// left changes it.
/// @spec TUI-NAV-055
#[test]
fn with_nothing_grown_and_no_one_arrived_a_quiet_note_stands_in_for_the_gains() {
    let span = 3 * 86_400 + 5 * 3_600;
    let quiet = OfflineReport {
        elapsed_seconds: span,
        ..OfflineReport::default()
    };
    let mut grew = quiet.clone();
    grew.resource_deltas[ResourceId::Catnip.ix()] = 4_800.0;

    let (quiet_line, grew_line) = (full_line(&quiet), full_line(&grew));
    let note = note_words(&quiet_line, &grew_line);
    assert!(
        !note.is_empty(),
        "with nothing grown and no one arrived, the banner puts nothing where the gains would \
         be; a quiet note stands in their place. It read as {quiet_line:?}"
    );
    assert!(
        find_word(&quiet_line, "tale").is_some(),
        "the quiet line no longer points at the tale: {quiet_line:?}"
    );

    // Arrivals are something to tell: no quiet note beside them.
    let mut arrived = quiet.clone();
    arrived.arrived = vec![("Zibbly".to_string(), 2)];
    let arrived_line = full_line(&arrived);
    assert!(
        !note.iter().all(|w| find_word(&arrived_line, w).is_some()),
        "the quiet note shows beside arrivals; it stands in only where no stock grew and no \
         kitten arrived. It read as {arrived_line:?}"
    );

    // Falls, departures, homecomings and seasons are not growth, and do not
    // move the line at all.
    let mut fell = quiet.clone();
    for r in ResourceId::ALL {
        fell.resource_deltas[r.ix()] = -25.0;
    }
    fell.wandered = vec![("Mopsy".to_string(), 2)];
    fell.returned = vec![("Mopsy".to_string(), 1)];
    fell.seasons_passed = 9;
    let (a, b) = (away(&[(9_000, quiet)]), away(&[(9_000, fell)]));
    assert_eq!(
        banner(&a, &unread(&a)),
        banner(&b, &unread(&b)),
        "a report where every stock fell and kittens came and went drew a different banner from \
         one where nothing happened; with nothing grown and no one arrived, both are the quiet \
         note"
    );
}

/// The words a quiet line has that a line with one gain does not: the note
/// that stands in for the gains.
fn note_words(quiet_line: &str, grew_line: &str) -> Vec<String> {
    let grew = words(grew_line);
    words(quiet_line)
        .into_iter()
        .filter(|w| !grew.contains(w))
        .collect()
}

/// The line reads nothing of wanderings, homecomings, seasons passed or falling
/// stocks — each on its own, and all together, leaves it cell for cell as it
/// was.
/// @spec TUI-NAV-055
#[test]
fn the_banner_reads_nothing_of_wanderings_returns_seasons_or_falls() {
    type Variant = (&'static str, Box<dyn Fn(&mut OfflineReport)>);
    let base = full_report();
    let variants: Vec<Variant> = vec![
        (
            "wanderings",
            Box::new(|r| r.wandered = vec![("Mopsy".to_string(), 3)]),
        ),
        (
            "homecomings",
            Box::new(|r| r.returned = vec![("Mopsy".to_string(), 2)]),
        ),
        ("seasons passed", Box::new(|r| r.seasons_passed = 11)),
        (
            "falling stocks",
            Box::new(|r| {
                r.resource_deltas[ResourceId::Buttons.ix()] = -90_000.0;
                r.resource_deltas[ResourceId::Embers.ix()] = -7.0;
            }),
        ),
    ];

    let plain = away(&[(9_000, base.clone())]);
    let plain_banner = banner_at(&plain, &unread(&plain), WIDE);
    let mut everything = base.clone();
    for (what, change) in &variants {
        let mut changed = base.clone();
        change(&mut changed);
        change(&mut everything);
        let s = away(&[(9_000, changed)]);
        assert_eq!(
            banner_at(&s, &unread(&s), WIDE),
            plain_banner,
            "{what} changed the banner; the line reads nothing of wanderings, returns, seasons \
             passed, or falling stocks, leaving them to the pinned entry"
        );
    }
    let s = away(&[(9_000, everything)]);
    assert_eq!(
        banner_at(&s, &unread(&s), WIDE),
        plain_banner,
        "wanderings, returns, seasons and falls together changed the banner"
    );
}

/// A finder function for a `Clause`.
type ClauseFinder = Box<dyn Fn(&str) -> Option<usize>>;

/// A clause the narrowing checker can find on a line.
struct Clause {
    what: &'static str,
    find: ClauseFinder,
}

fn word_clause(what: &'static str, word: &'static str) -> Clause {
    Clause {
        what,
        find: Box::new(move |l| find_word(l, word)),
    }
}

/// Narrow the pane a column at a time, from wide enough for everything down to
/// the width of the span alone, and demand at every width: the clauses gone
/// are exactly the first few of `drop_order`, and no more than the width
/// forced; a clause is either wholly there or wholly gone, so a line with a
/// given set of clauses reads the same at every width that shows it; the span
/// is always there; and the clauses left keep the order they had.
fn assert_narrows_by_whole_clauses(what: &str, report: &OfflineReport, drop_order: Vec<Clause>) {
    let state = away(&[(9_000, report.clone())]);
    let view = unread(&state);
    assert_narrows_by_whole_clauses_in(what, &|w| banner_at(&state, &view, w), drop_order);
}

fn assert_narrows_by_whole_clauses_in(
    what: &str,
    render: &dyn Fn(u16) -> String,
    drop_order: Vec<Clause>,
) {
    let full = render(WIDE);
    for c in &drop_order {
        assert!(
            (c.find)(&full).is_some(),
            "{what}: the whole line has no {}: {full:?}",
            c.what
        );
    }
    let order_in = |line: &str, present: &[&Clause]| -> Vec<&'static str> {
        let mut at: Vec<(usize, &'static str)> = present
            .iter()
            .map(|c| ((c.find)(line).expect("present"), c.what))
            .collect();
        at.sort();
        at.into_iter().map(|(_, w)| w).collect()
    };

    let n = drop_order.len();
    let mut seen: Vec<Option<(String, u16)>> = vec![None; n + 1];
    let mut floor: Option<usize> = None;
    for w in (1..=WIDE).rev() {
        if floor.is_some_and(|f| (w as usize) < f) {
            break;
        }
        let line = render(w);
        let present: Vec<bool> = drop_order
            .iter()
            .map(|c| (c.find)(&line).is_some())
            .collect();
        let k = present.iter().take_while(|p| !**p).count();
        let dropped: Vec<&str> = drop_order[..k].iter().map(|c| c.what).collect();
        let stray: Vec<&str> = drop_order[k..]
            .iter()
            .zip(&present[k..])
            .filter(|(_, p)| !**p)
            .map(|(c, _)| c.what)
            .collect();
        assert!(
            stray.is_empty(),
            "{what}: at {w} columns the banner dropped {stray:?} while keeping clauses that are \
             dropped before it; the order is {:?}. It read as {line:?}",
            drop_order.iter().map(|c| c.what).collect::<Vec<_>>()
        );

        let kept: Vec<&Clause> = drop_order[k..].iter().collect();
        assert_eq!(
            order_in(&line, &kept),
            order_in(&full, &kept),
            "{what}: at {w} columns the clauses left changed their order. It read as {line:?}"
        );

        match &seen[k] {
            Some((earlier, at)) => assert_eq!(
                &line, earlier,
                "{what}: with {dropped:?} dropped, the banner reads differently at {w} columns \
                 than at {at}; a clause is dropped whole, never cut to fit"
            ),
            None => {
                if k > 0 {
                    let (with_one_more, _) = seen[k - 1].as_ref().unwrap_or_else(|| {
                        panic!(
                            "{what}: at {w} columns the banner dropped {dropped:?} at once, \
                             never showing the line with only the first {} of them dropped; \
                             it drops whole clauses one at a time, as the width requires. It \
                             read as {line:?}",
                            k - 1
                        )
                    });
                    assert!(
                        columns(with_one_more) > w as usize,
                        "{what}: at {w} columns the banner dropped the {} though the line with \
                         it ({} columns) fits; clauses are dropped only where the pane is too \
                         narrow for them. It read as {line:?}",
                        drop_order[k - 1].what,
                        columns(with_one_more)
                    );
                }
                seen[k] = Some((line.clone(), w));
            }
        }
        if k == n && floor.is_none() {
            floor = Some(columns(&line));
        }
    }

    let (span_only, _) = seen[n].clone().unwrap_or_else(|| {
        panic!("{what}: narrowing never reached the line with every droppable clause gone")
    });
    assert!(
        !span_only.trim().is_empty(),
        "{what}: with every clause dropped, the span went too"
    );
    for (line, w) in seen.iter().flatten() {
        assert!(
            line.starts_with(span_only.trim_end_matches(':')),
            "{what}: at {w} columns the line does not open with the span ({span_only:?}); the \
             span is never dropped. It read as {line:?}"
        );
    }
}

/// Too narrow for the whole line, the banner drops whole clauses in the order
/// the spec gives — the dismiss hint, the third gain, the second, the tale
/// hint, the arrivals, the largest gain — and never the span.
/// @spec TUI-NAV-055
#[test]
fn a_narrow_banner_drops_whole_clauses_in_order_and_never_the_span() {
    assert_narrows_by_whole_clauses(
        "the full line",
        &full_report(),
        vec![
            word_clause("dismiss hint", "esc"),
            word_clause("third gain", "catnip"),
            word_clause("second gain", "feathers"),
            word_clause("tale hint", "tale"),
            Clause {
                what: "arrivals",
                find: Box::new(|l| find_number(l, ARRIVALS_SUM)),
            },
            word_clause("largest gain", "twigs"),
        ],
    );
}

/// Where the quiet note stands in the gains' place, it goes last, after the
/// two hints — and the span still never does.
/// @spec TUI-NAV-055
#[test]
fn a_narrow_quiet_banner_drops_the_hints_then_the_note_and_never_the_span() {
    let span = 3 * 86_400 + 5 * 3_600;
    let quiet = OfflineReport {
        elapsed_seconds: span,
        ..OfflineReport::default()
    };
    let mut grew = quiet.clone();
    grew.resource_deltas[ResourceId::Catnip.ix()] = 4_800.0;
    let note = note_words(&full_line(&quiet), &full_line(&grew));
    assert!(
        !note.is_empty(),
        "the quiet line carries no note in the gains' place: {:?}",
        full_line(&quiet)
    );

    assert_narrows_by_whole_clauses(
        "the quiet line",
        &quiet,
        vec![
            word_clause("dismiss hint", "esc"),
            word_clause("tale hint", "tale"),
            Clause {
                what: "quiet note",
                find: Box::new(move |l| {
                    let at: Vec<Option<usize>> = note.iter().map(|w| find_word(l, w)).collect();
                    if at.iter().all(Option::is_some) {
                        at.into_iter().flatten().min()
                    } else {
                        None
                    }
                }),
            },
        ],
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-027 / TUI-NAV-056 — what the pinned entry tells
//
// The entry is read off the whole Journal, drawn tall and wide enough that no
// line of it is scrolled or clipped away, with no report queued: what the
// entry says is the same wherever it sits.
// ---------------------------------------------------------------------------

fn whole_journal(state: &GameState) -> String {
    let rect = Rect::new(0, 0, 400, 160);
    let mut term = Terminal::new(TestBackend::new(400, 160)).expect("TestBackend is infallible");
    let view = journal_view(0);
    term.draw(|f| overlays::journal(f, rect, state, &view))
        .expect("TestBackend is infallible");
    region_lines(term.backend().buffer(), rect)
        .into_iter()
        .map(|l| l.trim_end().to_string())
        .collect::<Vec<_>>()
        .join("\n")
        .trim_end()
        .to_string()
}

fn entry_of(report: OfflineReport) -> String {
    whole_journal(&away(&[(9_000, report)]))
}

/// The clause around byte `at`: between the separators on either side of it.
fn clause_at(text: &str, at: usize) -> &str {
    let seps = [',', ';', '\u{b7}', '\n', '|'];
    let start = text[..at].rfind(seps).map(|i| i + 1).unwrap_or(0);
    let end = text[at..].find(seps).map(|i| at + i).unwrap_or(text.len());
    &text[start..end]
}

/// The line `at` stands on.
fn line_at(text: &str, at: usize) -> &str {
    let start = text[..at].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let end = text[at..].find('\n').map(|i| at + i).unwrap_or(text.len());
    &text[start..end]
}

fn count_words(text: &str, word: &str) -> usize {
    let mut n = 0;
    let mut rest = text;
    while let Some(at) = find_word(rest, word) {
        n += 1;
        rest = &rest[at + word.len()..];
    }
    n
}

/// Every resource whose stock changed is in the entry, signed — gains before
/// decreases, and each group largest first (a decrease's size is how far the
/// stock fell) — and no resource whose stock did not change is.
/// @spec TUI-NAV-027
#[test]
fn the_pinned_entry_lists_every_changed_stock_signed_gains_first_each_largest_first() {
    let mut report = OfflineReport {
        elapsed_seconds: 2 * 86_400,
        ..OfflineReport::default()
    };
    // Every resource moves, alternately up and down, each by a distinct amount
    // that runs against the resources' own order.
    for (i, r) in ResourceId::ALL.into_iter().enumerate() {
        let size = ((i * 37) % ResourceId::ALL.len() + 1) as f64 * 100.0;
        report.resource_deltas[r.ix()] = if i % 2 == 0 { size } else { -size };
    }
    let entry = entry_of(report.clone());

    let mut expected: Vec<(ResourceId, f64)> = ResourceId::ALL
        .into_iter()
        .map(|r| (r, report.resource_deltas[r.ix()]))
        .collect();
    expected.sort_by(|a, b| {
        let group = |d: f64| if d > 0.0 { 0 } else { 1 };
        group(a.1)
            .cmp(&group(b.1))
            .then(b.1.abs().partial_cmp(&a.1.abs()).unwrap())
    });

    let mut last = None;
    for (r, d) in &expected {
        let at = find_resource(&entry, *r).unwrap_or_else(|| {
            panic!("the pinned entry leaves out {r:?}, whose stock changed by {d}; it tells every resource whose stock changed. It read as:\n{entry}")
        });
        let clause = clause_at(&entry, at);
        if *d > 0.0 {
            assert!(
                clause.contains('+'),
                "the pinned entry gives {r:?}'s gain unsigned: {clause:?}; every change is signed"
            );
        } else {
            assert!(
                clause.contains('-') || clause.contains('\u{2212}'),
                "the pinned entry gives {r:?}'s fall without its sign: {clause:?}; every change \
                 is signed"
            );
        }
        if let Some((prev, prev_at)) = last {
            assert!(
                prev_at < at,
                "the pinned entry tells {r:?} ({d}) before {prev:?}; gains come before decreases, \
                 and each group runs largest first. It read as:\n{entry}"
            );
        }
        last = Some((*r, at));
    }

    // Only what changed.
    let mut two = OfflineReport {
        elapsed_seconds: 2 * 86_400,
        ..OfflineReport::default()
    };
    two.resource_deltas[ResourceId::Catnip.ix()] = 480.0;
    two.resource_deltas[ResourceId::Twigs.ix()] = -30.0;
    let entry = entry_of(two);
    for r in ResourceId::ALL
        .into_iter()
        .filter(|r| ![ResourceId::Catnip, ResourceId::Twigs].contains(r))
    {
        assert!(
            find_resource(&entry, r).is_none(),
            "the pinned entry names {r:?}, whose stock did not change. It read as:\n{entry}"
        );
    }
}

/// Each kitten who arrived is named, once, with its count where the report
/// counts it more than once.
/// @spec TUI-NAV-027
#[test]
fn the_pinned_entry_names_each_arrival_once_with_its_count_where_more_than_one() {
    let mut report = OfflineReport {
        elapsed_seconds: 3 * 86_400 + 5 * 3_600,
        ..OfflineReport::default()
    };
    report.arrived = vec![("Pebblekin".to_string(), 1), ("Quoriel".to_string(), 4)];
    let entry = entry_of(report);

    for (name, count) in [("Pebblekin", 1), ("Quoriel", 4)] {
        assert_eq!(
            count_words(&entry, name),
            1,
            "the pinned entry names {name} {} times; each kitten who arrived is named once. It \
             read as:\n{entry}",
            count_words(&entry, name)
        );
        let at = find_word(&entry, name).expect("named once");
        let clause = clause_at(&entry, at);
        let runs = digit_runs(clause);
        if count > 1 {
            assert!(
                runs.contains(&count.to_string()),
                "{name} arrived {count} times and the entry does not say so: {clause:?}"
            );
        } else {
            assert!(
                !runs.contains(&"1".to_string()),
                "{name} arrived once and the entry counts it anyway: {clause:?}; the count is \
                 given only where the report counts more than one"
            );
        }
    }
}

/// The seasons that passed are part of the tale.
/// @spec TUI-NAV-027
#[test]
fn the_pinned_entry_tells_the_seasons_passed() {
    let quiet = OfflineReport {
        elapsed_seconds: 3 * 86_400 + 5 * 3_600,
        ..OfflineReport::default()
    };
    let mut seasons = quiet.clone();
    seasons.seasons_passed = 7;
    let (without, with) = (entry_of(quiet), entry_of(seasons));
    assert_ne!(
        without, with,
        "seven seasons passing left the pinned entry unchanged"
    );
    assert!(
        digit_runs(&with).contains(&"7".to_string()),
        "the pinned entry does not say how many seasons passed. It read as:\n{with}"
    );
}

/// Where the core stopped counting, the entry says the absence was more than
/// its span and adds a gentle note that the meadow stopped counting there.
/// @spec TUI-NAV-027
#[test]
fn a_truncated_reports_entry_says_more_than_its_span_and_that_counting_stopped() {
    let exact = OfflineReport {
        elapsed_seconds: 3 * 86_400,
        ..OfflineReport::default()
    };
    let mut floor = exact.clone();
    floor.truncated = true;
    let (exact_entry, floor_entry) = (entry_of(exact), entry_of(floor));

    assert!(
        ["more than", "over", ">"]
            .iter()
            .any(|m| floor_entry.to_lowercase().contains(m)),
        "a truncated report's entry does not say the absence was more than its span. It read \
         as:\n{floor_entry}"
    );
    let marker: BTreeSet<String> = ["more", "than", "over"]
        .iter()
        .map(|w| w.to_string())
        .collect();
    let extra: Vec<String> = words(&floor_entry)
        .into_iter()
        .filter(|w| !words(&exact_entry).contains(w) && !marker.contains(w))
        .collect();
    assert!(
        !extra.is_empty(),
        "a truncated report's entry carries no note that the meadow stopped counting; it read \
         as:\n{floor_entry}"
    );
    assert!(
        loss_vocabulary(&floor_entry).is_empty(),
        "the stopped-counting note speaks of loss: {floor_entry}"
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-056 — wanderings and homecomings in the pinned entry
// ---------------------------------------------------------------------------

/// Words that say where a kitten is *now* — which the report cannot know, since
/// its lists count what happened, not where anyone ended up.
const WHEREABOUTS: [&str; 10] = [
    "is",
    "are",
    "still",
    "now",
    "currently",
    "remains",
    "remain",
    "soon",
    "until",
    "yet",
];

fn wandering_report(wandered: &[(&str, u32)], returned: &[(&str, u32)]) -> OfflineReport {
    OfflineReport {
        elapsed_seconds: 3 * 86_400 + 5 * 3_600,
        wandered: wandered.iter().map(|(n, c)| (n.to_string(), *c)).collect(),
        returned: returned.iter().map(|(n, c)| (n.to_string(), *c)).collect(),
        ..OfflineReport::default()
    }
}

/// Each kitten who wandered is named once, with its count where more than one,
/// as a visit to the neighbour's yard that ends back home.
/// @spec TUI-NAV-056
#[test]
fn each_wandering_is_named_once_as_a_visit_that_ends_back_home() {
    let entry = entry_of(wandering_report(&[("Moss", 1), ("Wren", 2)], &[]));
    for (name, count) in [("Moss", 1), ("Wren", 2)] {
        assert_eq!(
            count_words(&entry, name),
            1,
            "{name} is named other than once:\n{entry}"
        );
        let at = find_word(&entry, name).expect("named");
        let line = line_at(&entry, at);
        assert!(
            mentions(line, "neighb"),
            "{name}'s visit does not say where they went: {line:?}"
        );
        assert!(
            ["home", "back"]
                .iter()
                .any(|w| find_word(line, w).is_some()),
            "{name}'s visit does not say it ends back home: {line:?}"
        );
        let runs = digit_runs(clause_at(&entry, at));
        if count > 1 {
            assert!(
                runs.contains(&count.to_string()),
                "{name} wandered {count} times and the entry does not say so: {line:?}"
            );
        } else {
            assert!(
                !runs.contains(&"1".to_string()),
                "{name} wandered once and is counted anyway: {line:?}"
            );
        }
    }
}

/// Each kitten who came home is named once, with its count where more than
/// one, as a homecoming — and a kitten on both lists is named once on each.
/// @spec TUI-NAV-056
#[test]
fn each_homecoming_is_named_once_per_list_with_its_count() {
    let entry = entry_of(wandering_report(&[], &[("Moss", 1), ("Wren", 2)]));
    for (name, count) in [("Moss", 1), ("Wren", 2)] {
        assert_eq!(
            count_words(&entry, name),
            1,
            "{name} is named other than once:\n{entry}"
        );
        let at = find_word(&entry, name).expect("named");
        let line = line_at(&entry, at);
        assert!(
            find_word(line, "home").is_some(),
            "{name}'s homecoming does not read as one: {line:?}"
        );
        let runs = digit_runs(clause_at(&entry, at));
        if count > 1 {
            assert!(
                runs.contains(&count.to_string()),
                "{name} came home {count} times and the entry does not say so: {line:?}"
            );
        } else {
            assert!(
                !runs.contains(&"1".to_string()),
                "{name} came home once and is counted anyway: {line:?}"
            );
        }
    }

    let both = entry_of(wandering_report(&[("Moss", 2)], &[("Moss", 2)]));
    assert_eq!(
        count_words(&both, "Moss"),
        2,
        "a kitten who both wandered and came home is named {} times; once per list. It read \
         as:\n{both}",
        count_words(&both, "Moss")
    );
}

fn kitten_named(name: &str, location: clowder_core::Location) -> clowder_core::Kitten {
    clowder_core::Kitten {
        name: name.to_string(),
        job: None,
        location,
        arrived_day: 0,
        job_seq: None,
        away_seq: None,
        skills: [0.0; 8],
        exp: 0.0,
        rank: 0,
        trait_id: clowder_core::KittenTrait::None,
        tinkering: None,
    }
}

/// The entry states occurrences, never whereabouts: where the roster says a
/// kitten is now changes nothing in it, and nothing in its kitten lines says
/// where anyone is.
/// @spec TUI-NAV-056
#[test]
fn the_entry_states_occurrences_and_never_whereabouts() {
    use clowder_core::Location;
    let report = wandering_report(&[("Moss", 2), ("Wren", 1)], &[("Moss", 1), ("Hazel", 1)]);
    let entry_with = |moss: Location, extra: usize| {
        let mut s = away(&[(9_000, report.clone())]);
        s.kittens = vec![
            kitten_named("Moss", moss),
            kitten_named("Wren", Location::Home),
            kitten_named("Hazel", Location::Home),
        ];
        s.kittens
            .extend((0..extra).map(|i| kitten_named(&format!("Extra{i}"), Location::Home)));
        whole_journal(&s)
    };
    let home = entry_with(Location::Home, 0);
    assert_eq!(
        home,
        entry_with(Location::NeighborsYard, 0),
        "where the roster says a kitten is now changed the pinned entry; the entry states what \
         happened during its span, never who is away now"
    );
    assert_eq!(
        home,
        entry_with(Location::Home, 9),
        "the size of the clowder changed the pinned entry; it never states a kitten total"
    );

    for name in ["Moss", "Wren", "Hazel"] {
        let mut rest = home.as_str();
        while let Some(at) = find_word(rest, name) {
            let line = line_at(rest, at);
            let said: Vec<&str> = WHEREABOUTS
                .iter()
                .copied()
                .filter(|w| find_word(line, w).is_some())
                .collect();
            assert!(
                said.is_empty(),
                "the entry's line for {name} says where they are ({said:?}): {line:?}; it states \
                 occurrences, never whereabouts"
            );
            rest = &rest[at + name.len()..];
        }
    }
}

/// No departure reads as a loss, as a count going down, or as a kitten total
/// net of departures.
/// @spec TUI-NAV-056
#[test]
fn no_departure_reads_as_a_loss_a_decrease_or_a_net_total() {
    let mut report = wandering_report(&[("Moss", 1), ("Wren", 3)], &[("Wren", 2)]);
    // Five arrived, four wandered: a total net of departures would read one.
    report.arrived = vec![("Pip".to_string(), 2), ("Clover".to_string(), 3)];
    let entry = entry_of(report);

    assert!(
        loss_vocabulary(&entry).is_empty(),
        "the entry speaks of {:?}:\n{entry}",
        loss_vocabulary(&entry)
    );
    assert!(
        !shows_a_decrease(&entry),
        "the entry shows a count going down:\n{entry}"
    );
    assert!(
        !digit_runs(&entry).contains(&"1".to_string()),
        "the entry shows a one where five arrived and four wandered — a total net of \
         departures:\n{entry}"
    );
}

// ---------------------------------------------------------------------------
// Coats in sentences — TUI-THEME-019, TUI-THEME-020, TUI-THEME-023,
// TUI-THEME-024
// ---------------------------------------------------------------------------

/// A resource's name as the stores spell it: the identifier spaced at each
/// capital.
fn label(r: ResourceId) -> String {
    let tag = format!("{r:?}");
    let mut out = String::new();
    for (i, c) in tag.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

/// Every cell of a rect, row by row, with the character it shows.
fn chars_and_cells(buf: &Buffer, rect: Rect) -> Vec<Vec<(char, &Cell)>> {
    (rect.y..rect.bottom())
        .map(|y| {
            (rect.x..rect.right())
                .map(|x| {
                    let cell = buf.cell((x, y)).expect("every cell of a drawn rect exists");
                    (cell.symbol().chars().next().unwrap_or(' '), cell)
                })
                .collect()
        })
        .collect()
}

/// Where each resource's stores-spelled name stands as whole words on a row:
/// (column, resource, name length).
fn named_on(row: &[char]) -> Vec<(usize, ResourceId, usize)> {
    let mut out = Vec::new();
    for r in ResourceId::ALL {
        let name: Vec<char> = label(r).chars().collect();
        if name.len() > row.len() {
            continue;
        }
        for at in 0..=row.len() - name.len() {
            let end = at + name.len();
            let left = at == 0 || !row[at - 1].is_alphanumeric();
            let right = end == row.len() || !row[end].is_alphanumeric();
            if left && right && row[at..end] == name[..] {
                out.push((at, r, name.len()));
            }
        }
    }
    out
}

/// Check one surface: every resource its rows name wears its coat across the
/// name, at full strength — no dim attribute — and every other cell passes
/// `rest`. Returns how many names were checked.
fn coats_on_names(what: &str, buf: &Buffer, rect: Rect, rest: impl Fn(&Cell) -> bool) -> usize {
    use clowder_tui::theme::{coat, coat_colours};
    let mut checked = 0;
    for (y, row) in chars_and_cells(buf, rect).iter().enumerate() {
        let text: Vec<char> = row.iter().map(|(c, _)| *c).collect();
        let line: String = text.iter().collect();
        let mut in_name = vec![false; row.len()];
        for (at, r, len) in named_on(&text) {
            checked += 1;
            let colours = coat_colours(coat(r), len);
            for (k, want) in colours.iter().enumerate() {
                let cell = row[at + k].1;
                assert_eq!(
                    &cell.fg, want,
                    "{what}, row {y}: {r:?}'s name, character {k}, wears {:?}, not its coat's \
                     {want:?}: {line:?}",
                    cell.fg
                );
                assert!(
                    !cell.modifier.contains(Modifier::DIM),
                    "{what}, row {y}: {r:?}'s name is drawn faint; a coat is worn at full \
                     strength: {line:?}"
                );
                in_name[at + k] = true;
            }
        }
        for (x, (c, cell)) in row.iter().enumerate() {
            if !in_name[x] && !c.is_whitespace() {
                assert!(
                    rest(cell),
                    "{what}, row {y}: {c:?} at {x} is no resource's name, yet wears {:?} \
                     {:?}: {line:?}",
                    cell.fg,
                    cell.modifier
                );
            }
        }
    }
    checked
}

/// The ticker's line for a time away names its gains, each in its coat at full
/// strength; the rest of the line keeps the ticker's quiet.
///
/// @spec TUI-THEME-019, TUI-THEME-020, TUI-THEME-023, TUI-THEME-024
#[test]
fn the_tickers_away_line_names_each_gain_in_its_coat() {
    let state = away(&[(10_000, full_report())]);
    let buf = ticker(&state);
    let checked = coats_on_names("the ticker", &buf, ticker_rect(), |cell| {
        is_quiet(cell) || cell.symbol() == "\u{2605}"
    });
    assert!(
        checked >= 3,
        "fixture: the ticker names the report's gains ({checked})"
    );
}

/// The Journal's away report names each changed store in its coat, and every
/// other character of the Journal stays in the inherited foreground.
///
/// @spec TUI-THEME-019, TUI-THEME-020, TUI-THEME-023, TUI-THEME-024
#[test]
fn the_journals_away_report_names_each_changed_store_in_its_coat() {
    let state = away(&[(10_000, full_report())]);
    let buf = journal(&state, 0);
    let checked = coats_on_names("the Journal", &buf, overlay_rect(), |cell| {
        cell.fg == ratatui::style::Color::Reset
    });
    assert!(
        checked >= 3,
        "fixture: the Journal names the changed stores ({checked})"
    );
}

/// The report banner is a reversed line: it keeps its line whole, every
/// resource it names drawn in the terminal's own pair inverted and never in a
/// coat, which a reversal would turn into a block of ground.
///
/// @spec TUI-THEME-020
#[test]
fn the_reversed_report_banner_keeps_its_names_in_its_own_colour() {
    let state = away(&[(10_000, full_report())]);
    let view = unread(&state);
    let buf = banner(&state, &view);
    let text = text_of(&buf, banner_rect());
    assert!(
        ["Twigs", "Feathers", "Catnip"]
            .iter()
            .any(|n| mentions(&text, n)),
        "fixture: the banner names a gain: {text:?}"
    );
    for cell in cells_of(&buf, banner_rect()) {
        assert_eq!(
            cell.fg,
            ratatui::style::Color::Reset,
            "the banner wears {:?} on {:?}: {text:?}",
            cell.fg,
            cell.symbol()
        );
    }
}

/// A store's result in the find palette names it in its coat; every other
/// result, and the query line, stays in the inherited foreground — another
/// item's name that holds a resource's word names that item, not the store.
///
/// @spec TUI-THEME-019
#[test]
fn a_stores_result_in_the_palette_wears_its_coat() {
    use clowder_tui::theme::{coat, coat_colours};
    let stores = [
        ResourceId::Catnip,
        ResourceId::Twigs,
        ResourceId::JarsOfPurrs,
    ];
    let mut state = GameState::new(11);
    for r in stores {
        state.resources[r.ix()] = 1.0;
    }
    let mut checked = 0;
    for r in stores {
        let buf = palette(&state, &label(r).to_lowercase());
        for (y, row) in chars_and_cells(&buf, overlay_rect()).iter().enumerate() {
            let line: String = row.iter().map(|(c, _)| *c).collect();
            let name_at = line.find("] ").map(|i| line[..i + 2].chars().count());
            let store = name_at.and_then(|at| {
                let rest: String = line.chars().skip(at).collect();
                ResourceId::ALL
                    .into_iter()
                    .find(|s| rest.trim_end() == label(*s))
                    .map(|s| (at, s))
            });
            for (x, (c, cell)) in row.iter().enumerate() {
                let want = match store {
                    Some((at, s)) if x >= at && x < at + label(s).chars().count() => {
                        coat_colours(coat(s), label(s).chars().count())[x - at]
                    }
                    _ => ratatui::style::Color::Reset,
                };
                if c.is_whitespace() && want == ratatui::style::Color::Reset {
                    continue;
                }
                assert_eq!(
                    cell.fg, want,
                    "palette for {r:?}, row {y}: {c:?} at {x} wears {:?}, not {want:?}: {line:?}",
                    cell.fg
                );
            }
            if store.is_some() {
                checked += 1;
            }
        }
    }
    assert!(
        checked >= stores.len(),
        "fixture: each store has a result ({checked})"
    );
}
