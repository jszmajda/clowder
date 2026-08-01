//! The persistent furniture — `render::widgets::chrome`: the header and its
//! attention chips, the tab bar, the resource sidebar, and the hint bar.
//!
//! @spec TUI-NAV-019, TUI-NAV-020, TUI-NAV-021, TUI-NAV-022, TUI-NAV-023,
//! TUI-NAV-024, TUI-NAV-025, TUI-NAV-034, TUI-NAV-044, TUI-NAV-045, TUI-NAV-054
//!
//! **The contract these tests are made of.** The chrome's job is to be quiet.
//! An empty attention cluster means nothing needs you, and that only works if
//! the eye has learned that something appearing there is information — so the
//! load-bearing assertions here are as much about what the chrome does *not*
//! draw as about what it does. A chip that showed when its opportunity was not
//! live, or a chip that asked for an action the player cannot take, would cost
//! the cluster the meaning that makes the two-second glance work at all.
//!
//! **Why everything is read as plain text.** Every assertion below reads the
//! `TestBackend` buffer's symbols and ignores its styles. That is not a
//! convenience: it *is* TUI-NAV-022. If a meaning survives being read this way
//! then colour was the second channel, and a player who cannot see the colour
//! has lost nothing. Two states that must be told apart are therefore required
//! to differ in their symbols, never merely in their styles.
//!
//! **Differential assertions.** The header's exact format is its own business —
//! nothing in this leaf fixes the order of its facts or the punctuation between
//! them. So "the header carries the kitten count" is asserted by changing only
//! the roster and requiring the drawn line to change. That tests the claim the
//! spec actually makes (the fact reaches the screen) without inventing a layout
//! the design never chose. Where the design *does* pin a form — mood as a word,
//! a chip as glyph + count + word, a capped store's text `full` tag, the help
//! key last on the hint bar — the form is asserted directly.
//!
//! **Fixtures assert themselves.** Several states here are built to sit in a
//! particular mood tier or to reveal a particular set of sections. Each such
//! fixture checks that it really did, with a message beginning `fixture:`, so a
//! balance change in the core fails as a broken fixture rather than silently
//! turning a test into a weaker one.

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use clowder_core::{
    content, ColonyId, Command, ContentId, GameState, IdeaId, JobId, Kitten, KittenTrait, Location,
    MoodTier, ResourceId, Season, StructureId,
};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::Terminal;

use clowder_tui::nav::rows::{rows, Row, RowKey, RowKind};
use clowder_tui::nav::{
    ArmedRow, CountPrompt, CountSemantics, PendingReport, Section, SeenMark, ViewState,
};
use clowder_tui::render::layout::{compute, Body};
use clowder_tui::render::widgets::{chrome, overlays};

// ---------------------------------------------------------------------------
// Sizes
// ---------------------------------------------------------------------------

/// The supported floor (TUI-RENDER-004). Every claim about the chrome has to
/// hold here, because this is the size the design promises works.
const FLOOR_W: u16 = 80;
const FLOOR_H: u16 = 24;

/// Comfortably past both layout breakpoints: wide enough that a tab bar with
/// every section named fits, which is what makes the *narrow* case a test of
/// compaction rather than of arithmetic.
const WIDE_W: u16 = 200;
const WIDE_H: u16 = 60;

// ---------------------------------------------------------------------------
// Kittens
// ---------------------------------------------------------------------------

fn kitten(job: Option<JobId>, location: Location) -> Kitten {
    Kitten {
        name: String::new(),
        job,
        location,
        arrived_day: 0,
        job_seq: None,
        away_seq: None,
        skills: [0.0; 8],
        exp: 0.0,
        rank: 0,
        trait_id: KittenTrait::None,
        tinkering: None,
    }
}

/// Kittens with work, at home: nobody the idle chip could be about.
fn employed(n: usize) -> Vec<Kitten> {
    (0..n).map(|_| kitten(Some(JobId::Gardener), Location::Home)).collect()
}

/// Jobless kittens in the meadow: exactly what `◦N idle` counts.
fn idle_here(n: usize) -> Vec<Kitten> {
    (0..n).map(|_| kitten(None, Location::Home)).collect()
}

/// Jobless kittens visiting the neighbour's yard. Just as jobless, and not
/// assignable by anyone — TUI-NAV-054's whole subject.
fn idle_away(n: usize) -> Vec<Kitten> {
    (0..n).map(|_| kitten(None, Location::NeighborsYard)).collect()
}

// ---------------------------------------------------------------------------
// States
// ---------------------------------------------------------------------------

/// A fixed point in the calendar, so a test about something else never has the
/// date moving under it. Day 362 of a 100-day year: year 3, and the third
/// season of the ring.
const SETTLED_TICKS: u64 = 362 * content::TICKS_PER_DAY;

/// A meadow far enough along that every one of the ten sections has at least
/// one revealed row — the state the tab bar's overflow behaviour is about.
fn every_section_revealed() -> GameState {
    let mut s = GameState::new(7);
    s.ticks = SETTLED_TICKS;
    for r in ResourceId::ALL {
        s.resources[r.ix()] = 1.0e9;
    }
    for b in StructureId::ALL {
        s.structures[b.ix()] = 2;
    }
    s.ideas = IdeaId::ALL.iter().copied().collect::<BTreeSet<_>>();
    s.revealed = StructureId::ALL
        .iter()
        .map(|b| ContentId::Structure(*b))
        .chain(IdeaId::ALL.iter().map(|i| ContentId::Idea(*i)))
        .collect::<BTreeSet<_>>();
    s.peak_devotion = 1.0e9;
    s.discovered = ColonyId::ALL.iter().copied().collect::<BTreeSet<_>>();
    s.generation = 1;
    s.kittens = employed(6);
    // Prestige reveals on having met the size it asks for, so the ratchet is
    // what this fixture needs — not a roster it never had.
    s.roster_high_water = 100;

    for section in Section::ALL {
        assert!(
            !rows(&s, section).is_empty(),
            "fixture: the {} section should have rows in a meadow this far along",
            section.title()
        );
    }
    s
}

/// Watermarks that say the player has just looked at every section: the
/// baseline against which "content revealed since you last looked" means
/// anything at all.
fn all_caught_up(state: &GameState) -> [SeenMark; 10] {
    let mut marks = [SeenMark::default(); 10];
    for section in Section::ALL {
        marks[section.index()] =
            SeenMark { visited: true, revealed_at_last_visit: rows(state, section).len() };
    }
    marks
}

/// A view over a state with nothing outstanding: every section seen, no report
/// unread, no prompt, no overlay, no armed row.
fn calm_view(state: &GameState) -> ViewState {
    let mut v = ViewState::default();
    v.section = Some(Section::Meadow);
    v.seen = all_caught_up(state);
    v
}

/// A view focused on one row of one section, with nothing else outstanding.
fn focus_on(state: &GameState, section: Section, key: RowKey) -> ViewState {
    let mut v = calm_view(state);
    v.section = Some(section);
    v.focus[section.index()] = Some(key);
    v
}

/// The first row of a section matching a predicate, by identity.
fn first_row_where(
    state: &GameState,
    section: Section,
    what: &str,
    pred: impl Fn(&Row) -> bool,
) -> RowKey {
    rows(state, section)
        .into_iter()
        .find(|r| pred(r))
        .unwrap_or_else(|| {
            panic!("fixture: the {} section should have {what}", section.title())
        })
        .key
}

fn single_row(state: &GameState) -> RowKey {
    first_row_where(state, Section::Meadow, "a plain one-shot row", |r| {
        !r.heavy && matches!(r.kind, RowKind::Act { semantics: CountSemantics::Single, .. })
    })
}

fn repeatable_row(state: &GameState) -> RowKey {
    first_row_where(state, Section::Meadow, "a repeat-N row", |r| {
        matches!(r.kind, RowKind::Act { semantics: CountSemantics::RepeatN, .. })
    })
}

fn count_taking_row(state: &GameState) -> RowKey {
    first_row_where(state, Section::Clowder, "a count-taking row", |r| {
        matches!(r.kind, RowKind::Act { semantics: CountSemantics::CountTaking, .. })
    })
}

fn count_row(state: &GameState) -> RowKey {
    first_row_where(state, Section::Clowder, "a +/- count row", |r| {
        matches!(r.kind, RowKind::Count { .. })
    })
}

fn heavy_row(state: &GameState) -> RowKey {
    first_row_where(state, Section::Legacy, "a heavy row", |r| r.heavy)
}

/// A meadow in a chosen mood, with everything else — the date, the roster size,
/// the speed — held still. Warm Memories is the one happiness lever that moves
/// nothing else the header draws.
fn moody(tier: MoodTier) -> GameState {
    let mut s = GameState::new(11);
    s.ticks = SETTLED_TICKS;
    s.kittens = employed(21);
    s.warm_memories = match tier {
        MoodTier::Fussy | MoodTier::Grumpy => 0.0,
        MoodTier::Restless => 300.0,
        MoodTier::Content => 4_000.0,
        MoodTier::Blissful => 13_000.0,
    };
    s.grumpy = tier == MoodTier::Grumpy;
    assert_eq!(
        s.mood_tier(),
        tier,
        "fixture: this meadow was built to be {tier:?} and the core calls it {:?}",
        s.mood_tier()
    );
    s
}

/// Every mood the core can report. Exhaustive on purpose: a sixth tier stops
/// this file compiling, which is what keeps "mood as a word" a claim about all
/// of them.
const MOODS: [MoodTier; 5] = [
    MoodTier::Blissful,
    MoodTier::Content,
    MoodTier::Restless,
    MoodTier::Fussy,
    MoodTier::Grumpy,
];

fn mood_name(tier: MoodTier) -> &'static str {
    match tier {
        MoodTier::Blissful => "blissful",
        MoodTier::Content => "content",
        MoodTier::Restless => "restless",
        MoodTier::Fussy => "fussy",
        MoodTier::Grumpy => "grumpy",
    }
}

// ---------------------------------------------------------------------------
// Drawing
// ---------------------------------------------------------------------------

fn terminal(w: u16, h: u16) -> Terminal<TestBackend> {
    Terminal::new(TestBackend::new(w, h)).expect("TestBackend is infallible")
}

/// The four regions the chrome owns at a size, taken from the real layout so a
/// test never invents a rectangle the frame would not have used.
fn regions(w: u16, h: u16) -> (Rect, Rect, Rect, Rect) {
    let layout = compute(Rect::new(0, 0, w, h), None, false);
    let sidebar = match layout.body {
        Body::Narrow { sidebar, .. } | Body::Wide { sidebar, .. } => sidebar,
        Body::TooSmall(_) => panic!("fixture: {w}x{h} is below the supported floor"),
    };
    (layout.header, layout.tabs, sidebar, layout.hints)
}

/// The text of a rectangle, one string per row, padded to the full width.
fn region_lines(buf: &Buffer, rect: Rect) -> Vec<String> {
    (rect.y..rect.bottom())
        .map(|y| {
            (rect.x..rect.right())
                .map(|x| buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "))
                .collect::<String>()
        })
        .collect()
}

/// Draw one widget into a terminal of exactly its own size and read the line
/// back. The chrome's rows are one line each, so a one-row terminal is the
/// whole of the region under test.
fn one_line(w: u16, draw: impl FnOnce(&mut ratatui::Frame, Rect)) -> String {
    let mut term = terminal(w, 1);
    term.draw(|f| {
        let area = f.area();
        draw(f, area);
    })
    .expect("TestBackend is infallible");
    region_lines(term.backend().buffer(), Rect::new(0, 0, w, 1))
        .pop()
        .expect("a one-row region has one line")
}

fn header_line(state: &GameState, view: &ViewState, w: u16) -> String {
    one_line(w, |f, area| chrome::header(f, area, state, view))
}

fn tab_line(state: &GameState, view: &ViewState, w: u16) -> String {
    one_line(w, |f, area| chrome::tabs(f, area, state, view))
}

fn hint_line(state: &GameState, view: &ViewState, w: u16) -> String {
    one_line(w, |f, area| chrome::hints(f, area, state, view))
}

fn sidebar_lines(state: &GameState, w: u16, h: u16) -> Vec<String> {
    let (_, _, side, _) = regions(w, h);
    let mut term = terminal(w, h);
    term.draw(|f| chrome::sidebar(f, side, state)).expect("TestBackend is infallible");
    region_lines(term.backend().buffer(), side)
}

fn blank(lines: &[String]) -> bool {
    lines.iter().all(|l| l.trim().is_empty())
}

// ---------------------------------------------------------------------------
// Reading a line
// ---------------------------------------------------------------------------

/// Characters an implementation might reasonably put *between* hints. Splitting
/// on them as well as on whitespace is what lets a key be matched as a whole
/// token — `Tab next` must not be read as advertising `x`.
const SEPARATORS: [char; 5] = ['·', '|', ',', ';', '•'];

fn tokens(line: &str) -> Vec<String> {
    line.split(|c: char| c.is_whitespace() || SEPARATORS.contains(&c))
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .collect()
}

fn has_key(line: &str, key: &str) -> bool {
    tokens(line).iter().any(|t| t == key)
}

fn says(line: &str, word: &str) -> bool {
    line.to_ascii_lowercase().contains(&word.to_ascii_lowercase())
}

// ---------------------------------------------------------------------------
// The attention chips
// ---------------------------------------------------------------------------

/// The four chips, each as the design states it: a glyph, then a count where it
/// has one, then a word. Triple-channel, so no chip depends on colour.
const IDLE: (char, &str) = ('◦', "idle");
const SKY: (char, &str) = ('!', "sky");
const NEW: (char, &str) = ('★', "new");
const AWAY: (char, &str) = ('~', "away");
const CHIPS: [(char, &str); 4] = [IDLE, SKY, NEW, AWAY];

fn chip_name(chip: (char, &str)) -> String {
    format!("`{}{}`", chip.0, chip.1)
}

/// A chip is shown when both of its channels are on the line. Either alone
/// would be half a chip, and half a chip is what colour-only signalling looks
/// like from the wrong side of TUI-NAV-020.
fn chip_shown(line: &str, chip: (char, &str)) -> bool {
    line.contains(chip.0) && says(line, chip.1)
}

/// The count a chip carries, read from between its glyph and its word.
fn chip_count(line: &str, chip: (char, &str)) -> Option<u64> {
    let glyph_at = line.find(chip.0)?;
    let lowered = line.to_ascii_lowercase();
    let word_at = lowered.get(glyph_at..)?.find(chip.1)? + glyph_at;
    let digits: String = line
        .get(glyph_at + chip.0.len_utf8()..word_at)?
        .chars()
        .filter(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

fn live_chips(line: &str) -> Vec<&'static str> {
    CHIPS.iter().filter(|c| chip_shown(line, **c)).map(|c| c.1).collect()
}

/// A meadow and a view in which all four opportunities are live at once — the
/// state every "clears on its own signal" claim is measured against.
fn all_four_live() -> (GameState, ViewState) {
    let mut state = every_section_revealed();
    let mut roster = idle_here(4);
    roster.extend(idle_away(2));
    roster.extend(employed(3));
    state.kittens = roster;
    state.pending_sky_event = Some(state.ticks + 5);

    let mut view = calm_view(&state);
    // Three rows have appeared in the Clowder section since it was last looked
    // at, which is what `★N new` counts.
    let clowder = Section::Clowder.index();
    assert!(
        view.seen[clowder].revealed_at_last_visit >= 3,
        "fixture: the Clowder section needs rows for the watermark to sit behind"
    );
    view.seen[clowder].revealed_at_last_visit -= 3;
    view.reports = vec![PendingReport { entry: 0, dismissed: false }];

    (state, view)
}

// ---------------------------------------------------------------------------
// TUI-NAV-019 — the chrome is always there
// ---------------------------------------------------------------------------

/// Every piece of the persistent furniture draws at every supported size. The
/// glance contract is worth nothing if the header is the thing that degradation
/// takes away: degradation removes simultaneity, never capability.
///
/// @spec TUI-NAV-019
#[test]
fn every_piece_of_the_chrome_is_drawn_at_every_supported_size() {
    let state = every_section_revealed();
    let view = calm_view(&state);

    for (w, h) in [(FLOOR_W, FLOOR_H), (100, 30), (120, 40), (WIDE_W, WIDE_H)] {
        let (header, tabs, sidebar, hints) = regions(w, h);
        let mut term = terminal(w, h);
        term.draw(|f| {
            chrome::header(f, header, &state, &view);
            chrome::tabs(f, tabs, &state, &view);
            chrome::sidebar(f, sidebar, &state);
            chrome::hints(f, hints, &state, &view);
        })
        .expect("TestBackend is infallible");
        let buffer = term.backend().buffer().clone();

        for (what, rect) in
            [("header", header), ("tab bar", tabs), ("sidebar", sidebar), ("hint bar", hints)]
        {
            let lines = region_lines(&buffer, rect);
            assert!(
                !blank(&lines),
                "{w}x{h}: the {what} drew nothing. The chrome is persistent at every supported \
                 size — it is the thing a player glances at, and a blank one is a size where the \
                 glance contract has quietly lapsed."
            );
        }
    }
}

/// The header carries each fact the glance contract names, tested by moving one
/// fact at a time and requiring the drawn line to move with it. The format is
/// the header's own business; that the fact reaches the screen is not.
///
/// The clowder's *name* is the one fact TUI-NAV-019 lists that this test cannot
/// vary, because `clowder_core::GameState` exposes no such field — reported as
/// an open question rather than invented here, since TUI-NAV-053 forbids the
/// shell inventing state the core did not give it.
///
/// @spec TUI-NAV-019
#[test]
fn the_header_carries_every_fact_the_glance_contract_names() {
    let base = {
        let mut s = GameState::new(11);
        s.ticks = SETTLED_TICKS;
        s.kittens = employed(3);
        s
    };
    let reference = header_line(&base, &calm_view(&base), WIDE_W);

    let moved: [(&str, GameState); 5] = [
        ("the generation", {
            let mut s = base.clone();
            s.generation += 1;
            s
        }),
        ("the date", {
            let mut s = base.clone();
            s.ticks += content::TICKS_PER_DAY * content::DAYS_PER_SEASON;
            s
        }),
        ("the kitten count", {
            let mut s = base.clone();
            s.kittens = employed(4);
            s
        }),
        ("the mood", {
            let mut s = base.clone();
            s.grumpy = true;
            s
        }),
        ("the speed", {
            let mut s = base.clone();
            s.speed = 4.0;
            s
        }),
    ];

    for (fact, state) in moved {
        let drawn = header_line(&state, &calm_view(&state), WIDE_W);
        assert_ne!(
            drawn, reference,
            "{fact} changed and the header did not. TUI-NAV-019 names it as one of the facts the \
             header keeps on screen, and a header that cannot tell the two states apart is not \
             carrying it.\n  before: {reference:?}\n   after: {drawn:?}"
        );
    }

    // The season is named, not merely tinted: the accent is the season's
    // *second* channel, and a player without colour still has to know where in
    // the year the clowder is.
    for season in Season::ALL {
        let mut s = base.clone();
        s.ticks = 0;
        while s.season() != season {
            s.ticks += content::TICKS_PER_DAY * content::DAYS_PER_SEASON;
        }
        let drawn = header_line(&s, &calm_view(&s), WIDE_W);
        assert!(
            says(&drawn, &format!("{season:?}")),
            "the header does not name the season in {season:?}; it read {drawn:?}. Colour is \
             always the second channel, so the season's own word has to be the first."
        );
    }
}

/// The sidebar is a glance at the stores, drawn from the snapshot. Two meadows
/// holding different amounts of the same thing cannot draw the same sidebar —
/// anything else means the sidebar is showing something other than the core's
/// numbers.
///
/// @spec TUI-NAV-019
#[test]
fn the_sidebar_shows_the_stores_it_is_handed() {
    let with_a_little = {
        let mut s = GameState::new(11);
        s.resources[ResourceId::Catnip.ix()] = 40.0;
        s
    };
    let with_a_lot = {
        let mut s = GameState::new(11);
        s.resources[ResourceId::Catnip.ix()] = 150.0;
        s
    };

    let little = sidebar_lines(&with_a_little, FLOOR_W, FLOOR_H);
    let lot = sidebar_lines(&with_a_lot, FLOOR_W, FLOOR_H);

    assert!(!blank(&little), "a meadow with stores must have a sidebar that says so");
    assert_ne!(
        little,
        lot,
        "two meadows holding different amounts drew the same sidebar; the sidebar is a glance at \
         the *stock*, and one that cannot tell 40 from 150 is not showing it.\n{}",
        lot.join("\n")
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-020 — the chips, and the emptiness between them
// ---------------------------------------------------------------------------

/// The whole design, stated as an absence: when nothing is live the cluster is
/// empty. This is the assertion the other three chip tests are worth something
/// because of — a cluster that is never quiet teaches the player to stop
/// looking at it, and then none of the chips mean anything.
///
/// @spec TUI-NAV-020
#[test]
fn an_empty_chip_cluster_is_what_nothing_needing_you_looks_like() {
    let settled = {
        let mut s = every_section_revealed();
        s.kittens = employed(6);
    // Prestige reveals on having met the size it asks for, so the ratchet is
    // what this fixture needs — not a roster it never had.
    s.roster_high_water = 100;
        s.pending_sky_event = None;
        s
    };

    let cases: [(&str, GameState); 2] = [
        ("a settled meadow with nothing outstanding", settled),
        ("a brand-new meadow", GameState::new(3)),
    ];

    for (what, state) in cases {
        let view = calm_view(&state);
        let line = header_line(&state, &view, WIDE_W);
        for chip in CHIPS {
            assert!(
                !line.contains(chip.0),
                "in {what} the header drew the {} glyph with nothing live behind it. The chip \
                 cluster's emptiness is the design: a chip that is always there is not a signal.\n\
                 header: {line:?}",
                chip_name(chip)
            );
            assert!(
                !says(&line, chip.1),
                "in {what} the header said {:?} with nothing live behind it.\nheader: {line:?}",
                chip.1
            );
        }
    }
}

/// A chip is a glyph, a count and a word — three channels, so that a player who
/// cannot see the colour, or cannot see the glyph, still reads it.
///
/// @spec TUI-NAV-020, TUI-NAV-022
#[test]
fn the_idle_chip_is_a_glyph_a_count_and_a_word() {
    let mut state = every_section_revealed();
    state.kittens = idle_here(4);
    state.kittens.extend(employed(2));
    let view = calm_view(&state);

    let line = header_line(&state, &view, WIDE_W);
    assert!(
        line.contains(IDLE.0),
        "four kittens are in the meadow with nothing to do and the header drew no {} glyph: \
         {line:?}",
        chip_name(IDLE)
    );
    assert!(
        says(&line, IDLE.1),
        "the idle chip must carry its word as well as its glyph — the word is the channel that \
         survives a terminal with no glyph coverage at all: {line:?}"
    );
    assert_eq!(
        chip_count(&line, IDLE),
        Some(4),
        "the idle chip must say *how many*: free production you are leaving on the table is a \
         quantity, and `◦ idle` alone does not tell the player whether to walk over. Header read \
         as {line:?}"
    );
}

/// `!sky` shows while the window is open and not otherwise — and a window that
/// has faded is not open. The core is the arbiter: `ObserveSky` on a faded
/// window is `Locked`, so a chip drawn for one would be pointing at a key press
/// the core will refuse.
///
/// @spec TUI-NAV-020
#[test]
fn the_sky_chip_shows_only_while_the_window_is_open() {
    let mut open = every_section_revealed();
    open.pending_sky_event = Some(open.ticks + 5);

    let mut faded = every_section_revealed();
    faded.pending_sky_event = Some(faded.ticks);

    let mut none = every_section_revealed();
    none.pending_sky_event = None;

    let open_line = header_line(&open, &calm_view(&open), WIDE_W);
    assert!(
        chip_shown(&open_line, SKY),
        "a sky-event window is open and the header showed no {} chip; the window expires \
         harmlessly, so the chip is the only thing that tells the player it is there at all.\n\
         header: {open_line:?}",
        chip_name(SKY)
    );

    for (what, state) in [("a faded window", faded), ("no window at all", none)] {
        let line = header_line(&state, &calm_view(&state), WIDE_W);
        assert!(
            !chip_shown(&line, SKY),
            "with {what} the header still showed the sky chip. A chip shows only while its \
             opportunity is *live*, and observing a faded window is a command the core rejects.\n\
             header: {line:?}"
        );
    }
}

/// `★N new` counts what has appeared since the player last looked, and says
/// nothing when they are caught up.
///
/// @spec TUI-NAV-020
#[test]
fn the_new_content_chip_counts_what_arrived_since_you_looked() {
    let state = every_section_revealed();

    let caught_up = calm_view(&state);
    let line = header_line(&state, &caught_up, WIDE_W);
    assert!(
        !chip_shown(&line, NEW),
        "the player has looked at every section and the header still claims there is new \
         content: {line:?}"
    );

    let mut behind = calm_view(&state);
    behind.seen[Section::Ideas.index()].revealed_at_last_visit -= 2;
    let line = header_line(&state, &behind, WIDE_W);
    assert!(
        chip_shown(&line, NEW),
        "two rows have appeared since the Ideas section was last visited and the header said \
         nothing. Newly revealed content is the one opportunity a player has no other way to \
         notice.\nheader: {line:?}"
    );
    assert_eq!(
        chip_count(&line, NEW),
        Some(2),
        "the new-content chip must count what arrived; header read as {line:?}"
    );
}

/// `~away` shows while a report is unread and clears when it has been read. The
/// report is pinned in the Journal either way, so the chip is an invitation and
/// never a warning that something is about to be lost.
///
/// @spec TUI-NAV-020
#[test]
fn the_away_chip_shows_while_a_report_is_unread() {
    let state = every_section_revealed();

    let mut unread = calm_view(&state);
    unread.reports = vec![PendingReport { entry: 0, dismissed: false }];
    let line = header_line(&state, &unread, WIDE_W);
    assert!(
        chip_shown(&line, AWAY),
        "a while-you-were-away report is waiting and the header showed no {} chip: {line:?}",
        chip_name(AWAY)
    );

    let mut read = calm_view(&state);
    read.reports = vec![PendingReport { entry: 0, dismissed: true }];
    let line = header_line(&state, &read, WIDE_W);
    assert!(
        !chip_shown(&line, AWAY),
        "the report has been read and the header still shows the away chip: {line:?}"
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-054 — the idle chip asks only for what the player can do
// ---------------------------------------------------------------------------

/// A kitten visiting the neighbour's yard is just as jobless and not assignable
/// by anybody. Counting one would make the chip a standing request for an
/// action the player cannot take, which is worse than no chip.
///
/// @spec TUI-NAV-054
#[test]
fn the_idle_chip_counts_only_kittens_who_are_here_to_be_assigned() {
    let mut state = every_section_revealed();
    let mut roster = idle_here(3);
    roster.extend(idle_away(5));
    roster.extend(employed(2));
    state.kittens = roster;
    let view = calm_view(&state);

    let line = header_line(&state, &view, WIDE_W);
    assert_eq!(
        chip_count(&line, IDLE),
        Some(3),
        "three kittens are in the meadow without work and five more are jobless at the \
         neighbour's. The chip must count the three: a chip counting all eight would ask the \
         player to assign kittens who are not here, and the count is the only part of the chip \
         that tells them how many they can actually reach.\nheader: {line:?}"
    );
}

/// The limiting case, and the one that proves the rule is a rule about
/// liveness: every jobless kitten is away, so there is no opportunity at all
/// and the chip must not appear.
///
/// @spec TUI-NAV-054, TUI-NAV-020
#[test]
fn an_idle_chip_never_asks_for_a_kitten_who_is_not_here() {
    let mut state = every_section_revealed();
    let mut roster = idle_away(6);
    roster.extend(employed(2));
    state.kittens = roster;
    let view = calm_view(&state);

    let line = header_line(&state, &view, WIDE_W);
    assert!(
        !chip_shown(&line, IDLE),
        "every jobless kitten is away from the meadow, so there is nobody to assign — and the \
         header still showed an idle chip. A chip whose action cannot be taken is worse than no \
         chip, because it teaches the player that the cluster can be ignored.\nheader: {line:?}"
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-044 — each chip clears on its own signal
// ---------------------------------------------------------------------------

/// Four chips, four signals, and no crossed wires. The point of "each on its
/// own signal" is that reading the away report must not silence the sky window
/// — a cluster where one action clears somebody else's chip loses exactly the
/// thing the glance contract promises.
///
/// @spec TUI-NAV-044, TUI-NAV-020
#[test]
fn each_chip_clears_on_its_own_signal_and_leaves_the_others_alone() {
    let (state, view) = all_four_live();
    let line = header_line(&state, &view, WIDE_W);
    assert_eq!(
        live_chips(&line),
        vec![IDLE.1, SKY.1, NEW.1, AWAY.1],
        "fixture: all four opportunities are live, so all four chips should be on the header. It \
         read as {line:?}"
    );

    // Each signal, applied on its own.
    let assigned = {
        let mut s = state.clone();
        for k in s.kittens.iter_mut() {
            if k.location == Location::Home {
                k.job = Some(JobId::Gardener);
            }
        }
        (s, view.clone())
    };
    let observed = {
        let mut s = state.clone();
        s.pending_sky_event = None;
        (s, view.clone())
    };
    let visited = {
        let mut v = view.clone();
        v.seen[Section::Clowder.index()].revealed_at_last_visit =
            rows(&state, Section::Clowder).len();
        (state.clone(), v)
    };
    let dismissed = {
        let mut v = view.clone();
        for r in v.reports.iter_mut() {
            r.dismissed = true;
        }
        (state.clone(), v)
    };

    let cases: [(&str, (char, &str), (GameState, ViewState)); 4] = [
        ("every idle kitten was given work", IDLE, assigned),
        ("the sky was observed", SKY, observed),
        ("the section with new content was visited", NEW, visited),
        ("the away report was read", AWAY, dismissed),
    ];

    for (signal, cleared, (s, v)) in cases {
        let line = header_line(&s, &v, WIDE_W);
        assert!(
            !chip_shown(&line, cleared),
            "{signal} and the {} chip is still on the header: {line:?}",
            chip_name(cleared)
        );
        for other in CHIPS.iter().filter(|c| c.1 != cleared.1) {
            assert!(
                chip_shown(&line, *other),
                "{signal}, which cleared the {} chip as well as the {} one. Each chip clears on \
                 its own signal; a chip that goes quiet because something *else* was dealt with \
                 has stopped tracking its own opportunity.\nheader: {line:?}",
                chip_name(*other),
                chip_name(cleared)
            );
        }
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-021 — one column per symbol
// ---------------------------------------------------------------------------

/// The blocks the design's glyph discipline admits: geometric shapes, box
/// drawing, block elements, arrows, and a short list of named single-width
/// marks. Everything else — every emoji, every fullwidth form, every glyph a
/// terminal might render two columns wide — is out, because column arithmetic
/// that disagrees with the terminal's is column arithmetic that breaks inside
/// tmux and not on the developer's machine.
fn permitted_symbol(c: char) -> bool {
    if c == ' ' || c.is_ascii_graphic() {
        return true;
    }
    matches!(
        c as u32,
        0x00B7                  // · middle dot, the design's separator
        | 0x00D7                // × the speed marker
        | 0x2013 | 0x2014       // – — dashes
        | 0x2026                // … the truncation mark (TUI-RENDER-014)
        | 0x2190..=0x21FF       // arrows
        | 0x2500..=0x257F       // box drawing
        | 0x2580..=0x259F       // block elements (the fill bar)
        | 0x25A0..=0x25FF       // geometric shapes (● ○ ◐ ▸ ◦)
        | 0x2605 | 0x2606       // ★ ☆
        | 0x2713 | 0x2717       // ✓ ✗
    )
}

/// Every cell the chrome draws holds exactly one character, and that character
/// is one the design admits. Two failure modes are caught here: a grapheme
/// built from several code points (an emoji with a variation selector is the
/// usual one), and a code point outside the geometric/box-drawing vocabulary.
///
/// @spec TUI-NAV-021
#[test]
fn every_symbol_the_chrome_draws_is_a_single_width_character() {
    let (busy, busy_view) = all_four_live();
    let fresh = GameState::new(3);
    let fresh_view = calm_view(&fresh);

    let mut checked = 0usize;
    for (state, view) in [(&busy, &busy_view), (&fresh, &fresh_view)] {
        for w in [FLOOR_W, 100, 120, WIDE_W] {
            let regions: [(&str, String); 3] = [
                ("header", header_line(state, view, w)),
                ("tab bar", tab_line(state, view, w)),
                ("hint bar", hint_line(state, view, w)),
            ];
            for (what, line) in regions {
                for c in line.chars() {
                    checked += 1;
                    assert!(
                        permitted_symbol(c),
                        "the {what} drew U+{:04X} ({c:?}) at width {w}. UI symbols are \
                         single-width geometric and box-drawing characters only: an emoji or an \
                         ambiguous-width glyph is measured differently by different terminals, \
                         and every column of the chrome after it lands somewhere else.\nline: \
                         {line:?}",
                        c as u32
                    );
                }
            }
        }

        for line in sidebar_lines(state, FLOOR_W, FLOOR_H) {
            for c in line.chars() {
                checked += 1;
                assert!(
                    permitted_symbol(c),
                    "the sidebar drew U+{:04X} ({c:?}).\nline: {line:?}",
                    c as u32
                );
            }
        }
    }
    assert!(checked > 0, "fixture: this sweep should have read some cells");

    // And the grapheme test, which the character sweep above cannot make: a
    // buffer cell holding more than one code point is a cluster the terminal
    // may or may not collapse into one column.
    let (header, tabs, sidebar, hints) = regions(WIDE_W, WIDE_H);
    let mut term = terminal(WIDE_W, WIDE_H);
    term.draw(|f| {
        chrome::header(f, header, &busy, &busy_view);
        chrome::tabs(f, tabs, &busy, &busy_view);
        chrome::sidebar(f, sidebar, &busy);
        chrome::hints(f, hints, &busy, &busy_view);
    })
    .expect("TestBackend is infallible");
    let buffer = term.backend().buffer().clone();
    for rect in [header, tabs, sidebar, hints] {
        for y in rect.y..rect.bottom() {
            for x in rect.x..rect.right() {
                let symbol = buffer.cell((x, y)).expect("every cell of the area exists").symbol();
                assert_eq!(
                    symbol.chars().count(),
                    1,
                    "cell ({x},{y}) of the chrome holds {symbol:?}, which is {} code points. A \
                     multi-code-point cluster is exactly how an emoji with a variation selector \
                     gets on screen, and its width is whatever the terminal decides.",
                    symbol.chars().count()
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-022 — nothing is lost when the colour is taken away
// ---------------------------------------------------------------------------

/// Mood is a word. Five tiers, five words, and the words alone tell them apart
/// — which is what makes the header readable to a player who sees no colour at
/// all, and speakable by a screen reader.
///
/// @spec TUI-NAV-022, TUI-NAV-019
#[test]
fn the_mood_is_a_word_that_survives_the_colour_being_stripped() {
    let drawn: Vec<(MoodTier, String)> = MOODS
        .into_iter()
        .map(|tier| {
            let state = moody(tier);
            let view = calm_view(&state);
            (tier, header_line(&state, &view, WIDE_W))
        })
        .collect();

    for (tier, line) in &drawn {
        assert!(
            says(line, mood_name(*tier)),
            "a {tier:?} clowder's header does not say so in words; it read {line:?}. Mood takes \
             no colour of its own precisely because the word already says it, unambiguously and \
             in a form no colour deficiency can compress."
        );
    }

    // Five states that differ only in mood must draw five different lines: the
    // distinction survives with every style thrown away.
    for (i, (tier_a, a)) in drawn.iter().enumerate() {
        for (tier_b, b) in drawn.iter().skip(i + 1) {
            assert_ne!(
                a, b,
                "a {tier_a:?} clowder and a {tier_b:?} one drew the same header text. Whatever \
                 separates them on screen is then colour alone, and the game stops being \
                 playable without colour perception."
            );
        }
    }
}

/// A store at its cap says so in words. The bar is full and the tag reads
/// `full`; neither of those is a hue, which is the whole requirement — a player
/// who cannot see the colour must still know that production is going nowhere.
///
/// @spec TUI-NAV-022
#[test]
fn a_capped_store_says_so_in_words_and_not_only_in_colour() {
    let mut capped = GameState::new(11);
    let cap = capped.cap(ResourceId::Catnip);
    assert!(cap > 0.0, "fixture: catnip needs a finite cap for this test to mean anything");
    capped.resources[ResourceId::Catnip.ix()] = cap;

    let mut room_to_spare = GameState::new(11);
    room_to_spare.resources[ResourceId::Catnip.ix()] = cap / 2.0;

    let full = sidebar_lines(&capped, FLOOR_W, FLOOR_H);
    let half = sidebar_lines(&room_to_spare, FLOOR_W, FLOOR_H);

    assert!(
        full.iter().any(|l| says(l, "full")),
        "a store sitting at its cap drew no `full` tag. Capped storage is one of the four \
         meanings that must reach the player through something other than colour, and the tag is \
         the design's channel for it.\nsidebar read as:\n{}",
        full.join("\n")
    );
    assert!(
        !half.iter().any(|l| says(l, "full")),
        "a store at half its cap was tagged `full`; a tag that is always there says nothing.\n\
         sidebar read as:\n{}",
        half.join("\n")
    );
    assert_ne!(
        full,
        half,
        "a full store and a half-full one drew the same sidebar text, so the fill is being told \
         by colour alone"
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-034 — the responsive tab bar
// ---------------------------------------------------------------------------

/// Every section's four-character head, checked for collisions so that finding
/// one in the bar can only mean that section's tab.
fn name_head(section: Section) -> String {
    let name = section.title();
    let head: String = name.chars().take(4).collect();
    let clashes = Section::ALL
        .into_iter()
        .filter(|o| *o != section && o.title().starts_with(&head))
        .count();
    assert_eq!(clashes, 0, "fixture: {head:?} does not identify one section");
    head
}

/// Where there is room, the tab bar names every section beside its digit. The
/// digit is the stable identity and the name is the content — a player learns
/// the pairing here and nowhere else.
///
/// @spec TUI-NAV-034
#[test]
fn a_wide_tab_bar_names_every_section_beside_its_digit() {
    let state = every_section_revealed();
    let view = calm_view(&state);
    let line = tab_line(&state, &view, WIDE_W);

    for section in Section::ALL {
        let name = section.title();
        let at = line.find(name).unwrap_or_else(|| {
            panic!(
                "at {WIDE_W} columns every revealed tab fits, and the {name} tab was not named.\n\
                 tab bar: {line:?}"
            )
        });
        let lead: String = line[..at].chars().rev().take(4).collect();
        assert!(
            lead.contains(section.digit()),
            "the {name} tab is drawn without its digit `{}` beside it. The digit is the section's \
             stable identity — the one thing that never renumbers — and a name without it teaches \
             the player nothing about how to get back.\ntab bar: {line:?}",
            section.digit()
        );
    }
}

/// Ten named tabs do not fit eighty columns, so the bar compacts to digits —
/// and compacting is not the same as dropping. Every tab keeps its digit, and
/// the two tabs a player needs words for keep their names: the one they are on,
/// and the one that has just appeared.
///
/// @spec TUI-NAV-034
#[test]
fn a_narrow_tab_bar_compacts_to_digits_and_still_reaches_every_tab() {
    let state = every_section_revealed();
    let focused = Section::Meadow;
    let badged = Section::Sky;

    let mut view = calm_view(&state);
    view.section = Some(focused);
    view.seen[badged.index()] = SeenMark { visited: false, revealed_at_last_visit: 0 };

    let widest: usize = Section::ALL.into_iter().map(|s| s.title().len() + 2).sum();
    assert!(
        widest > FLOOR_W as usize,
        "fixture: ten named tabs must genuinely not fit {FLOOR_W} columns"
    );

    let line = tab_line(&state, &view, FLOOR_W);

    for section in Section::ALL {
        assert!(
            line.contains(section.digit()),
            "the {} tab's digit `{}` is missing from an {FLOOR_W}-column tab bar. A compact strip \
             may drop names; it may never put a tab out of reach.\ntab bar: {line:?}",
            section.title(),
            section.digit()
        );
    }

    for section in [focused, badged] {
        assert!(
            line.contains(section.title()),
            "the compact strip does not label the {} tab. The strip labels the focused tab and \
             any newly badged one — those are the two a bare digit cannot tell the player \
             anything about.\ntab bar: {line:?}",
            section.title()
        );
    }

    let named = Section::ALL.into_iter().filter(|s| line.contains(s.title())).count();
    assert!(
        named < Section::ALL.len(),
        "every tab is still named at {FLOOR_W} columns, which cannot fit — so something has been \
         cut off the end of the line.\ntab bar: {line:?}"
    );
}

/// A name is either drawn or not drawn. Half a name is a tab a player cannot
/// identify and a word the eye trips over, and the design spends the digit
/// strip rather than ever letting one appear.
///
/// @spec TUI-NAV-034
#[test]
fn no_tab_name_is_ever_cut_mid_word() {
    let state = every_section_revealed();
    let mut view = calm_view(&state);
    view.section = Some(Section::Ideas);
    view.seen[Section::Dreams.index()] = SeenMark { visited: false, revealed_at_last_visit: 0 };

    for w in FLOOR_W..=WIDE_W {
        let line = tab_line(&state, &view, w);
        assert!(
            !line.contains('…') && !line.contains("..."),
            "the tab bar truncated at {w} columns: {line:?}. Names are dropped, never cut — the \
             digit is what identifies a section when the words will not fit."
        );
        for section in Section::ALL {
            let head = name_head(section);
            if line.contains(&head) {
                assert!(
                    line.contains(section.title()),
                    "at {w} columns the tab bar contains {head:?} but not the whole word {:?}, so \
                     the {} tab is drawn half-named.\ntab bar: {line:?}",
                    section.title(),
                    section.title()
                );
            }
        }
    }
}

/// However narrow the pane, every revealed section is one keypress away. The
/// bar never scrolls: a tab the player cannot see is a tab they have no reason
/// to believe still exists.
///
/// @spec TUI-NAV-034
#[test]
fn every_revealed_tab_is_reachable_at_every_width() {
    let state = every_section_revealed();
    let view = calm_view(&state);

    for w in FLOOR_W..=WIDE_W {
        let line = tab_line(&state, &view, w);
        for section in Section::ALL {
            assert!(
                line.contains(section.digit()),
                "at {w} columns the {} tab's digit `{}` is not on the bar. The bar never scrolls \
                 and never drops a tab; only the names give way.\ntab bar: {line:?}",
                section.title(),
                section.digit()
            );
        }
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-023, TUI-NAV-045 — the hint bar
// ---------------------------------------------------------------------------

/// The keys a hint bar can name, as whole tokens. Matching tokens rather than
/// substrings is what stops `Tab next` from reading as an offer of `x`.
const PROBE_KEYS: [&str; 11] =
    ["↑↓", "Enter", "Space", "Tab", "Esc", "x", "X", "+", "-", "i", "?"];

/// The keys a line advertises, in the order it advertises them.
fn keys_in_order(line: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for token in tokens(line) {
        if PROBE_KEYS.contains(&token.as_str()) && !out.contains(&token) {
            out.push(token);
        }
    }
    out
}

/// Whether the last thing on the line is the help key and its word — nothing
/// after `?` but the letters that name it.
fn ends_with_help_key(line: &str) -> bool {
    let trimmed = line.trim_end();
    match trimmed.rfind('?') {
        None => false,
        Some(at) => trimmed[at + 1..].chars().all(|c| c.is_alphabetic() || c.is_whitespace()),
    }
}

/// Every context the hint bar has to speak for, so a claim about "always" is
/// quantified over something.
fn hint_contexts(state: &GameState) -> Vec<(&'static str, ViewState)> {
    let mut armed = focus_on(state, Section::Legacy, heavy_row(state));
    armed.armed = Some(ArmedRow {
        key: heavy_row(state),
        command: Command::GreatMeadowMove,
        expires_at: Instant::now() + Duration::from_secs(8),
    });

    let mut prompt = focus_on(state, Section::Meadow, repeatable_row(state));
    prompt.prompt = Some(CountPrompt { digits: "12".to_string() });

    vec![
        ("a plain one-shot row", focus_on(state, Section::Meadow, single_row(state))),
        ("a buildable row", focus_on(state, Section::Meadow, repeatable_row(state))),
        ("a count row", focus_on(state, Section::Clowder, count_row(state))),
        ("a count-taking row", focus_on(state, Section::Clowder, count_taking_row(state))),
        ("an armed heavy row", armed),
        ("an open count prompt", prompt),
    ]
}

/// There is no manual: the hint bar and `?` are the manual. So whatever else
/// the bar has room for, the key that opens the complete map is on it, and it
/// is the last thing there — a fixed place the eye can find without reading.
///
/// @spec TUI-NAV-023, TUI-NAV-025, TUI-NAV-045
#[test]
fn the_hint_bar_ends_with_the_help_key_in_every_context_and_at_every_width() {
    let state = every_section_revealed();

    for (what, view) in hint_contexts(&state) {
        for w in [8u16, 12, 20, 40, FLOOR_W, 120, WIDE_W] {
            let line = hint_line(&state, &view, w);
            assert!(
                ends_with_help_key(&line),
                "with {what} focused, an {w}-column hint bar read {line:?} — the help key is not \
                 the last thing on it. The complete key map is only ever reachable through `?`, \
                 so it is the one hint that is never dropped and never moves."
            );
        }
    }
}

/// The bar adapts. Six contexts, six different lines: a hint bar that says the
/// same thing wherever the cursor is has stopped teaching anything, and the
/// speed layer never gets learned because it is never mentioned at the moment
/// it would help.
///
/// @spec TUI-NAV-023
#[test]
fn the_hint_bar_changes_with_the_focused_context() {
    let state = every_section_revealed();
    let drawn: Vec<(&str, String)> = hint_contexts(&state)
        .into_iter()
        .map(|(what, view)| (what, hint_line(&state, &view, WIDE_W)))
        .collect();

    for (i, (what_a, a)) in drawn.iter().enumerate() {
        for (what_b, b) in drawn.iter().skip(i + 1) {
            assert_ne!(
                a, b,
                "{what_a} and {what_b} drew the same hint bar. The bar's job is to name the keys \
                 that are live *here*; one that does not change is a fixed legend, and a fixed \
                 legend is the manual this design does not have."
            );
        }
    }
}

/// The keys a context actually offers, named where they are relevant. A count
/// row is driven by `+` and `-`; a row whose command takes or repeats a count
/// is where `x` is worth teaching; a row that is neither is a row where `x` is
/// a no-op, and advertising it there teaches a key that does nothing.
///
/// @spec TUI-NAV-023
#[test]
fn the_hint_bar_names_the_keys_the_focused_row_actually_offers() {
    let state = every_section_revealed();

    let counting = hint_line(&state, &focus_on(&state, Section::Clowder, count_row(&state)), WIDE_W);
    for key in ["+", "-"] {
        assert!(
            has_key(&counting, key),
            "a count row is worked with `+` and `-` and the hint bar named neither: {counting:?}"
        );
    }

    for (what, section, key) in [
        ("a count-taking row", Section::Clowder, count_taking_row(&state)),
        ("a buildable row", Section::Meadow, repeatable_row(&state)),
    ] {
        let line = hint_line(&state, &focus_on(&state, section, key), WIDE_W);
        assert!(
            has_key(&line, "x"),
            "{what} is focused and the hint bar did not surface the count key. The speed layer is \
             taught lazily, at the moment it becomes relevant — and this is that moment.\nhint \
             bar: {line:?}"
        );
    }

    let plain = hint_line(&state, &focus_on(&state, Section::Meadow, single_row(&state)), WIDE_W);
    assert!(
        !has_key(&plain, "x"),
        "the focused row's command is neither count-taking nor usefully repeatable, so `x` there \
         is a no-op with a footer note — and the hint bar offered it anyway: {plain:?}"
    );
}

/// When the keys will not fit, the least relevant go first and the help key
/// stays. Stated as the property that makes that checkable: what a narrow bar
/// shows is a *prefix* of what a wide one shows, because dropping from the
/// least-relevant end cannot reorder what is left, and the complete map is
/// always still one keypress away.
///
/// @spec TUI-NAV-045, TUI-NAV-023, TUI-NAV-025
#[test]
fn the_hint_bar_drops_the_least_relevant_key_first_and_never_the_help_key() {
    let state = every_section_revealed();
    let view = focus_on(&state, Section::Clowder, count_row(&state));

    let widths = [WIDE_W, 120, FLOOR_W, 60, 44, 32, 24, 16, 12];
    let mut previous: Option<(u16, Vec<String>)> = None;
    let mut ever_shed = false;

    for w in widths {
        let line = hint_line(&state, &view, w);
        assert!(
            !line.contains('…') && !line.contains("..."),
            "the hint bar was truncated at {w} columns rather than dropping a whole hint: \
             {line:?}. A half-written key teaches a key that does not exist."
        );
        assert!(
            ends_with_help_key(&line),
            "at {w} columns the hint bar lost the help key: {line:?}"
        );

        let keys: Vec<String> =
            keys_in_order(&line).into_iter().filter(|k| k != "?").collect();

        if let Some((wider_w, wider)) = &previous {
            assert!(
                wider.starts_with(&keys[..]),
                "shrinking from {wider_w} to {w} columns left {keys:?}, which is not a prefix of \
                 {wider:?}. Hints are dropped from the least-relevant end, so what survives is \
                 what was most relevant — a set that reorders, or that keeps a later hint while \
                 dropping an earlier one, is not doing that."
            );
            if keys.len() < wider.len() {
                ever_shed = true;
            }
        }
        previous = Some((w, keys));
    }

    assert!(
        ever_shed,
        "the hint bar named the same keys from {WIDE_W} columns down to 12, so this test never \
         exercised the overflow it is about. Either the bar is not adapting at all, or the \
         context it was given had nothing to shed."
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-024 — the first sixty seconds
// ---------------------------------------------------------------------------

/// A cold open teaches the backbone and nothing else: move the cursor, do the
/// thing, see the truth, go to the next section — plus the key that opens
/// everything else. Layer one plays the whole game, and this line is where a
/// player learns it.
///
/// @spec TUI-NAV-024, TUI-NAV-023
#[test]
fn a_cold_open_teaches_the_backbone() {
    let state = GameState::new(3);
    let first = rows(&state, Section::Meadow)
        .first()
        .expect("a new meadow has at least one row")
        .key
        .clone();
    let view = focus_on(&state, Section::Meadow, first);

    let line = hint_line(&state, &view, FLOOR_W);

    assert!(
        line.contains('↑') && line.contains('↓'),
        "the first hint bar a player ever sees does not name the cursor keys: {line:?}. The \
         cursor is the backbone — everything else in the scheme is an accelerator on top of it."
    );
    for key in ["Enter", "Space", "Tab"] {
        assert!(
            has_key(&line, key),
            "the cold-open hint bar does not name `{key}`. The only possible actions on a new \
             game are the ones that teach ↑↓ / Enter / Space / Tab, and this line is the whole of \
             the tutorial.\nhint bar: {line:?}"
        );
    }
    assert!(ends_with_help_key(&line), "and it ends with the key to everything else: {line:?}");
}

/// On a new game the Meadow is the only section there is. Tab names are content
/// — a tab for a section whose content the player has not reached would be a
/// tease, and it would make the first screen look like a menu instead of a
/// meadow.
///
/// @spec TUI-NAV-024
#[test]
fn on_a_new_game_the_meadow_is_the_only_section_with_a_tab() {
    let state = GameState::new(3);

    assert_eq!(
        rows(&state, Section::Meadow).first().map(|r| r.key.clone()),
        Some(RowKey::Fixed("GatherCatnip")),
        "a new game's first row is the gather action — the one thing a cold player can do"
    );

    // The tab bar renders what the row builder reveals, so "the only revealed
    // section" is decided one layer below the chrome. Asserting it here says
    // where a failure lives rather than leaving it to look like a drawing bug.
    let with_rows: Vec<&'static str> = Section::ALL
        .into_iter()
        .filter(|s| !rows(&state, *s).is_empty())
        .map(Section::title)
        .collect();
    assert_eq!(
        with_rows,
        vec![Section::Meadow.title()],
        "a new game reveals rows in {} sections, so the tab bar cannot show only the Meadow \
         however carefully it is drawn. TUI-NAV-024 says the cold open is one section with one \
         focused row; TUI-NAV-006 says a tab exists the moment one of its rows is revealed.",
        with_rows.len()
    );

    let view = calm_view(&state);
    let line = tab_line(&state, &view, WIDE_W);

    assert!(
        line.contains(Section::Meadow.title()),
        "a new game's tab bar does not name the Meadow: {line:?}"
    );
    for section in Section::ALL.into_iter().filter(|s| *s != Section::Meadow) {
        assert!(
            !line.contains(section.title()),
            "a new game's tab bar names the {} section. On a cold open only the Meadow exists: a \
             section's tab appears when one of its rows does, and nothing in {} has been reached \
             yet.\ntab bar: {line:?}",
            section.title(),
            section.title()
        );
        assert!(
            !line.contains(section.digit()),
            "a new game's tab bar carries the digit `{}`, which belongs to the {} section — a \
             section the player has not reached.\ntab bar: {line:?}",
            section.digit(),
            section.title()
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-025 — the manual the hint bar points at
// ---------------------------------------------------------------------------

/// Every glyph the interface uses to carry a meaning. If the legend does not
/// explain one, the player has a symbol on screen with nowhere to look it up.
const LEGEND_GLYPHS: [char; 9] = ['●', '◐', '○', '✓', '▸', '★', '◦', '~', '!'];

/// The keys whose spelling is not in question: a word, or a single character
/// that is itself the key.
const HELP_WORDS: [&str; 4] = ["Enter", "Space", "Tab", "Esc"];
const HELP_KEYS: [&str; 9] = ["?", "/", "x", "X", "q", "e", "o", "<", ">"];

/// `?` is the whole manual: the complete key map, and the legend for every
/// glyph the chrome and the rows put on screen. The hint bar shows what is live
/// here; this shows what exists at all, which is why nothing may be reachable
/// only through a key the hint bar happened to have room for.
///
/// @spec TUI-NAV-025, TUI-NAV-045
#[test]
fn the_help_overlay_is_the_complete_key_map_and_glyph_legend() {
    let mut term = terminal(120, 60);
    term.draw(|f| {
        let area = f.area();
        overlays::help(f, area);
    })
    .expect("TestBackend is infallible");
    let buffer = term.backend().buffer().clone();
    let text = region_lines(&buffer, Rect::new(0, 0, 120, 60)).join("\n");

    for glyph in LEGEND_GLYPHS {
        assert!(
            text.contains(glyph),
            "the help overlay's glyph legend does not explain {glyph:?}. Every marker is \
             glyph-first by design, so a glyph the manual never names is a state the player can \
             see and cannot read."
        );
    }
    for word in HELP_WORDS {
        assert!(
            says(&text, word),
            "the help overlay does not list the `{word}` key. `?` is the complete key map — there \
             is no other manual for it to be in."
        );
    }
    for key in HELP_KEYS {
        assert!(
            has_key(&text, key),
            "the help overlay does not list the `{key}` key. `?` is the complete key map — there \
             is no other manual for it to be in."
        );
    }

    // And it is genuinely more than the hint bar: the hint bar carries what is
    // live in one context, and the overlay is what "the complete key map
    // remaining available through the help overlay" means.
    let state = every_section_revealed();
    let view = focus_on(&state, Section::Clowder, count_row(&state));
    let bar = hint_line(&state, &view, WIDE_W);
    assert!(
        keys_in_order(&text).len() > keys_in_order(&bar).len(),
        "the help overlay names no more keys than one hint bar does ({:?} against {:?}), so the \
         full key map is not actually anywhere",
        keys_in_order(&text),
        keys_in_order(&bar)
    );
}
