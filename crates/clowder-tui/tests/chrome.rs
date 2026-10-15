//! The persistent furniture — `render::widgets::chrome`: the header and its
//! attention chips, the tab bar, the resource sidebar, and the hint bar.
//!
//! @spec TUI-NAV-019, TUI-NAV-020, TUI-NAV-021, TUI-NAV-022, TUI-NAV-023,
//! TUI-NAV-024, TUI-NAV-025, TUI-NAV-034, TUI-NAV-044, TUI-NAV-045, TUI-NAV-054,
//! TUI-NAV-086, TUI-NAV-092, TUI-THEME-011, TUI-THEME-015, TUI-THEME-019,
//! TUI-THEME-020, TUI-THEME-021, TUI-RENDER-016
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
//! a chip as glyph and word with a count between them on the two that count,
//! the header's width steps (TUI-NAV-086), a capped store's text `full` tag,
//! the help key last on the hint bar — the form is asserted directly.
//!
//! **Fixtures assert themselves.** Several states here are built to sit in a
//! particular mood tier or to reveal a particular set of sections. Each such
//! fixture checks that it really did, with a message beginning `fixture:`, so a
//! balance change in the core fails as a broken fixture rather than silently
//! turning a test into a weaker one.

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use clowder_core::{
    content, ArchId, ColonyId, Command, ContentId, GameState, IdeaId, JobId, Kitten, KittenTrait,
    Location, LockReason, MoodTier, ResourceId, RuleId, Season, StructureId,
};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::Terminal;

use clowder_tui::nav::palette::results;
use clowder_tui::nav::rows::{drawn_rows, rows, tab_sections, Row, RowKey, RowKind};
use clowder_tui::nav::{
    opening, reset_hidden_tabs, update, Action, ArmedRow, CountPrompt, CountSemantics, Overlay,
    PendingReport, Section, SeenMark, ViewState,
};
use clowder_tui::render::fmt;
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
    (0..n)
        .map(|_| kitten(Some(JobId::Gardener), Location::Home))
        .collect()
}

/// Jobless kittens in the meadow: exactly what `◦N idle` counts.
fn idle_here(n: usize) -> Vec<Kitten> {
    (0..n).map(|_| kitten(None, Location::Home)).collect()
}

/// Jobless kittens visiting the neighbour's yard. Just as jobless, and not
/// assignable by anyone — TUI-NAV-054's whole subject.
fn idle_away(n: usize) -> Vec<Kitten> {
    (0..n)
        .map(|_| kitten(None, Location::NeighborsYard))
        .collect()
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
        marks[section.index()] = SeenMark {
            visited: true,
            revealed_at_last_visit: rows(state, section).len(),
        };
    }
    marks
}

/// A view over a state with nothing outstanding: every section seen, no report
/// unread, no prompt, no overlay, no armed row.
fn calm_view(state: &GameState) -> ViewState {
    ViewState {
        section: Some(Section::Meadow),
        seen: all_caught_up(state),
        ..Default::default()
    }
}

/// A view focused on one row of one section, with nothing else outstanding.
fn focus_on(state: &GameState, section: Section, key: RowKey) -> ViewState {
    let mut v = calm_view(state);
    v.section = Some(section);
    v.focus[section.index()] = Some(key);
    unfold_focus(state, &mut v);
    v
}

/// `d` pressed as the player presses it, the repeat clock cleared after.
fn press_done_key(state: &GameState, view: &mut ViewState) {
    let d = clowder_tui::nav::keys::resolve(
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('d'),
            crossterm::event::KeyModifiers::NONE,
        ),
        view,
    );
    update(d, view, state);
    view.last_repeat = None;
}

/// Where `view`'s focused row is listed but folded into the Ideas section's
/// done group, open the group by its key — the one way a player reaches such
/// a row (TUI-NAV-113) — keeping the focus on that row.
fn unfold_focus(state: &GameState, view: &mut ViewState) {
    let Some(section) = view.section else { return };
    let Some(key) = view.focus[section.index()].clone() else {
        return;
    };
    if drawn_rows(state, section, view)
        .iter()
        .any(|r| r.key == key)
        || !rows(state, section).iter().any(|r| r.key == key)
    {
        return;
    }
    press_done_key(state, view);
    view.focus[section.index()] = Some(key);
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
            panic!(
                "fixture: the {} section should have {what}",
                section.title()
            )
        })
        .key
}

fn single_row(state: &GameState) -> RowKey {
    first_row_where(state, Section::Meadow, "a plain one-shot row", |r| {
        !r.heavy
            && matches!(
                r.kind,
                RowKind::Act {
                    semantics: CountSemantics::Single,
                    ..
                }
            )
    })
}

fn repeatable_row(state: &GameState) -> RowKey {
    first_row_where(state, Section::Meadow, "a repeat-N row", |r| {
        matches!(
            r.kind,
            RowKind::Act {
                semantics: CountSemantics::RepeatN,
                ..
            }
        )
    })
}

fn count_taking_row(state: &GameState) -> RowKey {
    first_row_where(state, Section::Clowder, "a count-taking row", |r| {
        matches!(
            r.kind,
            RowKind::Act {
                semantics: CountSemantics::CountTaking,
                ..
            }
        )
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
    term.draw(|f| chrome::sidebar(f, side, state))
        .expect("TestBackend is infallible");
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
    line.to_ascii_lowercase()
        .contains(&word.to_ascii_lowercase())
}

// ---------------------------------------------------------------------------
// The attention chips
// ---------------------------------------------------------------------------

/// The four chips, each as the design states it: a glyph, then a count where it
/// has one, then a word — so no chip depends on colour.
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
    CHIPS
        .iter()
        .filter(|c| chip_shown(line, **c))
        .map(|c| c.1)
        .collect()
}

/// How many tabs the bar draws wearing `★`, read from what `chrome::tabs`
/// draws at a width where every tab is named — never from the view's own
/// bookkeeping, since the chip is defined by the bar.
fn badged_tab_count(state: &GameState, view: &ViewState) -> usize {
    tab_line(state, view, WIDE_W)
        .chars()
        .filter(|c| *c == '★')
        .count()
}

/// The sections with a tab on the bar other than the focused one, in digit
/// order — the tabs a test may badge without contradicting the rule that the
/// section in focus never wears `★`.
fn unfocused_tabs(state: &GameState, view: &ViewState) -> Vec<Section> {
    tab_sections(state, view.section)
        .into_iter()
        .filter(|s| Some(*s) != view.section)
        .collect()
}

/// A settled meadow and a calm view with a chosen set of chips live, in the
/// order idle, sky, new, away: `idle` kittens waiting in the meadow when the
/// first is live, and `new` unfocused tabs wearing `★` when the third is.
fn live_chips_fixture(live: [bool; 4], idle: usize, new: usize) -> (GameState, ViewState) {
    let mut state = every_section_revealed();
    let mut roster = idle_here(if live[0] { idle } else { 0 });
    roster.extend(idle_away(2));
    roster.extend(employed(3));
    state.kittens = roster;
    state.pending_sky_event = if live[1] { Some(state.ticks + 5) } else { None };

    let mut view = calm_view(&state);
    if live[2] {
        let others = unfocused_tabs(&state, &view);
        assert!(
            others.len() >= new,
            "fixture: {new} tabs to badge, only {} on the bar",
            others.len()
        );
        for section in &others[..new] {
            view.seen[section.index()].visited = false;
        }
    }
    if live[3] {
        view.reports = vec![PendingReport {
            entry: 0,
            dismissed: false,
        }];
    }
    (state, view)
}

/// The chip cluster the design pins for a set of live chips: idle, sky, new,
/// away in that order, two blank columns apart, the counting chips carrying
/// `counts` (idle, new) between glyph and word where given and none where not.
fn chip_cluster(live: [bool; 4], counts: Option<(usize, usize)>) -> String {
    let (idle, new) = match counts {
        Some((i, n)) => (format!("◦{i} idle"), format!("★{n} new")),
        None => ("◦idle".to_string(), "★new".to_string()),
    };
    let forms = [idle, "!sky".to_string(), new, "~away".to_string()];
    forms
        .iter()
        .zip(live)
        .filter(|(_, on)| *on)
        .map(|(f, _)| f.clone())
        .collect::<Vec<_>>()
        .join("  ")
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
    // The Clowder section's tab wears `★` — it has appeared on the bar since
    // the player last looked — which is what `★N new` counts.
    view.seen[Section::Clowder.index()].visited = false;
    assert_eq!(
        badged_tab_count(&state, &view),
        1,
        "fixture: exactly the Clowder tab should wear ★ on the bar"
    );
    view.reports = vec![PendingReport {
        entry: 0,
        dismissed: false,
    }];

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

        for (what, rect) in [
            ("header", header),
            ("tab bar", tabs),
            ("sidebar", sidebar),
            ("hint bar", hints),
        ] {
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

    assert!(
        !blank(&little),
        "a meadow with stores must have a sidebar that says so"
    );
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

/// `★N new` counts the tabs on the bar wearing `★`, N being how many do, and
/// says nothing while none does. Asserted for every number of badged tabs the
/// bar can hold beside the focused one, with the chip's count read from the
/// header and the badges read from the tab bar drawn from the same inputs, so
/// the chip can never disagree with the bar it points at.
///
/// @spec TUI-NAV-020
#[test]
fn the_new_content_chip_counts_the_tabs_wearing_a_badge() {
    let state = every_section_revealed();
    let base = calm_view(&state);
    let others = unfocused_tabs(&state, &base);
    assert!(
        others.len() >= 9,
        "fixture: every section should have a tab in a meadow this far along, got {others:?}"
    );

    for k in 0..=others.len() {
        let mut view = base.clone();
        for section in &others[..k] {
            view.seen[section.index()].visited = false;
        }
        assert_eq!(
            badged_tab_count(&state, &view),
            k,
            "fixture: {k} tabs should wear ★"
        );

        let line = header_line(&state, &view, WIDE_W);
        if k == 0 {
            assert!(
                !chip_shown(&line, NEW),
                "no tab on the bar wears ★ and the header still shows the new-content chip. The \
                 chip counts badged tabs and nothing else, so with none there is nothing new.\n\
                 header: {line:?}"
            );
        } else {
            assert_eq!(
                chip_count(&line, NEW),
                Some(k as u64),
                "{k} tabs on the bar wear ★, so the chip must read ★{k} new — the number the \
                 player can find by looking at the tab bar.\nheader: {line:?}\ntabs:   {:?}",
                tab_line(&state, &view, WIDE_W)
            );
        }
    }

    // A section with no tab on the bar is counted nowhere, however its own
    // bookkeeping reads: a badge that is not drawn marks nothing, and a chip
    // counting it would point at no place on screen.
    let fresh = GameState::new(3);
    let mut view = ViewState {
        section: Some(Section::Meadow),
        ..Default::default()
    };
    view.seen[Section::Meadow.index()].visited = true;
    assert_eq!(
        badged_tab_count(&fresh, &view),
        0,
        "fixture: on a new game only the Meadow has a tab, and it has been visited"
    );
    let line = header_line(&fresh, &view, WIDE_W);
    assert!(
        !chip_shown(&line, NEW),
        "on a new game no tab but the Meadow's is on the bar and it wears no ★, yet the header \
         shows the new-content chip: {line:?}"
    );
}

/// The counting chips carry their count between glyph and word; `!sky` and
/// `~away` carry none, their glyph directly before their word. Checked with
/// every chip live at a width where the full line fits, and for several
/// counts, so a count of more than one digit is still one figure between the
/// glyph and the word.
///
/// @spec TUI-NAV-020
#[test]
fn each_chip_is_its_glyph_and_word_with_a_count_on_the_two_that_count() {
    for (idle, new) in [(1usize, 1usize), (4, 3), (12, 9)] {
        let (state, view) = live_chips_fixture([true, true, true, true], idle, new);
        let line = header_line(&state, &view, WIDE_W);
        for form in [
            format!("◦{idle} idle"),
            "!sky".to_string(),
            format!("★{new} new"),
            "~away".to_string(),
        ] {
            assert!(
                line.contains(&form),
                "with {idle} idle kittens and {new} badged tabs the header should carry the chip \
                 {form:?} exactly — glyph, then the count where the chip counts, then its word.\n\
                 header: {line:?}"
            );
        }
    }
}

/// `◦N idle` shows exactly while a kitten TUI-NAV-054 counts waits for work,
/// and counts every one of them — for rosters of every size from none up,
/// with kittens at work and kittens away alongside.
///
/// @spec TUI-NAV-020
#[test]
fn the_idle_chip_shows_exactly_while_a_counted_kitten_waits() {
    for waiting in 0..=7usize {
        for (away, working) in [(0usize, 0usize), (3, 0), (0, 4), (2, 5)] {
            let mut state = every_section_revealed();
            let mut roster = idle_here(waiting);
            roster.extend(idle_away(away));
            roster.extend(employed(working));
            state.kittens = roster;
            let line = header_line(&state, &calm_view(&state), WIDE_W);
            if waiting == 0 {
                assert!(
                    !chip_shown(&line, IDLE),
                    "no kitten in the meadow waits for work ({away} away, {working} working), \
                     and the idle chip shows: {line:?}"
                );
            } else {
                assert_eq!(
                    chip_count(&line, IDLE),
                    Some(waiting as u64),
                    "{waiting} kittens in the meadow wait for work ({away} jobless away, \
                     {working} working); the chip must count exactly them.\nheader: {line:?}"
                );
            }
        }
    }
}

/// `!sky` shows exactly while the core holds a window pending and the tick has
/// not reached the expiry the core recorded for it — on both sides of that
/// boundary, and with no window at all.
///
/// @spec TUI-NAV-020
#[test]
fn the_sky_chip_shows_exactly_until_the_tick_reaches_the_recorded_expiry() {
    let base = every_section_revealed();
    let now = base.ticks;
    let cases: [(&str, Option<u64>, bool); 6] = [
        ("no window pending", None, false),
        ("a window that expired long ago", Some(now - 10), false),
        ("a window that expired a tick ago", Some(now - 1), false),
        ("a window whose expiry is this very tick", Some(now), false),
        ("a window with one tick left", Some(now + 1), true),
        ("a window with a long while left", Some(now + 1_000), true),
    ];
    for (what, pending, live) in cases {
        let mut state = base.clone();
        state.pending_sky_event = pending;
        let line = header_line(&state, &calm_view(&state), WIDE_W);
        assert_eq!(
            chip_shown(&line, SKY),
            live,
            "with {what} the sky chip should {}show.\nheader: {line:?}",
            if live { "" } else { "not " }
        );
    }
}

/// `~away` shows exactly while any report in the queue is unread, whether an
/// absence or a nap brought it — however many are queued and wherever in the
/// queue the unread one sits.
///
/// @spec TUI-NAV-020
#[test]
fn the_away_chip_shows_exactly_while_any_queued_report_is_unread() {
    let state = every_section_revealed();
    let report = |entry: usize, read: bool| PendingReport {
        entry,
        dismissed: read,
    };
    type Case = (&'static str, Vec<PendingReport>, Vec<(usize, u32)>, bool);
    let cases: Vec<Case> = vec![
        ("an empty queue", vec![], vec![], false),
        (
            "one unread absence report",
            vec![report(0, false)],
            vec![],
            true,
        ),
        (
            "one unread nap report",
            vec![report(0, false)],
            vec![(0, 3)],
            true,
        ),
        (
            "one read nap report",
            vec![report(0, true)],
            vec![(0, 3)],
            false,
        ),
        (
            "every report read",
            vec![report(2, true), report(1, true), report(0, true)],
            vec![],
            false,
        ),
        (
            "only the newest unread",
            vec![report(2, false), report(1, true), report(0, true)],
            vec![],
            true,
        ),
        (
            "only the oldest unread",
            vec![report(2, true), report(1, true), report(0, false)],
            vec![],
            true,
        ),
        (
            "an unread nap behind a read absence",
            vec![report(1, true), report(0, false)],
            vec![(0, 2)],
            true,
        ),
    ];
    for (what, reports, naps, live) in cases {
        let mut view = calm_view(&state);
        view.reports = reports;
        view.nap_years = naps;
        let line = header_line(&state, &view, WIDE_W);
        assert_eq!(
            chip_shown(&line, AWAY),
            live,
            "with {what} the away chip should {}show.\nheader: {line:?}",
            if live { "" } else { "not " }
        );
    }
}

/// The live chips stand right-aligned, in the fixed order idle, sky, new,
/// away, two blank columns apart — for every combination of live chips, so a
/// chip coming or going never moves another out of its place in the order.
///
/// @spec TUI-NAV-020
#[test]
fn the_live_chips_stand_right_aligned_in_their_fixed_order_two_columns_apart() {
    for mask in 1u8..16 {
        let live = [mask & 1 != 0, mask & 2 != 0, mask & 4 != 0, mask & 8 != 0];
        let (state, view) = live_chips_fixture(live, 3, 2);
        let expected = chip_cluster(live, Some((3, 2)));
        let line = header_line(&state, &view, WIDE_W);
        assert!(
            line.ends_with(&expected),
            "with chips {live:?} live the header should end, flush with its last column, in \
             {expected:?} — idle, sky, new, away in that order, two blank columns apart.\n\
             header: {line:?}"
        );
        let before = line[..line.len() - expected.len()].chars().last();
        assert_eq!(
            before,
            Some(' '),
            "the chip cluster must stand apart from the facts by a blank column: {line:?}"
        );
    }
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
    unread.reports = vec![PendingReport {
        entry: 0,
        dismissed: false,
    }];
    let line = header_line(&state, &unread, WIDE_W);
    assert!(
        chip_shown(&line, AWAY),
        "a while-you-were-away report is waiting and the header showed no {} chip: {line:?}",
        chip_name(AWAY)
    );

    let mut read = calm_view(&state);
    read.reports = vec![PendingReport {
        entry: 0,
        dismissed: true,
    }];
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
        v.seen[Section::Clowder.index()].visited = true;
        (state.clone(), v)
    };
    let dismissed = {
        let mut v = view.clone();
        for r in v.reports.iter_mut() {
            r.dismissed = true;
        }
        (state.clone(), v)
    };

    type Case<'a> = (&'a str, (char, &'a str), (GameState, ViewState));
    let cases: [Case; 4] = [
        ("every idle kitten was given work", IDLE, assigned),
        ("the sky was observed", SKY, observed),
        ("the badged section was visited", NEW, visited),
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

/// A settled meadow with four tabs badged, idle kittens waiting, a sky window
/// open, and `unread` reports queued — the state the new-content chip's
/// countdown is measured against.
fn countdown_fixture(unread: usize) -> (GameState, ViewState, Vec<Section>) {
    let (state, mut view) = live_chips_fixture([true, true, true, false], 4, 4);
    view.reports = (0..unread)
        .rev()
        .map(|entry| PendingReport {
            entry,
            dismissed: false,
        })
        .collect();
    let badged: Vec<Section> = unfocused_tabs(&state, &view)
        .into_iter()
        .filter(|s| !view.seen[s.index()].visited)
        .collect();
    assert_eq!(badged.len(), 4, "fixture: four tabs should be badged");
    (state, view, badged)
}

/// Visiting each badged tab lowers `★N new` by exactly one, the chip clearing
/// once no tab wears `★` — and visiting touches no other chip: the idle count
/// and the sky window stand throughout. A visit to a tab that wears no badge
/// lowers nothing. Visits are driven through the reducer, the way a digit
/// press reaches them; each press also retires one report banner (TUI-NAV-083),
/// so the queue holds one report more than the presses made, keeping `~away`
/// live to show that no visit clears it.
///
/// @spec TUI-NAV-044
#[test]
fn each_visit_to_a_badged_tab_lowers_the_new_chip_by_one_and_no_other_chip() {
    let (state, mut view, badged) = countdown_fixture(6);

    // A visit to a tab that wears no badge: nothing to count down.
    let plain = unfocused_tabs(&state, &view)
        .into_iter()
        .find(|s| view.seen[s.index()].visited)
        .expect("fixture: an unbadged tab besides the focused one");
    let before = header_line(&state, &view, WIDE_W);
    update(Action::SectionJump(plain), &mut view, &state);
    let after = header_line(&state, &view, WIDE_W);
    assert_eq!(
        chip_count(&after, NEW),
        chip_count(&before, NEW),
        "visiting {} — a tab wearing no ★ — changed the new-content chip.\n before: {before:?}\n  \
         after: {after:?}",
        plain.title()
    );

    let mut expected = badged.len();
    for section in badged {
        update(Action::SectionJump(section), &mut view, &state);
        expected -= 1;
        let line = header_line(&state, &view, WIDE_W);
        assert_eq!(
            badged_tab_count(&state, &view),
            expected,
            "fixture: the visit should clear {}'s badge",
            section.title()
        );
        if expected == 0 {
            assert!(
                !chip_shown(&line, NEW),
                "the last badged tab was visited and the new-content chip is still on the \
                 header: {line:?}"
            );
        } else {
            assert_eq!(
                chip_count(&line, NEW),
                Some(expected as u64),
                "visiting {} should lower the chip by exactly one, to {expected}: {line:?}",
                section.title()
            );
        }
        assert_eq!(
            chip_count(&line, IDLE),
            Some(4),
            "a visit changed the idle chip: {line:?}"
        );
        assert!(
            chip_shown(&line, SKY),
            "a visit cleared the sky chip: {line:?}"
        );
        assert!(
            chip_shown(&line, AWAY),
            "a report is still unread, yet the away chip is gone after a visit: {line:?}"
        );
    }
}

/// A badged tab that leaves the bar unvisited lowers `★N new` by one too: a
/// badge no longer drawn marks nothing. Its leaving touches no other chip.
///
/// @spec TUI-NAV-044
#[test]
fn a_badged_tab_leaving_the_bar_lowers_the_new_chip_by_one() {
    let with_sky = every_section_revealed();
    let mut quiet_sky = with_sky.clone();
    quiet_sky.ideas.remove(&IdeaId::SkywardLonging);
    quiet_sky.ideas.remove(&IdeaId::SkywardSong);

    let focus = Some(Section::Meadow);
    let leaving: Vec<Section> = tab_sections(&with_sky, focus)
        .into_iter()
        .filter(|s| !tab_sections(&quiet_sky, focus).contains(s))
        .collect();
    let staying: Vec<Section> = tab_sections(&quiet_sky, focus)
        .into_iter()
        .filter(|s| Some(*s) != focus)
        .collect();
    assert!(
        !leaving.is_empty(),
        "fixture: quieting the sky should take a tab off the bar"
    );
    assert!(
        !staying.is_empty(),
        "fixture: some unfocused tab should stay on the bar"
    );

    let prepare = |state: &GameState| {
        let mut st = state.clone();
        st.kittens = idle_here(2);
        st.kittens.extend(employed(2));
        st.pending_sky_event = Some(st.ticks + 5);
        st
    };
    let before_state = prepare(&with_sky);
    let after_state = prepare(&quiet_sky);

    let mut view = calm_view(&before_state);
    view.seen[leaving[0].index()].visited = false;
    view.seen[staying[0].index()].visited = false;
    view.reports = vec![PendingReport {
        entry: 0,
        dismissed: false,
    }];

    assert_eq!(
        badged_tab_count(&before_state, &view),
        2,
        "fixture: two tabs should wear ★"
    );
    let before = header_line(&before_state, &view, WIDE_W);
    assert_eq!(
        chip_count(&before, NEW),
        Some(2),
        "two tabs on the bar wear ★, so the chip should read ★2 new: {before:?}"
    );

    // The tab leaves with no key pressed; the chip follows the bar at once.
    let after = header_line(&after_state, &view, WIDE_W);
    assert_eq!(
        chip_count(&after, NEW),
        Some(1),
        "the badged {} tab left the bar unvisited and the chip did not fall by one. A badge no \
         longer drawn marks nothing, and the chip counts the tabs on the bar.\n before: \
         {before:?}\n  after: {after:?}",
        leaving[0].title()
    );
    for other in [IDLE, SKY, AWAY] {
        assert!(
            chip_shown(&after, other),
            "a tab leaving cleared the {} chip: {after:?}",
            chip_name(other)
        );
    }

    // And the per-frame bookkeeping that follows a tab leaving keeps it so.
    let mut kept = view.clone();
    reset_hidden_tabs(&mut kept, &after_state);
    let line = header_line(&after_state, &kept, WIDE_W);
    assert_eq!(
        chip_count(&line, NEW),
        Some(1),
        "after the frame's bookkeeping: {line:?}"
    );
}

/// `~away` clears once no report in the queue stays unread, and not before:
/// retiring banners one by one leaves it standing while any report is unread;
/// opening the Journal reads them all at once. Neither touches the other
/// chips — `★N new` keeps its count throughout.
///
/// @spec TUI-NAV-044
#[test]
fn the_away_chip_clears_only_once_no_report_stays_unread() {
    // Banners retired one at a time by Esc.
    let (state, mut view, badged) = countdown_fixture(3);
    for remaining in (0..3usize).rev() {
        update(Action::Dismiss, &mut view, &state);
        let line = header_line(&state, &view, WIDE_W);
        assert_eq!(
            chip_shown(&line, AWAY),
            remaining > 0,
            "{remaining} reports remain unread, so the away chip should {}show: {line:?}",
            if remaining > 0 { "" } else { "not " }
        );
        assert_eq!(
            chip_count(&line, NEW),
            Some(badged.len() as u64),
            "reading a report changed the new-content chip: {line:?}"
        );
        assert_eq!(
            chip_count(&line, IDLE),
            Some(4),
            "reading a report changed the idle chip: {line:?}"
        );
        assert!(
            chip_shown(&line, SKY),
            "reading a report cleared the sky chip: {line:?}"
        );
    }

    // The Journal reads every unread report at once.
    let (state, mut view, badged) = countdown_fixture(3);
    update(Action::OpenJournal, &mut view, &state);
    let line = header_line(&state, &view, WIDE_W);
    assert!(
        !chip_shown(&line, AWAY),
        "the Journal opened and the away chip stays: {line:?}"
    );
    assert_eq!(
        chip_count(&line, NEW),
        Some(badged.len() as u64),
        "the Journal changed ★N new: {line:?}"
    );
    assert_eq!(
        chip_count(&line, IDLE),
        Some(4),
        "the Journal changed the idle chip: {line:?}"
    );
    assert!(
        chip_shown(&line, SKY),
        "the Journal cleared the sky chip: {line:?}"
    );
}

/// One press can carry two chips' signals: a digit landing in a badged section
/// while a banner shows both visits the section and retires the banner. Each
/// chip answers its own signal from that press — `★N new` falls by one, and
/// `~away` clears exactly when that banner was the last unread report.
///
/// @spec TUI-NAV-044
#[test]
fn one_press_carrying_two_signals_moves_each_chip_by_its_own() {
    for unread in [1usize, 2] {
        let (state, mut view, badged) = countdown_fixture(unread);
        update(Action::SectionJump(badged[0]), &mut view, &state);
        let line = header_line(&state, &view, WIDE_W);
        assert_eq!(
            chip_count(&line, NEW),
            Some(badged.len() as u64 - 1),
            "the press visited a badged tab, so ★N new falls by one: {line:?}"
        );
        assert_eq!(
            chip_shown(&line, AWAY),
            unread > 1,
            "with {unread} reports unread before the press retired one, the away chip should {}show: \
             {line:?}",
            if unread > 1 { "" } else { "not " }
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-086 — the header's width
// ---------------------------------------------------------------------------

/// The five facts in the full wording: the generation, the date, the kitten
/// count, the mood word, the speed.
fn full_facts(s: &GameState) -> Vec<String> {
    vec![
        format!("Gen {}", s.generation),
        format!("{:?} Day {} Year {}", s.season(), s.day(), s.year()),
        format!("{} kittens", s.kittens.len()),
        mood_name(s.mood_tier()).to_string(),
        format!("×{}", s.speed),
    ]
}

/// The same five facts in the compact wording.
fn compact_facts(s: &GameState) -> Vec<String> {
    let season: String = format!("{:?}", s.season()).chars().take(3).collect();
    vec![
        format!("G{}", s.generation),
        format!("{season} D{} Y{}", s.day(), s.year()),
        format!("{} kittens", s.kittens.len()),
        mood_name(s.mood_tier()).to_string(),
        format!("×{}", s.speed),
    ]
}

/// The header TUI-NAV-086 pins at width `w` — the first step whose line fits,
/// the facts on the left, the chips flush right, at least one blank column
/// between — with the step's number; `None` below the narrowest line any step
/// can draw.
fn pinned_header(
    s: &GameState,
    live: [bool; 4],
    counts: (usize, usize),
    w: usize,
) -> Option<(usize, String)> {
    let full = full_facts(s).join(" · ");
    let compact = compact_facts(s);
    let with_counts = chip_cluster(live, Some(counts));
    let without = chip_cluster(live, None);
    let mut steps: Vec<(usize, String, String)> = vec![
        (1, full, with_counts.clone()),
        (2, compact.join(" "), with_counts),
        (3, compact.join(" "), without.clone()),
    ];
    for shed in 1..=3 {
        steps.push((4, compact[shed..].join(" "), without.clone()));
    }
    steps.into_iter().find_map(|(step, facts, chips)| {
        let fw = facts.chars().count();
        let cw = chips.chars().count();
        if chips.is_empty() {
            (fw <= w).then(|| (step, format!("{facts}{}", " ".repeat(w - fw))))
        } else {
            (fw + 1 + cw <= w).then(|| (step, format!("{facts}{}{chips}", " ".repeat(w - fw - cw))))
        }
    })
}

/// At every width from the narrowest any step can draw up past the full line,
/// the header is exactly the first step that fits: the full line; the compact
/// facts; the chips without their counts; the compact facts shed whole —
/// generation, then date, then kitten count — never the mood word or the
/// speed. No live chip is cut or dropped and no figure is cut at any width,
/// which the exact line carries. Swept for all four chips live, for none, for
/// two, and for figures long enough that the facts must shed at the floor
/// width itself.
///
/// @spec TUI-NAV-086
#[test]
fn the_header_draws_the_first_step_that_fits_at_every_width() {
    let big = {
        let (mut s, v) = live_chips_fixture([true, true, true, true], 20, 9);
        s.generation = 12_345;
        s.ticks = 1_234_567_890 * content::TICKS_PER_DAY;
        s.pending_sky_event = Some(s.ticks + 5);
        s.kittens.extend(employed(12_209));
        s.speed = 0.25;
        (s, v)
    };
    type Case = (
        &'static str,
        [bool; 4],
        (usize, usize),
        (GameState, ViewState),
    );
    let cases: Vec<Case> = vec![
        (
            "all four chips live",
            [true; 4],
            (12, 3),
            live_chips_fixture([true; 4], 12, 3),
        ),
        (
            "no chip live",
            [false; 4],
            (0, 0),
            live_chips_fixture([false; 4], 0, 0),
        ),
        (
            "idle and away live",
            [true, false, false, true],
            (7, 0),
            live_chips_fixture([true, false, false, true], 7, 0),
        ),
        ("long figures, all four live", [true; 4], (20, 9), big),
    ];

    for (what, live, counts, (state, view)) in cases {
        let widest = pinned_header(&state, live, counts, 400)
            .expect("the full line fits 400")
            .1;
        let full_w = widest.trim_end().chars().count();
        let mut steps_seen = BTreeSet::new();
        for w in 10..=(full_w + 3) {
            let Some((step, expected)) = pinned_header(&state, live, counts, w) else {
                continue;
            };
            steps_seen.insert(step);
            let drawn = header_line(&state, &view, w as u16);
            assert_eq!(
                drawn, expected,
                "{what}, at width {w}: the header should draw step {step}, the first that fits.\n\
                 expected: {expected:?}\n   drawn: {drawn:?}"
            );
        }
        let wanted: BTreeSet<usize> = if live == [false; 4] {
            [1, 2, 4].into()
        } else {
            [1, 2, 3, 4].into()
        };
        assert!(
            wanted.is_subset(&steps_seen),
            "fixture: {what} should sweep through steps {wanted:?}, swept {steps_seen:?}"
        );

        // The floor width is swept too; say so for the long figures, where the
        // shedding step is reached there.
        if what.starts_with("long") {
            let (step, _) = pinned_header(&state, live, counts, FLOOR_W as usize)
                .expect("the floor width holds mood, speed and every chip");
            assert_eq!(
                step, 4,
                "fixture: long figures should shed facts at the floor width"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// TUI-RENDER-016 — the header rule, drawn in place beneath the header
//
// The rule is the header row's bottom edge, and it is an underline rather than
// a row: every cell of the header row carries it — facts, chips, the blank
// columns between them, the padding — at every width step the header takes,
// and it changes nothing a cell already shows. The oracle for "nothing" is the
// header widget drawn alone into a row of the same width: each cell of the
// composed header row must hold that cell's symbol in that cell's colours,
// its modifiers differing by the underline and nothing else. The rule is drawn
// in place, so the frame's regions are exactly the layout algebra's.
// ---------------------------------------------------------------------------

/// Draw one whole frame, cold, and return its cells and the layout it used.
fn composed_frame(
    state: &GameState,
    view: &ViewState,
    w: u16,
    h: u16,
) -> (Buffer, clowder_tui::render::layout::Layout) {
    let mut term = terminal(w, h);
    let mut layout = None;
    term.draw(|f| layout = Some(clowder_tui::render::frame(f, state, view, None)))
        .expect("TestBackend is infallible");
    (
        term.backend().buffer().clone(),
        layout.expect("frame returns the layout it used"),
    )
}

/// Every cell of the header row wears the rule's underline — at the floor, on
/// both sides of both layout breakpoints, wide, and on both sides of every
/// change of step between the floor and wide — for every combination of live chips with one- and two-digit counts, and for figures
/// long enough to shed facts at the floor — so every width step TUI-NAV-086
/// takes inside a real frame is reached. No cell's symbol, foreground,
/// background or other modifier differs from what the header widget draws
/// there on its own, and the frame's regions are the layout algebra's own, so
/// the rule took no row and resized nothing.
///
/// @spec TUI-RENDER-016
#[test]
fn the_header_rule_underlines_every_header_cell_at_every_width_step_and_changes_nothing_else() {
    use ratatui::style::Modifier;

    let big = {
        let (mut s, v) = live_chips_fixture([true, true, true, true], 20, 9);
        s.generation = 12_345;
        s.ticks = 1_234_567_890 * content::TICKS_PER_DAY;
        s.pending_sky_event = Some(s.ticks + 5);
        s.kittens.extend(employed(12_209));
        s.speed = 0.25;
        (s, v)
    };
    type Case = ([bool; 4], (usize, usize), (GameState, ViewState));
    let mut cases: Vec<Case> = Vec::new();
    for (idle, new) in [(3usize, 2usize), (12, 3)] {
        for mask in 0u8..16 {
            let live = [mask & 1 != 0, mask & 2 != 0, mask & 4 != 0, mask & 8 != 0];
            let counts = (
                if live[0] { idle } else { 0 },
                if live[2] { new } else { 0 },
            );
            cases.push((live, counts, live_chips_fixture(live, idle, new)));
        }
    }
    cases.push(([true; 4], (20, 9), big));

    let mut steps_seen: BTreeSet<usize> = BTreeSet::new();
    for (live, counts, (state, view)) in &cases {
        let (live, counts) = (*live, *counts);
        // Every width at which the header changes step, and the width either
        // side of it, plus the floor, both layout breakpoints and wide: each
        // step is drawn at its own narrowest and widest width in the sweep.
        let step_at = |w: u16| pinned_header(state, live, counts, w as usize).map(|(s, _)| s);
        let widths: BTreeSet<u16> = (FLOOR_W..=WIDE_W)
            .filter(|&w| {
                w == FLOOR_W
                    || w == WIDE_W
                    || [99, 100, 119, 120, 150].contains(&w)
                    || step_at(w) != step_at(w - 1)
                    || step_at(w) != step_at(w + 1)
            })
            .collect();
        let banner = view.reports.iter().any(|r| !r.dismissed);

        for w in widths {
            let (step, _) = pinned_header(state, live, counts, w as usize)
                .unwrap_or_else(|| panic!("fixture: some step of the header fits {w} columns"));
            steps_seen.insert(step);

            let (buf, layout) = composed_frame(state, view, w, FLOOR_H);
            assert_eq!(
                layout,
                compute(Rect::new(0, 0, w, FLOOR_H), None, banner),
                "at {w}x{FLOOR_H} with chips {live:?} live, the frame's regions are not the \
                 layout algebra's: the header rule is drawn in place and adds no row"
            );

            let mut alone_term = terminal(w, 1);
            alone_term
                .draw(|f| {
                    let area = f.area();
                    chrome::header(f, area, state, view);
                })
                .expect("TestBackend is infallible");
            let alone = alone_term.backend().buffer();
            let line = region_lines(&buf, layout.header).concat();

            for x in 0..w {
                let got = buf.cell((x, layout.header.y)).expect("header cell");
                let want = alone.cell((x, 0)).expect("header cell drawn alone");
                assert!(
                    got.modifier.contains(Modifier::UNDERLINED),
                    "at width {w} (step {step}) with chips {live:?} live, header column {x} \
                     (`{}`) carries no underline. The header rule runs beneath every cell of \
                     the header row — facts, chips, the blanks between, the padding.\n\
                     line: {line:?}",
                    got.symbol()
                );
                assert_eq!(
                    got.symbol(),
                    want.symbol(),
                    "at width {w} (step {step}), header column {x} holds `{}` where the header \
                     draws `{}`: an underline is a mark beneath a character, never a character",
                    got.symbol(),
                    want.symbol()
                );
                assert_eq!(
                    (got.fg, got.bg),
                    (want.fg, want.bg),
                    "at width {w} (step {step}) with chips {live:?} live, header column {x} \
                     (`{}`) is drawn in {:?} on {:?}, but the header draws it in {:?} on {:?}. \
                     The rule changes no character's colour.\nline: {line:?}",
                    got.symbol(),
                    got.fg,
                    got.bg,
                    want.fg,
                    want.bg
                );
                assert_eq!(
                    got.modifier.difference(Modifier::UNDERLINED),
                    want.modifier.difference(Modifier::UNDERLINED),
                    "at width {w} (step {step}), header column {x} (`{}`) gained or lost an \
                     emphasis other than the rule's underline",
                    got.symbol()
                );
            }
        }
    }

    assert_eq!(
        steps_seen,
        BTreeSet::from([1, 2, 3, 4]),
        "fixture: the sweep should reach every width step the header takes inside a frame"
    );
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
    assert!(
        checked > 0,
        "fixture: this sweep should have read some cells"
    );

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
                let symbol = buffer
                    .cell((x, y))
                    .expect("every cell of the area exists")
                    .symbol();
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
    assert!(
        cap > 0.0,
        "fixture: catnip needs a finite cap for this test to mean anything"
    );
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
        full, half,
        "a full store and a half-full one drew the same sidebar text, so the fill is being told \
         by colour alone"
    );
}

/// Whether a sidebar line draws a fill bar.
fn draws_fill_bar(line: &str) -> bool {
    line.contains('▓') || line.contains('░')
}

/// A store wears `full` exactly where the core reports a cap above zero and
/// its stock has reached that cap — for every resource the core defines, in
/// each of the cases the rule separates: a finite cap reached, a finite cap
/// not reached, no cap at all, and a cap of zero with something in it. The
/// word stands on the sidebar line that draws that store's fill bar, and on
/// no other line; checked at the floor and wide.
///
/// @spec TUI-NAV-022
#[test]
fn a_store_wears_full_exactly_where_a_cap_above_zero_is_reached() {
    let mut classes = [0usize; 3]; // finite above zero, uncapped, zero
    for r in ResourceId::ALL {
        let mut base = GameState::new(11);
        for other in ResourceId::ALL {
            base.resources[other.ix()] = 0.0;
        }
        let cap = base.cap(r);

        let mut cases: Vec<(&str, f64, bool)> = Vec::new();
        if cap.is_finite() && cap > 0.0 {
            classes[0] += 1;
            cases.push(("at its cap", cap, true));
            cases.push(("just under its cap", cap * 0.99, false));
            cases.push(("at half its cap", cap * 0.5, false));
        } else if cap.is_infinite() {
            classes[1] += 1;
            cases.push(("uncapped, with a great deal in it", 1.0e12, false));
            cases.push(("uncapped, with a little in it", 3.0, false));
        } else {
            classes[2] += 1;
            cases.push(("with a zero cap and something in it", 5.0, false));
        }

        for (what, stock, capped) in cases {
            let mut state = base.clone();
            state.resources[r.ix()] = stock;
            for (w, h) in [(FLOOR_W, FLOOR_H), (WIDE_W, WIDE_H)] {
                let lines = sidebar_lines(&state, w, h);
                let bars: Vec<&String> = lines.iter().filter(|l| draws_fill_bar(l)).collect();
                let tagged: Vec<&String> = lines.iter().filter(|l| has_key(l, "full")).collect();
                if capped {
                    assert_eq!(
                        bars.len(),
                        1,
                        "{r:?} {what} ({w}x{h}): the sidebar should draw its one fill bar.\n{}",
                        lines.join("\n")
                    );
                    assert!(
                        has_key(bars[0], "full"),
                        "{r:?} {what} ({w}x{h}): a capped store wears `full` on the line that \
                         draws its fill bar.\n{}",
                        lines.join("\n")
                    );
                    assert_eq!(
                        tagged.len(),
                        1,
                        "{r:?} {what}: `full` on one line only.\n{}",
                        lines.join("\n")
                    );
                } else {
                    assert!(
                        tagged.is_empty(),
                        "{r:?} {what} ({w}x{h}): the store is not capped — {} — and the sidebar \
                         wears `full`.\n{}",
                        if cap.is_infinite() {
                            "the core reports no cap for it"
                        } else if cap <= 0.0 {
                            "its cap is zero"
                        } else {
                            "its stock is below its cap"
                        },
                        lines.join("\n")
                    );
                }
            }
        }
    }
    assert!(
        classes.iter().all(|n| *n > 0),
        "fixture: a new game should hold stores of every cap class (finite, uncapped, zero); \
         found {classes:?}"
    );
}

/// Affordability's colour never stands on a line without its glyph: every
/// line of a drawn frame holding a cell in the ready colour carries `●`, and
/// every line holding one in the partial colour carries `◐`. Swept over every
/// section's list in a meadow where rows of both kinds are listed.
///
/// @spec TUI-NAV-022
#[test]
fn affordability_colour_stands_only_on_a_line_carrying_its_marker() {
    use clowder_tui::theme::Role;

    let mut state = every_section_revealed();
    for r in ResourceId::ALL {
        state.resources[r.ix()] = state.cap(r).min(40.0);
    }
    for b in StructureId::ALL {
        state.structures[b.ix()] = 0;
    }
    let ready = Role::Ready.color().expect("Ready carries a colour");
    let partial = Role::Partial.color().expect("Partial carries a colour");

    let (mut saw_ready, mut saw_partial) = (false, false);
    for section in Section::ALL {
        let mut view = calm_view(&state);
        view.section = Some(section);
        let mut term = terminal(WIDE_W, WIDE_H);
        term.draw(|f| {
            clowder_tui::render::frame(f, &state, &view, None);
        })
        .expect("TestBackend is infallible");
        let buf = term.backend().buffer().clone();
        for y in 0..WIDE_H {
            let cells: Vec<_> = (0..WIDE_W)
                .map(|x| buf.cell((x, y)).expect("cell").clone())
                .collect();
            let text: String = cells.iter().map(|c| c.symbol().to_string()).collect();
            if cells.iter().any(|c| c.fg == ready) {
                saw_ready = true;
                assert!(
                    text.contains('●'),
                    "{} section, line {y}: the ready colour is drawn on a line with no `●`, so \
                     the line's affordability is told by colour alone.\nline: {text:?}",
                    section.title()
                );
            }
            if cells.iter().any(|c| c.fg == partial) {
                saw_partial = true;
                assert!(
                    text.contains('◐'),
                    "{} section, line {y}: the partial colour is drawn on a line with no `◐`.\n\
                     line: {text:?}",
                    section.title()
                );
            }
        }
    }
    assert!(
        saw_ready && saw_partial,
        "fixture: the sweep should draw both colours (ready {saw_ready}, partial {saw_partial})"
    );
}

/// Attention reaches the player by each chip's glyph and word at every width
/// the header can be drawn at, whatever the header gives up for room: for
/// every combination of live chips, each live chip keeps both its glyph and
/// its word on the header line, and no chip that is not live shows either.
///
/// @spec TUI-NAV-022
#[test]
fn every_live_chip_keeps_its_glyph_and_word_at_every_header_width() {
    for mask in 0u8..16 {
        let live = [mask & 1 != 0, mask & 2 != 0, mask & 4 != 0, mask & 8 != 0];
        let (state, view) = live_chips_fixture(live, 3, 2);
        let floor_line = chip_cluster(live, None).chars().count()
            + compact_facts(&state)[3..].join(" ").chars().count()
            + 1;
        for w in floor_line..=(WIDE_W as usize) {
            let line = header_line(&state, &view, w as u16);
            for (chip, on) in CHIPS.iter().zip(live) {
                if on {
                    assert!(
                        line.contains(chip.0) && says(&line, chip.1),
                        "{} is live and at width {w} the header does not carry both its glyph \
                         and its word: {line:?}",
                        chip_name(*chip)
                    );
                } else {
                    assert!(
                        !line.contains(chip.0),
                        "{} is not live and at width {w} the header draws its glyph: {line:?}",
                        chip_name(*chip)
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// TUI-THEME-011 — the compressed run: each state its own glyph, each glyph
// its own colour
//
// Ready, partial and attention are the three colours a red-green deficiency
// compresses toward one. For the player this is written for, the glyph is the
// whole distinction between those three states, so the chrome is held to two
// rules: attention never borrows `●` or `◐`, and none of the three glyphs is
// ever drawn in a neighbour's colour. Colours are identified by the roles the
// palette hands out, never by their values, so the rules are checked whatever
// values the palette holds.
// ---------------------------------------------------------------------------

/// Attention's non-colour channels — each chip's glyph, and the word `full`
/// on a capped store — are never `●` or `◐`: for every combination of live
/// chips at every width the header can be drawn at, no header line carries
/// either marker and every live chip's own glyph is neither; and every
/// sidebar line that wears `full`, for every store the core reports a cap
/// for, carries neither marker.
///
/// @spec TUI-THEME-011
#[test]
fn no_attention_channel_is_ever_the_affordable_or_partial_marker() {
    const MARKERS: [char; 2] = ['●', '◐'];

    for chip in CHIPS {
        assert!(
            !MARKERS.contains(&chip.0),
            "{} uses an affordability marker as its glyph; once the hues compress it is the \
             same appearance as that marker",
            chip_name(chip)
        );
    }

    for mask in 0u8..16 {
        let live = [mask & 1 != 0, mask & 2 != 0, mask & 4 != 0, mask & 8 != 0];
        let (state, view) = live_chips_fixture(live, 3, 2);
        let floor_line = chip_cluster(live, None).chars().count()
            + compact_facts(&state)[3..].join(" ").chars().count()
            + 1;
        for w in floor_line..=(WIDE_W as usize) {
            let line = header_line(&state, &view, w as u16);
            for m in MARKERS {
                assert!(
                    !line.contains(m),
                    "with chips {live:?} live, at width {w} the header draws `{m}` — an \
                     affordability marker on the line that carries attention: {line:?}"
                );
            }
            for (chip, on) in CHIPS.iter().zip(live) {
                if !on {
                    continue;
                }
                let at = line
                    .rfind(chip.1)
                    .unwrap_or_else(|| panic!("fixture: {} is live and drawn", chip_name(*chip)));
                let glyph = line[..at]
                    .chars()
                    .rev()
                    .find(|c| !c.is_ascii_digit() && !c.is_whitespace())
                    .unwrap_or(' ');
                assert!(
                    !MARKERS.contains(&glyph),
                    "at width {w}, the live {} chip is drawn with `{glyph}` as its glyph: {line:?}",
                    chip.1
                );
            }
        }
    }

    let mut capped = GameState::new(11);
    let mut finite = 0;
    for r in ResourceId::ALL {
        let cap = capped.cap(r);
        if cap.is_finite() && cap > 0.0 {
            capped.resources[r.ix()] = cap;
            finite += 1;
        }
    }
    assert!(finite > 0, "fixture: a new game holds stores with a cap");
    for (w, h) in [(FLOOR_W, FLOOR_H), (WIDE_W, WIDE_H)] {
        let lines = sidebar_lines(&capped, w, h);
        let tagged: Vec<&String> = lines.iter().filter(|l| has_key(l, "full")).collect();
        assert!(
            !tagged.is_empty(),
            "fixture: at {w}x{h} some capped store wears `full`"
        );
        for line in tagged {
            for m in MARKERS {
                assert!(
                    !line.contains(m),
                    "at {w}x{h}, a capped store's line carries `{m}` beside its `full` tag: {line:?}"
                );
            }
        }
    }
}

/// A meadow where the list draws `●` and `◐` rows, a Teased row wears `◐` in
/// dim, a store is capped, and all four attention chips are live — so every
/// glyph of the compressed run is on screen at once.
fn compressed_run_meadows() -> Vec<(&'static str, GameState, ViewState)> {
    let mut out = Vec::new();

    // Rows of both affordability kinds, every chip live, some store capped.
    let mut mixed = every_section_revealed();
    for r in ResourceId::ALL {
        mixed.resources[r.ix()] = mixed.cap(r).min(40.0);
    }
    for b in StructureId::ALL {
        mixed.structures[b.ix()] = 0;
    }
    let capped = ResourceId::ALL
        .into_iter()
        .find(|r| mixed.cap(*r).is_finite() && mixed.cap(*r) > 0.0)
        .expect("fixture: some store carries a cap");
    mixed.resources[capped.ix()] = mixed.cap(capped);
    let mut roster = idle_here(4);
    roster.extend(employed(3));
    mixed.kittens = roster;
    mixed.pending_sky_event = Some(mixed.ticks + 5);
    let mut view = calm_view(&mixed);
    view.seen[Section::Clowder.index()].visited = false;
    view.reports = vec![PendingReport {
        entry: 0,
        dismissed: false,
    }];
    out.push(("a mixed meadow with every chip live", mixed, view));

    // The rich meadow, every chip live: mostly `●` and `✓`.
    let (rich, view) = all_four_live();
    out.push(("a rich meadow with every chip live", rich, view));

    // A revealed structure half paid for: Teased, and wearing `◐` in dim.
    let teased = StructureId::ALL
        .into_iter()
        .find_map(|s| {
            let id = ContentId::Structure(s);
            let mut state = GameState::new(17);
            state.revealed.insert(id);
            let cost = state.cost_of_next(id, 1);
            if cost.is_empty() {
                return None;
            }
            for (r, amount) in cost {
                state.resources[r.ix()] = amount * 0.5;
            }
            let covered = state.affordability(id);
            (state.availability(id) == clowder_core::Availability::Teased
                && matches!(state.lock_reason(id), Some(LockReason::ShortBy { .. }))
                && covered > 0.0
                && covered < 1.0)
                .then_some(state)
        })
        .expect("fixture: some structure can be revealed, short, and partly covered");
    let view = calm_view(&teased);
    out.push(("a meadow with a Teased row half paid for", teased, view));

    out
}

/// Among `ready`, `partial` and `attention`, each glyph of the compressed run
/// is drawn only in its own state's colour: `●` never in partial or
/// attention, `◐` never in ready or attention, and an attention chip's glyph
/// never in ready or partial. Swept over every cell of whole frames — every
/// section with the inspector asked for and not, a footer note showing and
/// not, and each full-screen overlay, at the floor, in the narrow range, and
/// past both breakpoints; and the cursor on every row with the list and the
/// inspector pane both on screen — in meadows where every one of those glyphs
/// is drawn.
///
/// @spec TUI-THEME-011
#[test]
fn among_the_compressed_run_each_glyph_wears_only_its_own_colour() {
    use clowder_tui::theme::Role;

    let ready = Role::Ready.color().expect("Ready carries a colour");
    let partial = Role::Partial.color().expect("Partial carries a colour");
    let attention = Role::Attention.color().expect("Attention carries a colour");
    let colour_name = |c: ratatui::style::Color| {
        if c == ready {
            "ready"
        } else if c == partial {
            "partial"
        } else if c == attention {
            "attention"
        } else {
            "another colour"
        }
    };

    let sizes: [(u16, u16); 4] = [(FLOOR_W, FLOOR_H), (99, 30), (120, 40), (WIDE_W, WIDE_H)];
    let (mut saw_filled, mut saw_half, mut saw_chip) = (0usize, 0usize, 0usize);

    for (label, state, base) in compressed_run_meadows() {
        // Each view with the sizes it is drawn at. The cursor is walked over
        // every row at one size where the list and the inspector pane are both
        // on screen; every other view is drawn at every size.
        let pane: (u16, u16) = (120, 40);
        type ChromeView = (String, ViewState, Vec<(u16, u16)>);
        let mut views: Vec<ChromeView> = Vec::new();
        for section in Section::ALL {
            for key in rows(&state, section)
                .into_iter()
                .filter(|r| r.selectable())
                .map(|r| r.key)
            {
                let mut v = base.clone();
                v.section = Some(section);
                v.focus[section.index()] = Some(key.clone());
                unfold_focus(&state, &mut v);
                v.inspector_wanted = true;
                views.push((
                    format!("{} section, cursor on {key:?}", section.title()),
                    v,
                    vec![pane],
                ));
            }
            for inspector in [false, true] {
                let mut v = base.clone();
                v.section = Some(section);
                v.inspector_wanted = inspector;
                views.push((
                    format!("{} section, inspector {inspector}", section.title()),
                    v,
                    sizes.to_vec(),
                ));
            }
            let mut v = base.clone();
            v.section = Some(section);
            v.footer = Some((
                "a footer note".to_string(),
                Instant::now() + Duration::from_secs(600),
            ));
            views.push((
                format!("{} section with a footer note", section.title()),
                v,
                sizes.to_vec(),
            ));
        }
        for overlay in [
            Overlay::Help,
            Overlay::Journal { anchor: 0 },
            Overlay::Palette {
                query: String::new(),
                selected: 0,
            },
        ] {
            let v = ViewState {
                overlay: Some(overlay.clone()),
                ..base.clone()
            };
            views.push((format!("overlay {overlay:?}"), v, sizes.to_vec()));
        }

        for (what, view, at) in &views {
            for &(w, h) in at {
                let mut term = terminal(w, h);
                let mut layout = None;
                term.draw(|f| layout = Some(clowder_tui::render::frame(f, &state, view, None)))
                    .expect("TestBackend is infallible");
                let header = layout.expect("frame returns its layout").header;
                let buf = term.backend().buffer();
                for y in 0..h {
                    for x in 0..w {
                        let cell = buf.cell((x, y)).expect("cell");
                        let (forbidden, glyph): (&[ratatui::style::Color], &str) =
                            match cell.symbol() {
                                "●" => {
                                    saw_filled += 1;
                                    (&[partial, attention], "●")
                                }
                                "◐" => {
                                    saw_half += 1;
                                    (&[ready, attention], "◐")
                                }
                                s if header.contains(ratatui::layout::Position { x, y })
                                    && CHIPS.iter().any(|c| s.starts_with(c.0)) =>
                                {
                                    saw_chip += 1;
                                    (&[ready, partial], "an attention chip's glyph")
                                }
                                _ => continue,
                            };
                        assert!(
                            !forbidden.contains(&cell.fg),
                            "in {label}, {what}, at {w}x{h}: {glyph} at ({x}, {y}) is drawn in \
                             the {} colour. Among ready, partial and attention each glyph wears \
                             only its own state's colour, or two states become one appearance \
                             once the hues compress.\nline: {:?}",
                            colour_name(cell.fg),
                            (0..w)
                                .map(|x| buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "))
                                .collect::<String>()
                        );
                    }
                }
            }
        }
    }

    assert!(
        saw_filled > 0 && saw_half > 0 && saw_chip > 0,
        "fixture: the sweep should draw every glyph of the run (● {saw_filled}, ◐ {saw_half}, \
         chip glyphs {saw_chip})"
    );
}

// ---------------------------------------------------------------------------
// TUI-THEME-015 — what wears `attention`, and nothing else
//
// `attention` means one thing — this wants you — and the glance contract rests
// on that: an empty chip cluster and no `full` tag reads as calm only if orange
// appearing is always that signal. So each chip is drawn whole in it (glyph,
// count where it has one, word) at every width step the header takes, the
// capped store's `full` tag alone wears it, and nothing that is merely
// information borrows it — a footer note included. The armed row's expansion,
// the third element, is checked with the armed row in `armed_summary.rs`.
// Every assertion compares against the one value `Role::Attention` hands out,
// so three surfaces agreeing with it is one orange to learn.
// ---------------------------------------------------------------------------

fn attention_colour() -> ratatui::style::Color {
    clowder_tui::theme::Role::Attention
        .color()
        .expect("Attention carries a colour")
}

/// The chips drawn on one line of cells, each as the columns it spans and the
/// word it is: a chip's glyph, then any digits, then — after at most one
/// blank — its word. A glyph not followed by its word is not a chip and is
/// not returned.
fn chip_spans(symbols: &[String]) -> Vec<(std::ops::Range<usize>, &'static str)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < symbols.len() {
        let Some(chip) = CHIPS.iter().find(|c| symbols[i].starts_with(c.0)) else {
            i += 1;
            continue;
        };
        let mut j = i + 1;
        let mut counted = false;
        while j < symbols.len()
            && symbols[j].chars().all(|c| c.is_ascii_digit())
            && !symbols[j].is_empty()
        {
            j += 1;
            counted = true;
        }
        if counted && j < symbols.len() && symbols[j] == " " {
            j += 1;
        }
        let word_len = chip.1.chars().count();
        let word: String = symbols
            .get(j..j + word_len)
            .map(|s| s.concat())
            .unwrap_or_default();
        if word == chip.1 {
            out.push((i..j + word_len, chip.1));
            i = j + word_len;
        } else {
            i += 1;
        }
    }
    out
}

/// The columns of every whole-token `word` on a line of cells.
fn word_spans(symbols: &[String], word: &str) -> Vec<std::ops::Range<usize>> {
    let len = word.chars().count();
    (0..symbols.len().saturating_sub(len - 1))
        .filter(|&i| {
            symbols[i..i + len].concat() == word
                && (i == 0 || symbols[i - 1].trim().is_empty())
                && symbols.get(i + len).is_none_or(|s| s.trim().is_empty())
        })
        .map(|i| i..i + len)
        .collect()
}

/// A leading fragment of `word` sitting at the very last column of `symbols`,
/// preceded by whitespace: what a whole-word span becomes once the sidebar's
/// fixed width (TUI-NAV-022's surface, not this spec's) clips the row's last
/// character off. Recognized so this test's job — nothing wears `attention`
/// but a chip or the `full` tag — stays about colour even on the row where
/// width already cuts the tag's last letter; whether the tag fits whole is a
/// layout question this spec does not decide.
fn clipped_word_span(symbols: &[String], word: &str) -> Option<std::ops::Range<usize>> {
    let word: Vec<char> = word.chars().collect();
    (1..word.len()).rev().find_map(|k| {
        let start = symbols.len().checked_sub(k)?;
        let fragment: String = word[..k].iter().collect();
        (symbols[start..].concat() == fragment
            && (start == 0 || symbols[start - 1].trim().is_empty()))
        .then_some(start..symbols.len())
    })
}

/// One row of a buffer as its cells, clipped to `rect`'s columns.
fn row_cells(buf: &Buffer, rect: Rect, y: u16) -> Vec<ratatui::buffer::Cell> {
    (rect.x..rect.right())
        .map(|x| buf.cell((x, y)).expect("cell").clone())
        .collect()
}

fn symbols_of(cells: &[ratatui::buffer::Cell]) -> Vec<String> {
    cells.iter().map(|c| c.symbol().to_string()).collect()
}

/// Each attention chip is drawn whole in the one `attention` value — its
/// glyph, its count where it has one, and its word — and nothing else on the
/// header line is: not the blank columns between chips, not the facts. Swept
/// for every combination of live chips, at every width from the narrowest any
/// step can draw to wide, so every step TUI-NAV-086 takes is reached, the
/// counted and the uncounted cluster alike; and for counts of one and of two
/// digits. The `★N new` chip is held to this like any other.
///
/// @spec TUI-THEME-015
#[test]
fn every_attention_chip_is_drawn_whole_in_attention_and_nothing_else_on_the_header_is() {
    let attention = attention_colour();
    let mut forms_seen: BTreeSet<(&str, bool)> = BTreeSet::new();

    for (idle, new) in [(3usize, 2usize), (12, 3)] {
        for mask in 0u8..16 {
            let live = [mask & 1 != 0, mask & 2 != 0, mask & 4 != 0, mask & 8 != 0];
            let (state, view) = live_chips_fixture(live, idle, new);
            let floor_line = chip_cluster(live, None).chars().count()
                + compact_facts(&state)[3..].join(" ").chars().count()
                + 1;
            for w in floor_line..=(WIDE_W as usize) {
                let mut term = terminal(w as u16, 1);
                term.draw(|f| {
                    let area = f.area();
                    chrome::header(f, area, &state, &view);
                })
                .expect("TestBackend is infallible");
                let buf = term.backend().buffer();
                let cells = row_cells(buf, Rect::new(0, 0, w as u16, 1), 0);
                let symbols = symbols_of(&cells);
                let line = symbols.concat();

                let spans = chip_spans(&symbols);
                let drawn: Vec<&str> = spans.iter().map(|(_, word)| *word).collect();
                let wanted: Vec<&str> = CHIPS
                    .iter()
                    .zip(live)
                    .filter(|(_, on)| *on)
                    .map(|(c, _)| c.1)
                    .collect();
                assert_eq!(
                    drawn, wanted,
                    "fixture: at width {w} with chips {live:?} live, the header's chips should \
                     read as {wanted:?}: {line:?}"
                );

                let mut in_chip = vec![false; cells.len()];
                for (span, word) in &spans {
                    let counted = symbols[span.clone()]
                        .iter()
                        .any(|s| s.chars().all(|c| c.is_ascii_digit()));
                    forms_seen.insert((word, counted));
                    for x in span.clone() {
                        in_chip[x] = true;
                        assert_eq!(
                            cells[x].fg,
                            attention,
                            "at width {w}, the `{word}` chip's `{}` at column {x} is drawn in \
                             {:?}, not attention. A chip wears attention whole — glyph, count, \
                             and word — at every width step.\nline: {line:?}",
                            cells[x].symbol(),
                            cells[x].fg
                        );
                    }
                }
                for (x, cell) in cells.iter().enumerate() {
                    if in_chip[x] {
                        continue;
                    }
                    assert_ne!(
                        cell.fg,
                        attention,
                        "at width {w} with chips {live:?} live, `{}` at column {x} is outside \
                         every chip and drawn in attention — the blank columns between chips and \
                         the header's facts are not something that wants the player.\n\
                         line: {line:?}",
                        cell.symbol()
                    );
                }
            }
        }
    }

    for form in [
        ("idle", true),
        ("idle", false),
        ("new", true),
        ("new", false),
        ("sky", false),
        ("away", false),
    ] {
        assert!(
            forms_seen.contains(&form),
            "fixture: the sweep should draw the `{}` chip {} its count (saw {forms_seen:?})",
            form.0,
            if form.1 { "with" } else { "without" }
        );
    }
}

/// A capped store wears `attention` on its `full` tag and nowhere else: the
/// tag's cells are drawn in the one attention value, and the store's name,
/// figures, fill bar and rate — and every other store's lines — are not. A
/// store not capped wears it nowhere, the one just under its cap whose bar
/// may already read full included. For every store the core defines, in each
/// case the capping rule separates, at the sidebar's one fixed width.
///
/// A crew works every job in the fixture, so a capped store is checked while
/// its rate is actually moving — the ordinary case in play, since a store
/// that stays capped while nothing produces or consumes it is the unusual
/// one — and the sweep is required to reach a rate compact enough to still
/// read as a whole number of columns yet wide enough to spend all four of
/// `fmt::compact`'s significant figures, and a stock of three integer digits,
/// so the case that most crowds the line is on record. Whether the figures
/// ever run long enough to cost the tag its own last column is the sidebar's
/// fixed width (TUI-NAV-022's surface), not this spec's; whatever the row
/// draws, the tag's cells and only the tag's cells wear `attention`.
///
/// @spec TUI-THEME-015
#[test]
fn a_capped_store_wears_attention_on_its_full_tag_and_nowhere_else() {
    let attention = attention_colour();
    let mut tagged_stores = 0usize;
    let mut bar_full_below_cap = false;
    let mut saw_nonzero_rate_capped = false;
    let mut saw_three_digit_stock_capped = false;
    let mut saw_four_sig_fig_rate_capped = false;

    // Crew sizes tried per resource, so at least one store in the sweep is
    // capped with a rate large enough to spend all four of `fmt::compact`'s
    // significant figures rather than every crew leaving it at rest.
    const CREW_SIZES: [usize; 8] = [6, 20, 50, 100, 167, 200, 300, 500];

    for r in ResourceId::ALL {
        // Pick whichever crew size makes this resource's own rate reach the
        // most significant figures once capped, so the regression case —
        // three-digit stocks and four-significant-figure rates together — is
        // on record wherever the core's own numbers allow it.
        let crew = CREW_SIZES
            .iter()
            .copied()
            .max_by_key(|&n| {
                let mut probe = GameState::new(11);
                for job in JobId::ALL {
                    for _ in 0..n {
                        probe.kittens.push(kitten(Some(job), Location::Home));
                    }
                }
                let rate = probe.net_rate(r);
                let sig_figs = fmt::rate(rate)
                    .trim_start_matches(['+', '-'])
                    .chars()
                    .filter(char::is_ascii_digit)
                    .count();
                (sig_figs, rate != 0.0)
            })
            .unwrap_or(CREW_SIZES[0]);

        let mut base = GameState::new(11);
        // A crew working every job, so a capped store is checked with real
        // production and consumption running rather than only at rest.
        for job in JobId::ALL {
            for _ in 0..crew {
                base.kittens.push(kitten(Some(job), Location::Home));
            }
        }
        let cap = base.cap(r);
        // Every other store stays at zero and so off the sidebar entirely —
        // the store under test is the only line drawn, which keeps it inside
        // the sidebar's fixed height (46 stores at two lines apiece would not
        // otherwise all fit) while still letting "no other span wears
        // attention" be checked against every cell the sidebar draws.

        let mut cases: Vec<(&str, f64, bool)> = Vec::new();
        if cap.is_finite() && cap > 0.0 {
            cases.push(("at its cap", cap, true));
            cases.push(("just under its cap", cap * 0.99, false));
            cases.push(("at half its cap", cap * 0.5, false));
        } else if cap.is_infinite() {
            cases.push(("uncapped, with a great deal in it", 1.0e12, false));
        } else {
            cases.push(("with a zero cap and something in it", 5.0, false));
        }

        for (what, stock, capped) in cases {
            let mut state = base.clone();
            state.resources[r.ix()] = stock;
            let rate = state.net_rate(r);
            if capped {
                if rate != 0.0 {
                    saw_nonzero_rate_capped = true;
                }
                let compact_stock = fmt::compact(stock);
                let integer_digits = compact_stock
                    .trim_start_matches('-')
                    .split(['.', 'K', 'M', 'B', 'T', 'Q'])
                    .next()
                    .unwrap_or("")
                    .len();
                if integer_digits >= 3 {
                    saw_three_digit_stock_capped = true;
                }
                let compact_rate = fmt::rate(rate);
                let sig_figs = compact_rate
                    .trim_start_matches(['+', '-'])
                    .chars()
                    .filter(char::is_ascii_digit)
                    .count();
                if sig_figs >= 4 {
                    saw_four_sig_fig_rate_capped = true;
                }
            }
            for (w, h) in [(FLOOR_W, FLOOR_H), (WIDE_W, WIDE_H)] {
                let (_, _, side, _) = regions(w, h);
                let mut term = terminal(w, h);
                term.draw(|f| chrome::sidebar(f, side, &state))
                    .expect("TestBackend is infallible");
                let buf = term.backend().buffer();
                let lines = region_lines(buf, side);

                let mut tags = 0usize;
                for y in side.y..side.bottom() {
                    let cells = row_cells(buf, side, y);
                    let symbols = symbols_of(&cells);
                    let line = symbols.concat();
                    if !capped && line.contains("▓▓▓▓") {
                        bar_full_below_cap = true;
                    }
                    let mut on_tag = vec![false; cells.len()];
                    for span in word_spans(&symbols, "full") {
                        tags += 1;
                        for x in span {
                            on_tag[x] = true;
                            assert_eq!(
                                cells[x].fg,
                                attention,
                                "{r:?} {what} ({w}x{h}): the `full` tag's `{}` is drawn in {:?}, \
                                 not attention.\nsidebar:\n{}",
                                cells[x].symbol(),
                                cells[x].fg,
                                lines.join("\n")
                            );
                        }
                    }
                    for (x, cell) in cells.iter().enumerate() {
                        if on_tag[x] {
                            continue;
                        }
                        assert_ne!(
                            cell.fg,
                            attention,
                            "{r:?} {what} ({w}x{h}): `{}` at ({}, {y}) is drawn in attention. \
                             Only the `full` tag wears it — never a store's name, figures, fill \
                             bar or rate.\nline: {line:?}",
                            cell.symbol(),
                            side.x as usize + x
                        );
                    }
                }
                assert_eq!(
                    tags,
                    usize::from(capped),
                    "fixture: {r:?} {what} ({w}x{h}) should draw {} `full` tag(s).\nsidebar:\n{}",
                    usize::from(capped),
                    lines.join("\n")
                );
                tagged_stores += usize::from(capped);
            }
        }
    }
    assert!(
        tagged_stores > 0,
        "fixture: some store should be drawn capped"
    );
    assert!(
        bar_full_below_cap,
        "fixture: some store just under its cap should draw a bar that already reads full"
    );
    assert!(
        saw_nonzero_rate_capped,
        "fixture: some capped store should draw with a nonzero net rate"
    );
    assert!(
        saw_three_digit_stock_capped,
        "fixture: some capped store's stock should compact to three integer digits"
    );
    assert!(
        saw_four_sig_fig_rate_capped,
        "fixture: some capped store's rate should compact to four significant figures"
    );
}

/// A footer note — the reply to a key just pressed — is drawn in the
/// inherited foreground on every cell of its row, never in attention nor in
/// any other palette colour: the note is where the player is already looking
/// and lasts seconds, and in orange it would be one more mark the glance has
/// to rule out. Checked in every section, with the inspector asked for and
/// not, at the floor, in the narrow range, and past both breakpoints, for a
/// short note and one long enough to be clipped.
///
/// @spec TUI-THEME-015
#[test]
fn a_footer_note_is_drawn_in_the_inherited_foreground() {
    let state = every_section_revealed();
    let notes = [
        "not yet".to_string(),
        "a reply long enough that it runs past the edge of even the widest pane this sweep \
         draws, so the clipped note is checked as well as the whole one, every column of it"
            .to_string(),
    ];
    for section in Section::ALL {
        for inspector in [false, true] {
            for note in &notes {
                for (w, h) in [(FLOOR_W, FLOOR_H), (99, 30), (120, 40), (WIDE_W, WIDE_H)] {
                    let mut view = calm_view(&state);
                    view.section = Some(section);
                    view.inspector_wanted = inspector;
                    view.footer = Some((note.clone(), Instant::now() + Duration::from_secs(600)));
                    let mut term = terminal(w, h);
                    let mut layout = None;
                    term.draw(|f| {
                        layout = Some(clowder_tui::render::frame(f, &state, &view, None))
                    })
                    .expect("TestBackend is infallible");
                    let hints = layout.expect("frame returns its layout").hints;
                    let buf = term.backend().buffer();
                    let cells = row_cells(buf, hints, hints.y);
                    let line = symbols_of(&cells).concat();
                    let head: String = note.chars().take(7).collect();
                    assert!(
                        line.starts_with(&head),
                        "fixture: the footer note should hold the hint bar's row: {line:?}"
                    );
                    for (x, cell) in cells.iter().enumerate() {
                        assert_eq!(
                            cell.fg,
                            ratatui::style::Color::Reset,
                            "{} section, inspector {inspector}, {w}x{h}: the footer note's `{}` at \
                             column {x} is drawn in {:?}. A footer note is ordinary prose in the \
                             inherited foreground — it does not want the player, so it may not \
                             borrow attention.\nline: {line:?}",
                            section.title(),
                            cell.symbol(),
                            cell.fg
                        );
                    }
                }
            }
        }
    }
}

/// A meadow with nothing wanting the player — no chip live, no store capped,
/// nothing armed — so that nothing on screen may wear attention at all.
fn calm_meadow() -> (GameState, ViewState) {
    let mut state = every_section_revealed();
    for r in ResourceId::ALL {
        let cap = state.cap(r);
        state.resources[r.ix()] = if cap.is_finite() && cap > 0.0 {
            cap * 0.5
        } else {
            40.0
        };
    }
    state.kittens = employed(6);
    state.pending_sky_event = None;
    let view = calm_view(&state);
    (state, view)
}

/// Nothing wears `attention` but a chip and a capped store's `full` tag: in
/// every cell of whole frames with nothing armed, a cell is in the attention
/// value exactly when it belongs to a chip on the header line or to a `full`
/// tag on the sidebar. Swept over a calm meadow, where nothing on screen may
/// wear it at all, and over meadows where every chip is live and stores are
/// capped — every section, with the inspector asked for and not, a footer note
/// showing and not, and each full-screen overlay, at the floor, in the narrow
/// range and past both breakpoints; and the cursor on every row with the list
/// and the inspector pane both on screen.
///
/// The sidebar's fixed width is TUI-NAV-022's surface, not this spec's: on
/// the rare row where the figures already reach the last column, the `full`
/// tag's own trailing letter is what the width clips, and a fragment of the
/// tag there still counts as the tag for this test's purpose — this spec
/// only decides which cells wear the colour, never whether the tag fits
/// whole.
///
/// @spec TUI-THEME-015
#[test]
fn nothing_but_a_chip_or_a_full_tag_wears_attention_in_a_frame_with_nothing_armed() {
    let attention = attention_colour();
    let sizes: [(u16, u16); 4] = [(FLOOR_W, FLOOR_H), (99, 30), (120, 40), (WIDE_W, WIDE_H)];

    let mut meadows: Vec<(&'static str, GameState, ViewState)> = Vec::new();
    let (calm, calm_v) = calm_meadow();
    meadows.push(("a calm meadow", calm, calm_v));
    meadows.extend(compressed_run_meadows());

    let (mut saw_chip_cells, mut saw_tag_cells, mut calm_frames) = (0usize, 0usize, 0usize);
    for (label, state, base) in &meadows {
        let pane: (u16, u16) = (120, 40);
        type FrameView = (String, ViewState, Vec<(u16, u16)>);
        let mut views: Vec<FrameView> = Vec::new();
        for section in Section::ALL {
            for key in rows(state, section)
                .into_iter()
                .filter(|r| r.selectable())
                .map(|r| r.key)
            {
                let mut v = base.clone();
                v.section = Some(section);
                v.focus[section.index()] = Some(key.clone());
                unfold_focus(state, &mut v);
                v.inspector_wanted = true;
                views.push((
                    format!("{} section, cursor on {key:?}", section.title()),
                    v,
                    vec![pane],
                ));
            }
            for inspector in [false, true] {
                for footer in [false, true] {
                    let mut v = base.clone();
                    v.section = Some(section);
                    v.inspector_wanted = inspector;
                    if footer {
                        v.footer = Some((
                            "a footer note".to_string(),
                            Instant::now() + Duration::from_secs(600),
                        ));
                    }
                    views.push((
                        format!(
                            "{} section, inspector {inspector}, footer {footer}",
                            section.title()
                        ),
                        v,
                        sizes.to_vec(),
                    ));
                }
            }
        }
        for overlay in [
            Overlay::Help,
            Overlay::Journal { anchor: 0 },
            Overlay::Palette {
                query: String::new(),
                selected: 0,
            },
        ] {
            let v = ViewState {
                overlay: Some(overlay.clone()),
                ..base.clone()
            };
            views.push((format!("overlay {overlay:?}"), v, sizes.to_vec()));
        }

        for (what, view, at) in &views {
            for &(w, h) in at {
                let mut term = terminal(w, h);
                let mut layout = None;
                term.draw(|f| layout = Some(clowder_tui::render::frame(f, state, view, None)))
                    .expect("TestBackend is infallible");
                let layout = layout.expect("frame returns its layout");
                let buf = term.backend().buffer();
                let whole = Rect::new(0, 0, w, h);
                let sidebar = match (&view.overlay, layout.body) {
                    (None, Body::Narrow { sidebar, .. } | Body::Wide { sidebar, .. }) => {
                        Some(sidebar)
                    }
                    _ => None,
                };

                let mut marks = 0usize;
                for y in 0..h {
                    let cells = row_cells(buf, whole, y);
                    let symbols = symbols_of(&cells);
                    let line = symbols.concat();
                    let mut allowed = vec![false; cells.len()];
                    if y == layout.header.y {
                        for (span, _) in chip_spans(&symbols) {
                            saw_chip_cells += span.len();
                            marks += 1;
                            span.for_each(|x| allowed[x] = true);
                        }
                    }
                    if let Some(side) = sidebar.filter(|s| y >= s.y && y < s.bottom()) {
                        let side_symbols = symbols_of(&row_cells(buf, side, y));
                        let whole_spans = word_spans(&side_symbols, "full");
                        let spans: Vec<std::ops::Range<usize>> = if whole_spans.is_empty() {
                            clipped_word_span(&side_symbols, "full")
                                .into_iter()
                                .collect()
                        } else {
                            whole_spans
                        };
                        for span in spans {
                            saw_tag_cells += span.len();
                            marks += 1;
                            for x in span {
                                allowed[side.x as usize + x] = true;
                            }
                        }
                    }
                    for (x, cell) in cells.iter().enumerate() {
                        if allowed[x] {
                            assert_eq!(
                                cell.fg,
                                attention,
                                "in {label}, {what}, at {w}x{h}: `{}` at ({x}, {y}) belongs to a \
                                 chip or a `full` tag and is drawn in {:?}, not attention.\n\
                                 line: {line:?}",
                                cell.symbol(),
                                cell.fg
                            );
                        } else {
                            assert_ne!(
                                cell.fg,
                                attention,
                                "in {label}, {what}, at {w}x{h}: `{}` at ({x}, {y}) is drawn in \
                                 attention and is neither a chip nor a capped store's `full` \
                                 tag. Nothing else wears attention, or an empty cluster stops \
                                 meaning nothing needs you.\nline: {line:?}",
                                cell.symbol()
                            );
                        }
                    }
                }
                if *label == "a calm meadow" {
                    assert_eq!(
                        marks, 0,
                        "fixture: {what} at {w}x{h} in the calm meadow should draw no chip and no \
                         `full` tag"
                    );
                    calm_frames += 1;
                }
            }
        }
    }
    assert!(
        saw_chip_cells > 0 && saw_tag_cells > 0 && calm_frames > 0,
        "fixture: the sweep should draw chips ({saw_chip_cells} cells), `full` tags \
         ({saw_tag_cells} cells) and calm frames ({calm_frames})"
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-006 — which sections have a tab, and where
// ---------------------------------------------------------------------------

/// The digits the tab bar draws, in the order drawn. Section names hold no
/// digits, so every digit on the bar is a tab.
fn tab_digits(state: &GameState, view: &ViewState) -> Vec<char> {
    tab_line(state, view, WIDE_W)
        .chars()
        .filter(char::is_ascii_digit)
        .collect()
}

/// Meadows spanning the reveal: a new game, where every section but home
/// lists nothing; a young meadow with a clowder to organise; one far along,
/// where every section lists something; and one far along with the sky's
/// rows gated shut.
fn reveal_corpus() -> Vec<(&'static str, GameState)> {
    let new_game = GameState::new(3);

    let mut young = GameState::new(3);
    young.kittens = employed(2);

    let far = every_section_revealed();

    let mut quiet_sky = every_section_revealed();
    quiet_sky.ideas.remove(&IdeaId::SkywardLonging);
    quiet_sky.ideas.remove(&IdeaId::SkywardSong);

    vec![
        ("a new game", new_game),
        ("a young meadow", young),
        ("a meadow far along", far),
        ("a meadow far along with a quiet sky", quiet_sky),
    ]
}

/// A section has its tab exactly while it lists at least one selectable row —
/// a group header never counting — or is the section in focus, re-derived
/// from the state and focus handed in; the bar lists the tabs in digit order,
/// 1 through 9 and then 0, each at its own digit. Asserted for every section,
/// with every section in focus in turn, across meadows where most sections
/// list nothing and where every one does.
///
/// @spec TUI-NAV-006
#[test]
fn a_section_has_its_tab_exactly_while_it_lists_a_selectable_row_or_holds_focus() {
    let mut focus_kept_an_empty_tab = false;
    let mut an_unfocused_section_had_none = false;

    for (what, state) in reveal_corpus() {
        for focus in Section::ALL {
            let mut view = calm_view(&state);
            view.section = Some(focus);

            let expected: Vec<char> = Section::ALL
                .into_iter()
                .filter(|s| {
                    let lists = rows(&state, *s).iter().any(|r| r.selectable());
                    if !lists && *s == focus {
                        focus_kept_an_empty_tab = true;
                    }
                    if !lists && *s != focus {
                        an_unfocused_section_had_none = true;
                    }
                    lists || *s == focus
                })
                .map(Section::digit)
                .collect();

            let drawn = tab_digits(&state, &view);
            assert_eq!(
                drawn,
                expected,
                "in {what} with the {} section in focus, the tab bar draws the tabs {drawn:?}; a \
                 section has its tab while it lists a selectable row or holds focus, in digit \
                 order 1–9 then 0.\ntab bar: {:?}",
                focus.title(),
                tab_line(&state, &view, WIDE_W)
            );
        }
    }

    assert!(
        focus_kept_an_empty_tab,
        "fixture: some case focuses a section that lists nothing"
    );
    assert!(
        an_unfocused_section_had_none,
        "fixture: some case has a section with no tab"
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
    assert_eq!(
        clashes, 0,
        "fixture: {head:?} does not identify one section"
    );
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
    view.seen[badged.index()] = SeenMark {
        visited: false,
        revealed_at_last_visit: 0,
    };

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

    let named = Section::ALL
        .into_iter()
        .filter(|s| line.contains(s.title()))
        .count();
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
    view.seen[Section::Dreams.index()] = SeenMark {
        visited: false,
        revealed_at_last_visit: 0,
    };

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
// TUI-NAV-092 — the focused tab, reversed
//
// These are the tab bar's one set of assertions that read a style as well as
// symbols. That is no exception to TUI-NAV-022: reverse video is a fill — the
// terminal's own pair swapped — not a colour, so it survives every colour
// being stripped. The claim is read cell by cell: the reversed cells of the
// bar form exactly one run; that run is exactly the focused tab's label, its
// digit, a space, and its name; each of its cells leaves the pair unresolved
// for the terminal to invert (TUI-THEME-007); and the columns either side of
// it are the bar's own plain gaps, so the reversal brought no column with it.
// ---------------------------------------------------------------------------

/// A tab's label as the design fixes it: its digit, then its name.
fn tab_label(section: Section) -> String {
    format!("{} {}", section.digit(), section.title())
}

/// The tab bar drawn alone at `w` columns, as cells.
fn tab_cells(state: &GameState, view: &ViewState, w: u16) -> Vec<ratatui::buffer::Cell> {
    let mut term = terminal(w, 1);
    term.draw(|f| {
        let area = f.area();
        chrome::tabs(f, area, state, view);
    })
    .expect("TestBackend is infallible");
    row_cells(term.backend().buffer(), Rect::new(0, 0, w, 1), 0)
}

/// Every run of consecutive reversed cells on a line: its first column and
/// the text its cells hold.
fn reversed_runs(cells: &[ratatui::buffer::Cell]) -> Vec<(usize, String)> {
    use ratatui::style::Modifier;
    let mut runs: Vec<(usize, String)> = Vec::new();
    let mut open = false;
    for (x, cell) in cells.iter().enumerate() {
        if cell.modifier.contains(Modifier::REVERSED) {
            if !open {
                runs.push((x, String::new()));
                open = true;
            }
            runs.last_mut()
                .expect("a run is open")
                .1
                .push_str(cell.symbol());
        } else {
            open = false;
        }
    }
    runs
}

/// The whole of TUI-NAV-092's claim about one drawn tab bar: the focused
/// tab's label, and nothing else on the bar, is reversed; the reversal
/// inverts the terminal's own pair; and it is flanked by the bar's plain
/// gaps or its edge, so it added no column to the label.
fn assert_only_focus_reversed(cells: &[ratatui::buffer::Cell], focus: Section, what: &str) {
    use ratatui::style::Color;

    let line: String = cells.iter().map(|c| c.symbol()).collect();
    let label = tab_label(focus);
    let runs = reversed_runs(cells);
    assert_eq!(
        runs.len(),
        1,
        "{what}, with the {} section in focus, the tab bar reverses {} runs of cells {runs:?}. \
         Exactly one tab is reversed on every frame that draws the bar — the focused one — and \
         it is the one mark on the bar that says where the player is.\ntab bar: {line:?}",
        focus.title(),
        runs.len()
    );
    let (at, text) = &runs[0];
    assert_eq!(
        text, &label,
        "{what}, the tab bar reverses {text:?} at column {at}, where the focused tab's label is \
         {label:?} — its digit and its name, no more and no less: no other tab's label and no \
         column between tabs is reversed.\ntab bar: {line:?}"
    );
    let end = at + label.chars().count();
    for (x, cell) in cells.iter().enumerate().take(end).skip(*at) {
        assert_eq!(
            (cell.fg, cell.bg),
            (Color::Reset, Color::Reset),
            "{what}, focused-tab column {x} (`{}`) is reversed over {:?} on {:?}. The reversal \
             inverts the terminal's own foreground and background (TUI-THEME-007), never a \
             palette colour.\ntab bar: {line:?}",
            cell.symbol(),
            cell.fg,
            cell.bg
        );
    }
    if *at > 0 {
        assert_eq!(
            cells[at - 1].symbol(),
            " ",
            "{what}, the column before the focused tab's reversed label holds `{}`: the reversal \
             adds no column to the bar, so the label sits in the bar's own gap.\ntab bar: {line:?}",
            cells[at - 1].symbol()
        );
    }
    if end < cells.len() {
        assert_eq!(
            cells[end].symbol(),
            " ",
            "{what}, the column after the focused tab's reversed label holds `{}`: the reversal \
             adds no column to the bar, so the label sits in the bar's own gap.\ntab bar: {line:?}",
            cells[end].symbol()
        );
    }
}

/// The tab after `focus` in the ring the bar draws, if the bar holds another.
fn next_tab(state: &GameState, focus: Section) -> Option<Section> {
    let ring = tab_sections(state, Some(focus));
    let at = ring.iter().position(|s| *s == focus)?;
    let next = ring[(at + 1) % ring.len()];
    (next != focus).then_some(next)
}

/// Whether a drawn bar is in its full form: every tab on it named.
fn in_full_form(state: &GameState, focus: Section, line: &str) -> bool {
    tab_sections(state, Some(focus))
        .into_iter()
        .all(|s| line.contains(s.title()))
}

/// The focused tab's label — digit and name — is the one run of reversed
/// cells on the tab bar, in the full form and in the compact strip alike, its
/// cells inverting the terminal's own pair, and no other tab's label and no
/// column between tabs reversed. Swept over meadows where most sections list
/// nothing and where every one does, with every section in focus in turn —
/// one that lists nothing included — with no other tab badged and with the
/// next tab badged (so the compact strip names a tab beside the focused one),
/// at every width from the floor to wide.
///
/// @spec TUI-NAV-092
#[test]
fn only_the_focused_tabs_label_is_reversed_in_the_full_form_and_the_compact_strip() {
    let mut full_seen = false;
    let mut compact_seen = false;
    let mut compact_named_another = false;
    let mut empty_focus_seen = false;

    for (what, state) in reveal_corpus() {
        for focus in Section::ALL {
            if !rows(&state, focus).iter().any(|r| r.selectable()) {
                empty_focus_seen = true;
            }
            for badge_neighbour in [false, true] {
                let mut view = calm_view(&state);
                view.section = Some(focus);
                if badge_neighbour {
                    if let Some(s) = next_tab(&state, focus) {
                        view.seen[s.index()].visited = false;
                    }
                }
                for w in FLOOR_W..=WIDE_W {
                    let cells = tab_cells(&state, &view, w);
                    let line: String = cells.iter().map(|c| c.symbol()).collect();
                    if in_full_form(&state, focus, &line) {
                        full_seen = true;
                    } else {
                        compact_seen = true;
                        if Section::ALL
                            .into_iter()
                            .any(|s| s != focus && line.contains(s.title()))
                        {
                            compact_named_another = true;
                        }
                    }
                    let badges = if badge_neighbour {
                        "the next tab badged"
                    } else {
                        "no tab badged"
                    };
                    assert_only_focus_reversed(
                        &cells,
                        focus,
                        &format!("in {what} with {badges}, at {w} columns"),
                    );
                }
            }
        }
    }

    assert!(
        full_seen,
        "fixture: some case draws the bar in its full form"
    );
    assert!(
        compact_seen,
        "fixture: some case draws the bar as its compact strip"
    );
    assert!(
        compact_named_another,
        "fixture: some compact strip names a tab besides the focused one"
    );
    assert!(
        empty_focus_seen,
        "fixture: some case focuses a section that lists nothing"
    );
}

/// The reversal adds no column: over one set of tabs, the widths at which the
/// bar takes its full form are the same whichever tab is in focus, and in the
/// full form the bar's characters are the same whichever tab is in focus —
/// only which label is reversed differs.
///
/// @spec TUI-NAV-092
#[test]
fn the_focused_tabs_reversal_adds_no_column_to_the_bar() {
    let state = every_section_revealed();

    // The first focus's full-form widths and full-form lines, which every
    // other focus is held to.
    type FullForm = (Section, BTreeSet<u16>, Vec<(u16, String)>);
    let mut baseline: Option<FullForm> = None;
    for focus in Section::ALL {
        let mut view = calm_view(&state);
        view.section = Some(focus);
        assert_eq!(
            tab_sections(&state, Some(focus)),
            Section::ALL.to_vec(),
            "fixture: every section has its tab whatever is in focus"
        );

        let mut full_widths: BTreeSet<u16> = BTreeSet::new();
        let mut full_lines: Vec<(u16, String)> = Vec::new();
        for w in FLOOR_W..=WIDE_W {
            let line = tab_line(&state, &view, w);
            if in_full_form(&state, focus, &line) {
                full_widths.insert(w);
                full_lines.push((w, line));
            }
        }
        assert!(
            !full_widths.is_empty() && !full_widths.contains(&FLOOR_W),
            "fixture: the sweep reaches both the full form and the compact strip"
        );

        match &baseline {
            None => baseline = Some((focus, full_widths, full_lines)),
            Some((first, widths, lines)) => {
                assert_eq!(
                    &full_widths,
                    widths,
                    "with the {} tab in focus the bar takes its full form at widths {:?}..; with \
                     the {} tab in focus, at {:?}... The reversal adds no column, so the width at \
                     which the full form fits is the same whichever tab is in focus.",
                    focus.title(),
                    full_widths.first(),
                    first.title(),
                    widths.first()
                );
                for ((w, got), (_, want)) in full_lines.iter().zip(lines) {
                    assert_eq!(
                        got,
                        want,
                        "at {w} columns the full-form bar reads differently with the {} tab in \
                         focus than with the {} tab in focus. The reversal is a fill on the \
                         focused label, never a character or a column.",
                        focus.title(),
                        first.title()
                    );
                }
            }
        }
    }
}

/// The views a frame can hold over one focus: nothing open; a count prompt
/// on the focused row; the palette empty and with a query typed; the Journal;
/// help; the inspector called for (a swap at the floor, a pane wide); a
/// report banner showing; the next tab badged; and — where the focus is
/// the section with a heavy row — that row armed. Each keeps `focus` as the
/// section beneath.
fn focus_contexts(state: &GameState, focus: Section) -> Vec<(&'static str, ViewState)> {
    let mut base = calm_view(state);
    base.section = Some(focus);
    let first_row = rows(state, focus)
        .into_iter()
        .find(|r| r.selectable())
        .map(|r| r.key);
    base.focus[focus.index()] = first_row.clone();

    let mut out: Vec<(&'static str, ViewState)> = vec![("nothing open", base.clone())];

    if let Some(key) = first_row {
        let mut prompt = base.clone();
        prompt.prompt = Some(CountPrompt {
            digits: "3".to_string(),
            row: Some(key),
        });
        out.push(("a count prompt open", prompt));
    }

    let mut palette = base.clone();
    palette.overlay = Some(Overlay::Palette {
        query: String::new(),
        selected: 0,
    });
    out.push(("the palette open and empty", palette));

    let mut typed = base.clone();
    typed.overlay = Some(Overlay::Palette {
        query: "e".to_string(),
        selected: 0,
    });
    out.push(("the palette open with a query typed", typed));

    let mut journal = base.clone();
    journal.overlay = Some(Overlay::Journal { anchor: 0 });
    out.push(("the Journal open", journal));

    let mut help = base.clone();
    help.overlay = Some(Overlay::Help);
    out.push(("help open", help));

    let mut inspector = base.clone();
    inspector.inspector_wanted = true;
    out.push(("the inspector called for", inspector));

    let mut banner = base.clone();
    banner.reports = vec![PendingReport {
        entry: 0,
        dismissed: false,
    }];
    out.push(("a report banner showing", banner));

    if let Some(s) = next_tab(state, focus) {
        let mut badged = base.clone();
        badged.seen[s.index()].visited = false;
        out.push(("the next tab badged", badged));
    }

    if focus == Section::Legacy && rows(state, focus).iter().any(|r| r.heavy) {
        let key = heavy_row(state);
        let mut armed = base.clone();
        armed.focus[focus.index()] = Some(key.clone());
        armed.armed = Some(ArmedRow {
            key,
            command: Command::GreatMeadowMove,
            expires_at: Instant::now() + Duration::from_secs(8),
        });
        out.push(("a heavy row armed", armed));
    }

    out
}

/// On every whole frame that draws the tab bar, exactly one tab is reversed
/// and it is the section in focus — whatever else the view holds. While a
/// field (the count prompt, the palette) or a full-screen overlay (the
/// Journal, help) is open, the reversal stays on the section beneath; the
/// inspector, a banner, badges, and an armed row move it nowhere. Swept over
/// every section in focus, one that lists nothing included, in meadows young
/// and far along, at the floor and wide.
///
/// @spec TUI-NAV-092
#[test]
fn every_frame_reverses_the_section_in_focus_even_beneath_a_field_or_full_screen_overlay() {
    let mut contexts_seen: BTreeSet<&'static str> = BTreeSet::new();
    for (what, state) in reveal_corpus() {
        for focus in Section::ALL {
            for (context, view) in focus_contexts(&state, focus) {
                contexts_seen.insert(context);
                for (w, h) in [(FLOOR_W, FLOOR_H), (WIDE_W, WIDE_H)] {
                    let (buf, layout) = composed_frame(&state, &view, w, h);
                    let cells = row_cells(&buf, layout.tabs, layout.tabs.y);
                    assert_only_focus_reversed(
                        &cells,
                        focus,
                        &format!("in {what} with {context}, in a {w}x{h} frame"),
                    );
                }
            }
        }
    }
    for context in [
        "a count prompt open",
        "the palette open and empty",
        "the palette open with a query typed",
        "the Journal open",
        "help open",
        "a heavy row armed",
    ] {
        assert!(
            contexts_seen.contains(context),
            "fixture: the sweep draws a frame with {context}"
        );
    }
}

/// The reversal follows focus alone, and moves on the very frame focus moves:
/// from the opening focus, through a digit to every section, a section cycle
/// forward and back around the whole ring, and the palette's jump to every
/// section it can reach — the palette open over the old section keeping the
/// reversal there until the jump. Each step's expected tab is worked out from
/// the key pressed, never read back from the view, and every frame is drawn
/// at the floor (the compact strip) and wide (the full form).
///
/// @spec TUI-NAV-092
#[test]
fn the_reversal_moves_on_the_frame_a_digit_a_cycle_or_the_palettes_jump_moves_focus() {
    let state = every_section_revealed();
    let sizes = [(FLOOR_W, FLOOR_H), (WIDE_W, WIDE_H)];
    let check = |view: &ViewState, expected: Section, step: &str| {
        for (w, h) in sizes {
            let (buf, layout) = composed_frame(&state, view, w, h);
            let cells = row_cells(&buf, layout.tabs, layout.tabs.y);
            assert_only_focus_reversed(
                &cells,
                expected,
                &format!("after {step}, in a {w}x{h} frame"),
            );
        }
    };

    let mut view = ViewState::default();
    opening(&mut view, &state);
    check(&view, Section::Meadow, "the opening focus");

    let mut current = Section::Meadow;
    for target in Section::ALL {
        update(Action::SectionJump(target), &mut view, &state);
        check(&view, target, &format!("pressing {}", target.digit()));
        current = target;
    }

    let ring = tab_sections(&state, Some(current));
    for _ in 0..=ring.len() {
        let at = ring
            .iter()
            .position(|s| *s == current)
            .expect("the focused section has a tab");
        let next = ring[(at + 1) % ring.len()];
        update(Action::SectionNext, &mut view, &state);
        check(
            &view,
            next,
            &format!("cycling forward from {}", current.digit()),
        );
        current = next;
    }
    for _ in 0..=ring.len() {
        let at = ring
            .iter()
            .position(|s| *s == current)
            .expect("the focused section has a tab");
        let prev = ring[(at + ring.len() - 1) % ring.len()];
        update(Action::SectionPrev, &mut view, &state);
        check(
            &view,
            prev,
            &format!("cycling back from {}", current.digit()),
        );
        current = prev;
    }

    let mut landed: BTreeSet<Section> = BTreeSet::new();
    for target in Section::ALL {
        if target == current {
            continue;
        }
        let Some(query) = results(&state, "")
            .into_iter()
            .filter(|r| r.section == target)
            .map(|r| r.name)
            .find(|name| results(&state, name).first().map(|r| r.section) == Some(target))
        else {
            continue;
        };

        // Each jump is its own deliberate Enter, well apart from the last:
        // no field-accept guard (TUI-NAV-061) still stands from the jump before.
        view.field_guard = false;
        update(Action::OpenPalette, &mut view, &state);
        for c in query.chars() {
            update(Action::Type(c), &mut view, &state);
        }
        check(
            &view,
            current,
            &format!("typing a query for {} into the palette", target.digit()),
        );
        update(Action::Activate, &mut view, &state);
        check(
            &view,
            target,
            &format!("the palette's jump to {}", target.digit()),
        );
        landed.insert(target);
        current = target;
    }
    assert!(
        landed.len() >= 5,
        "fixture: the palette's jump reaches most sections ({} reached)",
        landed.len()
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-023, TUI-NAV-045 — the hint bar
// ---------------------------------------------------------------------------

/// The keys a hint bar can name, as whole tokens. Matching tokens rather than
/// substrings is what stops `Tab next` from reading as an offer of `x`.
const PROBE_KEYS: [&str; 11] = [
    "↑↓", "Enter", "Space", "Tab", "Esc", "x", "X", "+", "-", "i", "?",
];

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
        Some(at) => trimmed[at + 1..]
            .chars()
            .all(|c| c.is_alphabetic() || c.is_whitespace()),
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
    prompt.prompt = Some(CountPrompt {
        digits: "12".to_string(),
        ..Default::default()
    });

    vec![
        (
            "a plain one-shot row",
            focus_on(state, Section::Meadow, single_row(state)),
        ),
        (
            "a buildable row",
            focus_on(state, Section::Meadow, repeatable_row(state)),
        ),
        (
            "a count row",
            focus_on(state, Section::Clowder, count_row(state)),
        ),
        (
            "a count-taking row",
            focus_on(state, Section::Clowder, count_taking_row(state)),
        ),
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

    let counting = hint_line(
        &state,
        &focus_on(&state, Section::Clowder, count_row(&state)),
        WIDE_W,
    );
    for key in ["+", "-"] {
        assert!(
            has_key(&counting, key),
            "a count row is worked with `+` and `-` and the hint bar named neither: {counting:?}"
        );
    }

    for (what, section, key) in [
        (
            "a count-taking row",
            Section::Clowder,
            count_taking_row(&state),
        ),
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

    let plain = hint_line(
        &state,
        &focus_on(&state, Section::Meadow, single_row(&state)),
        WIDE_W,
    );
    assert!(
        !has_key(&plain, "x"),
        "the focused row's command is neither count-taking nor usefully repeatable, so `x` there \
         is a no-op with a footer note — and the hint bar offered it anyway: {plain:?}"
    );
}

/// The count key is named exactly beside the rows `x` opens the count prompt
/// on — every row tagged count-taking or repeat-N and every count row, unless
/// the core reports its item as already the player's — and beside no other,
/// so it never appears where it would only earn a refusal. The max key follows
/// the same rule for the rows `X` runs on, and so is never named beside a heavy
/// row: DeepNap's row names the count key alone. Quantified over every row the
/// game lists, in a meadow that also holds an owned one-shot and an owned
/// heavy row, with nothing open or armed to take the bar first.
///
/// @spec TUI-NAV-074
#[test]
fn the_hint_bar_names_x_and_x_max_exactly_beside_the_rows_they_act_on() {
    let mut state = every_section_revealed();
    state.arches_bought.insert(ArchId::EverdreamArch);
    let adopted = RuleId::ALL
        .into_iter()
        .find(|r| {
            let mut s = state.clone();
            s.rules.insert(*r);
            s.lock_reason(ContentId::Rule(*r)) == Some(LockReason::AlreadyYours)
        })
        .expect("fixture: some rule can be adopted and read back as the player's");
    state.rules.insert(adopted);

    let (mut count_rows, mut heavy_counted, mut owned_rows) = (0, 0, 0);
    for section in Section::ALL {
        for row in rows(&state, section).into_iter().filter(|r| r.selectable()) {
            let owned = match &row.key {
                RowKey::Content(id, _) => state.lock_reason(*id) == Some(LockReason::AlreadyYours),
                _ => false,
            };
            let tagged = match &row.kind {
                RowKind::Act { semantics, .. } => {
                    matches!(
                        semantics,
                        CountSemantics::CountTaking | CountSemantics::RepeatN
                    )
                }
                RowKind::Count { .. } => true,
                RowKind::Display | RowKind::Group => false,
            };
            let x_opens = tagged && !owned;
            let max_runs = x_opens && !row.heavy;
            if owned {
                owned_rows += 1;
            }
            if matches!(row.kind, RowKind::Count { .. }) {
                count_rows += 1;
            }
            if x_opens && row.heavy {
                heavy_counted += 1;
            }

            let line = hint_line(&state, &focus_on(&state, section, row.key.clone()), WIDE_W);
            assert_eq!(
                has_key(&line, "x"),
                x_opens,
                "with {:?} focused in {}, the hint bar {} the count key, but x {} the count \
                 prompt there.\nhint bar: {line:?}",
                row.key,
                section.title(),
                if has_key(&line, "x") {
                    "names"
                } else {
                    "does not name"
                },
                if x_opens { "opens" } else { "does not open" },
            );
            assert_eq!(
                has_key(&line, "X"),
                max_runs,
                "with {:?} focused in {}, the hint bar {} the max key, but X {} there — and \
                 never on a heavy row.\nhint bar: {line:?}",
                row.key,
                section.title(),
                if has_key(&line, "X") {
                    "names"
                } else {
                    "does not name"
                },
                if max_runs { "runs" } else { "does not run" },
            );
        }
    }

    assert!(count_rows > 0, "fixture: no count row is listed");
    assert!(
        heavy_counted > 0,
        "fixture: the heavy DeepNap row is not listed"
    );
    assert!(
        owned_rows > 0,
        "fixture: no row the core reports as the player's is listed"
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-082 — the mnemonics the hint bar names
// ---------------------------------------------------------------------------

/// The Meadow's fixed mnemonics, in their rows' list order — the design's
/// table.
const FIXED_MNEMONICS: [(char, &str); 3] =
    [('g', "GatherCatnip"), ('f', "Forage"), ('r', "Rummage")];

/// The letters the global key table binds. A single lowercase letter on the
/// hint bar that is not one of these can only be naming a mnemonic — `n` is
/// one of these, not a mnemonic: TUI-NAV-046 carves it out as the one letter
/// the global table binds to a kitten's row without standing for a row of its
/// own (TUI-NAV-094), and TUI-NAV-082/096 name it on the bar by that rule.
const GLOBAL_LETTERS: [char; 10] = ['h', 'j', 'k', 'l', 'i', 'x', 'e', 'o', 'q', 'n'];

/// Whether a mnemonic is fixed (stands for one fixed row of its screen) or
/// stands for a row of the item under the cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MnemonicKind {
    Fixed,
    FocusedItem,
}

/// The mnemonics live on `section` with `focus` under the cursor, each with
/// the row it stands for and its kind, fixed ones in list order — the design's
/// table as a function of state.
fn live_mnemonics(
    state: &GameState,
    section: Section,
    focus: Option<&RowKey>,
) -> Vec<(char, RowKey, MnemonicKind)> {
    let listed = rows(state, section);
    let is_listed = |k: &RowKey| listed.iter().any(|r| &r.key == k);
    let mut out = Vec::new();
    match section {
        Section::Meadow => {
            for (c, tag) in FIXED_MNEMONICS {
                let key = RowKey::Fixed(tag);
                if is_listed(&key) {
                    out.push((c, key, MnemonicKind::Fixed));
                }
            }
        }
        Section::Clowder => {
            // A kitten is one row (TUI-NAV-093); `p` stands for that very
            // row. `n`, the row-bound key TUI-NAV-094 states, is not a
            // mnemonic under this table's rule (TUI-NAV-046) and so is never
            // in this list.
            if let Some(key @ RowKey::Kitten(..)) = focus {
                if is_listed(key) {
                    out.push(('p', key.clone(), MnemonicKind::FocusedItem));
                }
            }
        }
        Section::Friends => {
            if let Some(RowKey::Colony(colony, _)) = focus {
                let key = RowKey::Colony(*colony, "penpal");
                if is_listed(&key) {
                    out.push(('p', key, MnemonicKind::FocusedItem));
                }
            }
        }
        _ => {}
    }
    out
}

/// The keys a line names, in order and with repeats kept: the probe keys, and
/// every single lowercase letter standing as a token of its own — read only
/// where a key stands, at the head of a hint, so a one-letter word inside a
/// hint's own wording (a later row's label, TUI-NAV-090) is never read as a
/// key.
fn named_keys(line: &str) -> Vec<String> {
    line.split(|c: char| SEPARATORS.contains(&c))
        .filter_map(|entry| entry.split_whitespace().next())
        .map(str::to_string)
        .filter(|t| {
            PROBE_KEYS.contains(&t.as_str())
                || (t.chars().count() == 1 && t.chars().all(|c| c.is_ascii_lowercase()))
        })
        .collect()
}

/// The hints a line names, each trimmed, split where a hint ends.
fn hint_entries(line: &str) -> Vec<String> {
    line.split(|c: char| SEPARATORS.contains(&c))
        .map(|e| e.trim().to_string())
        .filter(|e| !e.is_empty())
        .collect()
}

/// The label TUI-NAV-089 draws in the name's place on a later row of an item,
/// keyed by the row's own tag.
fn later_row_label(key: &RowKey) -> Option<&'static str> {
    match key {
        RowKey::Content(ContentId::Craft(_), "tinker") => Some("tinkerers"),
        RowKey::Colony(_, "penpal") => Some("send a pen-pal"),
        _ => None,
    }
}

/// Whether `target` is a later row of its item in `section` — a row of an
/// item that carries several, other than the first of them listed.
fn is_later_row(state: &GameState, section: Section, target: &RowKey) -> bool {
    let same_item = |k: &RowKey| match (k, target) {
        (RowKey::Content(a, _), RowKey::Content(b, _)) => a == b,
        (RowKey::Kitten(a, _), RowKey::Kitten(b, _)) => a == b,
        (RowKey::Colony(a, _), RowKey::Colony(b, _)) => a == b,
        _ => k == target,
    };
    rows(state, section)
        .iter()
        .find(|r| same_item(&r.key))
        .is_some_and(|first| &first.key != target)
}

/// The word naming the item a focused-item mnemonic acts on.
fn item_word(key: &RowKey) -> &'static str {
    match key {
        RowKey::Kitten(..) => "kitten",
        RowKey::Colony(..) => "colony",
        RowKey::Content(ContentId::Craft(_), _) => "recipe",
        _ => "item",
    }
}

/// Wherever the hint bar names a focused-item mnemonic whose row is a later
/// row of its item, it words the letter's act by that row's label, so the act
/// reads the same beside its letter as on its row. Quantified over every
/// selectable row of every screen as the focus, at the floor, at the pane
/// breakpoint, and wide — every context in which such a letter is named.
///
/// @spec TUI-NAV-090
#[test]
fn the_hint_bar_words_a_later_rows_letter_by_that_rows_label() {
    let state = every_section_revealed();
    let mut named: BTreeSet<(char, &'static str)> = BTreeSet::new();
    for w in [FLOOR_W, 100, WIDE_W] {
        for section in Section::ALL {
            for row in rows(&state, section).into_iter().filter(|r| r.selectable()) {
                let view = focus_on(&state, section, row.key.clone());
                let line = hint_line(&state, &view, w);
                let entries = hint_entries(&line);
                for (c, target, kind) in live_mnemonics(&state, section, Some(&row.key)) {
                    if kind != MnemonicKind::FocusedItem
                        || target == row.key
                        || !is_later_row(&state, section, &target)
                    {
                        continue;
                    }
                    let label = later_row_label(&target).unwrap_or_else(|| {
                        panic!("fixture: {target:?} is a later row with no label TUI-NAV-089 names")
                    });
                    let lead = format!("{c} ");
                    let mine: Vec<&String> =
                        entries.iter().filter(|e| e.starts_with(&lead)).collect();
                    let whr = format!(
                        "at {w} columns on the {} screen with {:?} focused",
                        section.title(),
                        row.key
                    );
                    for entry in &mine {
                        assert_eq!(
                            entry.as_str(),
                            format!("{c} {label}"),
                            "{whr}, the hint bar names `{c}`, which acts on the later row \
                             {target:?}, as {entry:?}; the letter's act is worded by that row's \
                             label {label:?}.\nhint bar: {line:?}"
                        );
                    }
                    if w == WIDE_W {
                        assert_eq!(
                            mine.len(),
                            1,
                            "fixture: {whr}, the hint bar names the live letter `{c}` once at a \
                             width where nothing is dropped.\nhint bar: {line:?}"
                        );
                    }
                    if !mine.is_empty() {
                        named.insert((c, label));
                    }
                }
            }
        }
    }
    assert!(
        named.contains(&('p', "send a pen-pal")),
        "fixture: the later-row letter was named on the bar somewhere; named {named:?}"
    );
}

/// Wherever the help overlay names a focused-item mnemonic whose row is a
/// later row of its item, it words the letter's act by that row's label and
/// follows it with the item it acts on, in parentheses — so the act reads the
/// same beside its letter as on its row. Quantified over every such letter the
/// design's table spends: each (screen, letter) whose live mnemonic, with any
/// row of the screen focused, stands for a later row.
///
/// @spec TUI-NAV-090
#[test]
fn the_help_overlay_words_a_later_rows_letter_by_its_label_and_names_its_item() {
    let state = every_section_revealed();
    let mut later: Vec<(Section, char, &'static str, &'static str)> = Vec::new();
    for section in Section::ALL {
        for row in rows(&state, section).into_iter().filter(|r| r.selectable()) {
            for (c, target, kind) in live_mnemonics(&state, section, Some(&row.key)) {
                if kind != MnemonicKind::FocusedItem || !is_later_row(&state, section, &target) {
                    continue;
                }
                let label = later_row_label(&target).unwrap_or_else(|| {
                    panic!("fixture: {target:?} is a later row with no label TUI-NAV-089 names")
                });
                let entry = (section, c, label, item_word(&target));
                if !later.contains(&entry) {
                    later.push(entry);
                }
            }
        }
    }
    assert!(
        later
            .iter()
            .any(|(s, c, _, _)| *s == Section::Friends && *c == 'p'),
        "fixture: the later-row letter was found; found {later:?}"
    );

    let (w, h) = (120u16, 60u16);
    let mut term = terminal(w, h);
    term.draw(|f| {
        let area = f.area();
        overlays::help(f, area);
    })
    .expect("TestBackend is infallible");
    let lines = region_lines(term.backend().buffer(), Rect::new(0, 0, w, h));

    for (section, c, label, item) in later {
        let line = lines
            .iter()
            .find(|l| l.trim_start().starts_with(section.title()))
            .unwrap_or_else(|| {
                panic!(
                    "the help overlay has no line for the {} screen's letters",
                    section.title()
                )
            });
        let worded = format!("{c} {label} (");
        let at = line.find(&worded).unwrap_or_else(|| {
            panic!(
                "the help overlay's {} line does not word `{c}` by its row's label followed by \
                 its item in parentheses — `{c} {label} (…)`.\nline: {line:?}",
                section.title()
            )
        });
        let rest = &line[at + worded.len()..];
        let inside = rest
            .split(')')
            .next()
            .filter(|_| rest.contains(')'))
            .unwrap_or_else(|| {
                panic!("the help overlay's `{c} {label} (` is never closed.\nline: {line:?}")
            });
        assert!(
            inside.split_whitespace().any(|word| word == item),
            "the help overlay follows `{c} {label}` with ({inside}), which does not name the \
             {item} the letter acts on.\nline: {line:?}"
        );
    }
}

/// Wherever the hint bar shows the focused row's keys, it names every live
/// mnemonic but the one whose row is the focused row, and no other letter as
/// a mnemonic; a focused-item letter comes directly after the key that names
/// Enter, and the fixed letters come in their rows' list order after every
/// other context key and just before the help key. Quantified over every
/// selectable row of every revealed screen — and each screen with no row
/// focused — in a meadow where every mnemonic's row is listed and in a new
/// meadow where the rummage row is not yet, with nothing open or armed to take
/// the bar, at a width where nothing need be dropped.
///
/// @spec TUI-NAV-082
#[test]
fn the_hint_bar_names_every_live_mnemonic_but_the_focused_rows_own_in_its_place() {
    let far = every_section_revealed();
    let fresh = GameState::new(3);
    assert!(
        rows(&far, Section::Meadow)
            .iter()
            .any(|r| r.key == RowKey::Fixed("Rummage")),
        "fixture: the meadow far along lists the rummage row"
    );
    assert!(
        !rows(&fresh, Section::Meadow)
            .iter()
            .any(|r| r.key == RowKey::Fixed("Rummage")),
        "fixture: a new meadow does not yet list the rummage row"
    );

    let (mut fixed_named, mut item_named, mut own_unnamed) = (0u32, 0u32, 0u32);
    for (label, state) in [("a meadow far along", &far), ("a new meadow", &fresh)] {
        for section in Section::ALL {
            let listed = rows(state, section);
            if listed.is_empty() {
                continue;
            }
            let mut contexts: Vec<Option<RowKey>> = vec![None];
            contexts.extend(
                listed
                    .iter()
                    .filter(|r| r.selectable())
                    .map(|r| Some(r.key.clone())),
            );

            for focus in contexts {
                let view = match &focus {
                    Some(k) => focus_on(state, section, k.clone()),
                    None => calm_view(state),
                };
                let view = ViewState {
                    section: Some(section),
                    ..view
                };
                let line = hint_line(state, &view, WIDE_W);
                let keys = named_keys(&line);
                let whr = match &focus {
                    Some(k) => format!(
                        "in {label}, on the {} screen with {k:?} focused",
                        section.title()
                    ),
                    None => format!(
                        "in {label}, on the {} screen with no row focused",
                        section.title()
                    ),
                };

                let live = live_mnemonics(state, section, focus.as_ref());
                let expected: Vec<(char, MnemonicKind)> = live
                    .iter()
                    .filter(|(_, target, _)| Some(target) != focus.as_ref())
                    .map(|(c, _, kind)| (*c, *kind))
                    .collect();
                own_unnamed += (live.len() - expected.len()) as u32;

                // The Ideas screen's done key is the other letter TUI-NAV-082
                // names that is not a mnemonic; where it is named, it is named
                // as TUI-NAV-122 has it, and its own test covers where.
                if section == Section::Ideas {
                    for entry in hint_entries(&line).iter().filter(|e| e.starts_with("d ")) {
                        assert!(
                            entry == "d show done" || entry == "d fold done",
                            "{whr}, the hint bar names the done key as {entry:?}.\nhint bar: {line:?}"
                        );
                    }
                }

                // Exactly the expected letters, each once, and no other letter
                // outside the global table.
                let mut named_letters: Vec<char> = keys
                    .iter()
                    .filter_map(|k| k.chars().next().filter(|_| k.chars().count() == 1))
                    .filter(|c| c.is_ascii_lowercase() && !GLOBAL_LETTERS.contains(c))
                    .filter(|c| !(section == Section::Ideas && *c == 'd'))
                    .collect();
                named_letters.sort_unstable();
                let mut want: Vec<char> = expected.iter().map(|(c, _)| *c).collect();
                want.sort_unstable();
                assert_eq!(
                    named_letters, want,
                    "{whr}, the hint bar names the mnemonic letters {named_letters:?}, but the \
                     live mnemonics whose row is not the focused row are {want:?}; the bar names \
                     every one of those and no other letter as a mnemonic.\nhint bar: {line:?}"
                );

                // A focused-item letter sits directly after the key naming Enter.
                for (c, _) in expected
                    .iter()
                    .filter(|(_, k)| *k == MnemonicKind::FocusedItem)
                {
                    let at = keys.iter().position(|k| k == "Enter").unwrap_or_else(|| {
                        panic!("fixture: {whr}, the hint bar names no Enter key: {line:?}")
                    });
                    assert_eq!(
                        keys.get(at + 1).map(String::as_str),
                        Some(c.to_string().as_str()),
                        "{whr}, the focused-item mnemonic `{c}` is not named directly after the \
                         key that names Enter; it is the other act on the item Enter acts on.\n\
                         keys in order: {keys:?}\nhint bar: {line:?}"
                    );
                    item_named += 1;
                }

                // The fixed letters, in list order, are the last keys before
                // the help key, after every other context key.
                let fixed: Vec<String> = expected
                    .iter()
                    .filter(|(_, k)| *k == MnemonicKind::Fixed)
                    .map(|(c, _)| c.to_string())
                    .collect();
                if !fixed.is_empty() {
                    assert_eq!(
                        keys.last().map(String::as_str),
                        Some("?"),
                        "{whr}, the hint bar does not end with the help key: {line:?}"
                    );
                    let tail_start = keys.len().checked_sub(1 + fixed.len()).unwrap_or_else(|| {
                        panic!("{whr}, the hint bar names too few keys to hold {fixed:?}: {line:?}")
                    });
                    assert_eq!(
                        &keys[tail_start..keys.len() - 1],
                        &fixed[..],
                        "{whr}, the fixed mnemonics are not named in their rows' list order after \
                         every other context key and just before the help key.\nkeys in order: \
                         {keys:?}\nhint bar: {line:?}"
                    );
                    fixed_named += fixed.len() as u32;
                }
            }
        }
    }

    assert!(
        fixed_named > 0,
        "fixture: no fixed mnemonic was ever expected on the bar"
    );
    assert!(
        item_named > 0,
        "fixture: no focused-item mnemonic was ever expected on the bar"
    );
    assert!(
        own_unnamed > 0,
        "fixture: no context focused a mnemonic's own row"
    );

    // A full-screen overlay suspends every mnemonic the same way it suspends
    // the rest of the global table, so no letter it would otherwise name is
    // live, and the bar must not name one: `keys::resolve` leaves every
    // mnemonic letter unbound while an overlay is open.
    let meadow_focus = first_row_where(&far, Section::Meadow, "the gather row", |r| {
        r.key == RowKey::Fixed("GatherCatnip")
    });
    assert!(
        !live_mnemonics(&far, Section::Meadow, Some(&meadow_focus)).is_empty(),
        "fixture: the Meadow has a live mnemonic other than gather's own"
    );
    for overlay in [Overlay::Journal { anchor: 0 }, Overlay::Help] {
        let mut view = focus_on(&far, Section::Meadow, meadow_focus.clone());
        view.overlay = Some(overlay);
        let line = hint_line(&far, &view, WIDE_W);
        let keys = named_keys(&line);
        let named_letters: Vec<char> = keys
            .iter()
            .filter_map(|k| k.chars().next().filter(|_| k.chars().count() == 1))
            .filter(|c| c.is_ascii_lowercase() && !GLOBAL_LETTERS.contains(c))
            .collect();
        assert!(
            named_letters.is_empty(),
            "with a full-screen overlay open, the hint bar named the mnemonic letters \
             {named_letters:?}, but `keys::resolve` leaves every mnemonic unbound while an \
             overlay holds the keyboard.\nhint bar: {line:?}"
        );
    }
}

/// The cold open's line, stated in the design in full: the backbone leads,
/// and the one letter after it names the Meadow action the cursor is not on —
/// gather's own letter unnamed while Enter names that act — before the help
/// key, at the supported floor.
///
/// @spec TUI-NAV-082, TUI-NAV-024
#[test]
fn the_cold_open_names_the_other_fixed_letter_after_the_backbone_and_not_the_focused_rows_own() {
    let state = GameState::new(3);
    let view = focus_on(&state, Section::Meadow, RowKey::Fixed("GatherCatnip"));
    let line = hint_line(&state, &view, FLOOR_W);
    let keys = named_keys(&line);
    assert_eq!(
        keys,
        ["↑↓", "Enter", "Space", "Tab", "f", "?"]
            .map(String::from)
            .to_vec(),
        "the cold-open hint bar names the backbone, then the forage mnemonic, then the help key — \
         and not the gather mnemonic, whose row is the focused row.\nhint bar: {line:?}"
    );
}

/// An open field holds the keyboard, so the hint bar speaks for the field and
/// nothing else: the same line whatever lies beneath it — any kind of focused
/// row, or an armed heavy row whose own `Enter yes, truly` would otherwise be
/// the most urgent thing on screen. The field is what Enter will reach, so an
/// armed row's line under an open field would be a promise the next Enter does
/// not keep. The palette's line names typing to find, Enter, and Esc.
///
/// @spec TUI-NAV-066
#[test]
fn an_open_field_owns_the_hint_bar_whatever_lies_beneath_it() {
    let state = every_section_revealed();

    let beneath: Vec<(&str, ViewState)> = hint_contexts(&state)
        .into_iter()
        .filter(|(_, v)| v.prompt.is_none())
        .collect();
    assert!(
        beneath.iter().any(|(_, v)| v.armed.is_some()),
        "fixture: one of the contexts beneath the field is an armed heavy row"
    );
    let bare_lines: Vec<(&str, String)> = beneath
        .iter()
        .map(|(what, v)| (*what, hint_line(&state, v, WIDE_W)))
        .collect();

    type Opener = fn(&mut ViewState);
    let fields: [(&str, Opener); 2] = [
        ("the count prompt", |v| {
            v.prompt = Some(CountPrompt {
                digits: "12".to_string(),
                ..Default::default()
            })
        }),
        ("the palette", |v| {
            v.overlay = Some(Overlay::Palette {
                query: "ab".to_string(),
                selected: 0,
            })
        }),
    ];

    for (field, open) in fields {
        let lines: Vec<(&str, String)> = beneath
            .iter()
            .map(|(what, v)| {
                let mut v = v.clone();
                open(&mut v);
                (*what, hint_line(&state, &v, WIDE_W))
            })
            .collect();

        let (first_what, first) = &lines[0];
        for (what, line) in &lines[1..] {
            assert_eq!(
                line, first,
                "with {field} open, the hint bar over {what} differs from the one over \
                 {first_what}; while a field is open the bar shows the field's own keys in place \
                 of any other context's"
            );
        }
        for (what, bare) in &bare_lines {
            assert_ne!(
                first, bare,
                "with {field} open, the hint bar still reads as it does over {what} with nothing \
                 open; the field's keys replace the context's"
            );
        }
        assert!(
            has_key(first, "Enter"),
            "with {field} open, the hint bar does not name Enter: {first:?}"
        );
        assert!(
            has_key(first, "Esc"),
            "with {field} open, the hint bar does not name Esc: {first:?}"
        );
        if field == "the palette" {
            assert!(
                says(first, "type") && says(first, "find"),
                "with the palette open, the hint bar does not say that typing finds: {first:?}"
            );
        }
    }
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

        let keys: Vec<String> = keys_in_order(&line)
            .into_iter()
            .filter(|k| k != "?")
            .collect();

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
// TUI-NAV-077 — the armed row's keys, and only while the arming holds
// ---------------------------------------------------------------------------

/// Words only the armed row's line says. Either one on the bar is the bar
/// showing a row armed.
const ARMED_WORDS: [&str; 2] = ["yes, truly", "not today"];

fn shows_armed(line: &str) -> bool {
    ARMED_WORDS.iter().any(|w| line.contains(w))
}

/// Every heavy row listed anywhere, with its section and command.
fn every_heavy_row(state: &GameState) -> Vec<(Section, RowKey, Command)> {
    let mut out = Vec::new();
    for section in Section::ALL {
        for r in rows(state, section) {
            if let RowKind::Act { command, .. } = r.kind {
                if r.heavy {
                    out.push((section, r.key, command));
                }
            }
        }
    }
    out
}

fn armed_on(state: &GameState, section: Section, key: RowKey, command: Command) -> ViewState {
    let mut v = focus_on(state, section, key.clone());
    v.armed = Some(ArmedRow {
        key,
        command,
        expires_at: Instant::now() + Duration::from_secs(8),
    });
    v
}

/// While a row is armed and no field or full-screen overlay holds the
/// keyboard, the hint bar shows the armed row's keys — for every heavy row,
/// with or without the inspector asked for, under a report banner, and at
/// every width, since no layout ends an arming.
///
/// @spec TUI-NAV-077
#[test]
fn the_armed_rows_keys_show_wherever_nothing_holds_the_keyboard() {
    let state = every_section_revealed();
    let heavy = every_heavy_row(&state);
    let kinds: BTreeSet<String> = heavy
        .iter()
        .map(|(_, _, c)| format!("{c:?}").split('(').next().unwrap().to_string())
        .collect();
    assert_eq!(
        kinds.len(),
        4,
        "fixture: all four heavy actions are listed ({kinds:?})"
    );

    for (section, key, command) in heavy {
        let armed = armed_on(&state, section, key.clone(), command);
        let contexts: Vec<(&str, ViewState)> = vec![
            ("nothing else", armed.clone()),
            (
                "the inspector asked for",
                ViewState {
                    inspector_wanted: true,
                    ..armed.clone()
                },
            ),
            (
                "a report banner showing",
                ViewState {
                    reports: vec![PendingReport {
                        entry: 0,
                        dismissed: false,
                    }],
                    ..armed.clone()
                },
            ),
        ];
        for (what, view) in contexts {
            for w in [FLOOR_W, 99, 100, 120, WIDE_W] {
                let line = hint_line(&state, &view, w);
                assert!(
                    line.contains(ARMED_WORDS[0]),
                    "with {command:?} armed and {what}, a {w}-column hint bar read {line:?}; \
                     the armed row's keys belong there wherever no field or overlay holds \
                     the keyboard"
                );
            }
        }
    }
}

/// A full-screen overlay holds the keyboard: under the Journal or help, the
/// next Enter does not reach the armed row, so the bar does not offer it.
///
/// @spec TUI-NAV-077
#[test]
fn a_full_screen_overlay_shows_no_armed_keys() {
    let state = every_section_revealed();
    for (section, key, command) in every_heavy_row(&state) {
        for overlay in [Overlay::Journal { anchor: 0 }, Overlay::Help] {
            let mut view = armed_on(&state, section, key.clone(), command);
            view.overlay = Some(overlay.clone());
            for w in [FLOOR_W, WIDE_W] {
                let line = hint_line(&state, &view, w);
                assert!(
                    !shows_armed(&line),
                    "with {overlay:?} open over armed {command:?}, a {w}-column hint bar read \
                     {line:?}; the overlay holds the keyboard, not the armed row"
                );
            }
        }
    }
}

/// The moment an arming ends the bar stops showing it: an arming past its
/// expiry, or one whose row is no longer listed, leaves nothing on the bar
/// that reads as armed — a stale "Enter yes, truly" would promise a commit
/// the next Enter does not make.
///
/// @spec TUI-NAV-077, TUI-NAV-076
#[test]
fn an_ended_arming_leaves_no_armed_keys_on_the_hint_bar() {
    let state = every_section_revealed();
    let past = Instant::now()
        .checked_sub(Duration::from_secs(1))
        .expect("clock has a past");

    for (section, key, command) in every_heavy_row(&state) {
        let mut view = armed_on(&state, section, key, command);
        view.armed.as_mut().expect("armed").expires_at = past;
        for w in [FLOOR_W, WIDE_W] {
            let line = hint_line(&state, &view, w);
            assert!(
                !shows_armed(&line),
                "an expired arming of {command:?} still reads as armed on a {w}-column bar: \
                 {line:?}"
            );
        }
    }

    // The nap row leaves every list once its gate closes.
    let (_, nap_key, nap_command) = every_heavy_row(&state)
        .into_iter()
        .find(|(_, _, c)| matches!(c, Command::DeepNap(_)))
        .expect("fixture: the nap row is listed");
    let mut gone = state.clone();
    gone.ideas.remove(&IdeaId::DeepNapTheory);
    assert!(
        Section::ALL
            .iter()
            .all(|s| rows(&gone, *s).iter().all(|r| r.key != nap_key)),
        "fixture: the nap row must be listed nowhere"
    );
    let mut view = focus_on(&gone, Section::Legacy, heavy_row(&gone));
    view.armed = Some(ArmedRow {
        key: nap_key,
        command: nap_command,
        expires_at: Instant::now() + Duration::from_secs(8),
    });
    for w in [FLOOR_W, WIDE_W] {
        let line = hint_line(&gone, &view, w);
        assert!(
            !shows_armed(&line),
            "an arming whose row is no longer listed still reads as armed on a {w}-column bar: \
             {line:?}"
        );
    }
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
    assert!(
        ends_with_help_key(&line),
        "and it ends with the key to everything else: {line:?}"
    );
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

    // The first row the cursor can land on: a group header above it is a
    // signpost, not a row (TUI-NAV-099).
    assert_eq!(
        rows(&state, Section::Meadow)
            .into_iter()
            .find(|r| r.selectable())
            .map(|r| r.key),
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
const LEGEND_GLYPHS: [char; 10] = ['●', '◐', '○', '✓', '▸', '★', '◦', '~', '!', '◆'];

/// The keys whose spelling is not in question: a word, or a single character
/// that is itself the key.
const HELP_WORDS: [&str; 4] = ["Enter", "Space", "Tab", "Esc"];
const HELP_KEYS: [&str; 9] = ["?", "/", "x", "X", "q", "e", "o", "<", ">"];

/// Every letter the design's per-screen mnemonic table spends (TUI-NAV-046) —
/// live only on its own screen, so the hint bar never carries a permanent
/// line for it and the help overlay is the one place a player can find them
/// all, whatever screen they are on.
const MNEMONIC_LETTERS: [char; 5] = ['g', 'f', 'r', 'p', 'n'];

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
    for letter in MNEMONIC_LETTERS {
        assert!(
            has_key(&text, &letter.to_string()),
            "the help overlay does not list the mnemonic letter `{letter}`. A mnemonic is live \
             only on its own screen, so a letter the hint bar dropped for width — or never shows \
             because the player is elsewhere — must still be found here: `?` is the one place \
             every mnemonic is named at once."
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

// ---------------------------------------------------------------------------
// TUI-NAV-096 — the row-bound key, named on the hint bar and in the help
// overlay
// ---------------------------------------------------------------------------

/// Every kitten row the Clowder section lists, with the roster position of the
/// kitten it is about — found by the kitten, never by a tag.
fn kitten_rows(state: &GameState) -> Vec<(usize, RowKey)> {
    rows(state, Section::Clowder)
        .into_iter()
        .filter_map(|r| match r.key {
            RowKey::Kitten(ix, _) => Some((ix, r.key)),
            _ => None,
        })
        .collect()
}

/// Wherever the hint bar shows the focused row's keys with the cursor on a
/// kitten's row whose AppointElder gate is open and whose kitten is not the
/// one the core holds as the Elder — the bare screen, the inspector asked for,
/// a report banner up — it names the row-bound key as `n appoint Elder`,
/// directly after the key that names Enter, ranked for width as a
/// focused-item mnemonic is: as the bar narrows it outlasts every key named
/// after it, the inspector and section keys among them, and where it is named
/// it is still directly after Enter. Quantified over every kitten row, with
/// nobody the Elder and with each other kitten the Elder in turn, from past
/// the widest bar down to where keys are shed. The Elder's own row is
/// `the_hint_bar_leaves_n_unnamed_on_the_elders_own_row_where_the_key_stays_live`.
///
/// @spec TUI-NAV-096
#[test]
fn the_hint_bar_names_n_appoint_elder_directly_after_enter_on_every_kitten_row_but_the_elders() {
    let base = every_section_revealed();
    let kittens = kitten_rows(&base);
    assert!(kittens.len() > 1, "fixture: several kitten rows are listed");

    let widths = [WIDE_W, 120, 100, FLOOR_W, 60, 44, 32, 24, 16];
    let (mut named_wide, mut shed_after_n, mut beside_an_elder) = (0u32, 0u32, 0u32);
    let elders: Vec<Option<usize>> = std::iter::once(None)
        .chain(kittens.iter().map(|(ix, _)| Some(*ix)))
        .collect();
    for elder in elders {
        let mut state = base.clone();
        state.elder = elder;
        for (ix, key) in kitten_rows(&state) {
            if elder == Some(ix) {
                continue; // the Elder's own row: the bar leaves `n` unnamed there
            }
            if elder.is_some() {
                beside_an_elder += 1;
            }
            assert!(
                state.command_gate_open(&Command::AppointElder(ix)),
                "fixture: the core reports the gate of AppointElder open for kitten {ix}"
            );
            let bare = focus_on(&state, Section::Clowder, key.clone());
            let contexts: [(&str, ViewState); 3] = [
                ("nothing else", bare.clone()),
                (
                    "the inspector asked for",
                    ViewState {
                        inspector_wanted: true,
                        ..bare.clone()
                    },
                ),
                (
                    "a report banner up",
                    ViewState {
                        reports: vec![PendingReport {
                            entry: 0,
                            dismissed: false,
                        }],
                        ..bare.clone()
                    },
                ),
            ];
            for (what, view) in contexts {
                let mut widest: Option<Vec<String>> = None;
                for w in widths {
                    let line = hint_line(&state, &view, w);
                    let keys = named_keys(&line);
                    let whr = format!(
                        "at {w} columns with the kitten row {key:?} focused, {} and {what}",
                        match elder {
                            Some(e) => format!("kitten {e} the Elder"),
                            None => "nobody the Elder".to_string(),
                        }
                    );
                    let n_entries: Vec<String> = hint_entries(&line)
                        .into_iter()
                        .filter(|e| e.split_whitespace().next() == Some("n"))
                        .collect();

                    for entry in &n_entries {
                        assert_eq!(
                            entry, "n appoint Elder",
                            "{whr}, the hint bar names `n` as {entry:?}; the row-bound key is named \
                             `n appoint Elder`.\nhint bar: {line:?}"
                        );
                    }
                    if let Some(at) = keys.iter().position(|k| k == "n") {
                        assert!(
                            at > 0 && keys[at - 1] == "Enter",
                            "{whr}, `n` is not named directly after the key that names Enter.\nkeys in \
                             order: {keys:?}\nhint bar: {line:?}"
                        );
                    }

                    match &widest {
                        None => {
                            assert_eq!(
                                n_entries.len(),
                                1,
                                "{whr}, where nothing need be dropped, the hint bar names `n` {} times; \
                                 while `n` is live on the focused row of a kitten who is not the Elder \
                                 the bar names it once.\nhint bar: {line:?}",
                                n_entries.len()
                            );
                            named_wide += 1;
                            widest = Some(keys.clone());
                        }
                        Some(wide) => {
                            let n_at = wide
                                .iter()
                                .position(|k| k == "n")
                                .expect("named at the widest");
                            let after_n = &wide[n_at + 1..];
                            let kept_after = keys.iter().any(|k| after_n.contains(k) && k != "?");
                            if kept_after {
                                assert!(
                                    keys.iter().any(|k| k == "n"),
                                    "{whr}, the hint bar keeps a key named after `n` at full width \
                                     but drops `n`; ranked as a focused-item mnemonic, it outlasts \
                                     every key after it, the inspector and section keys among them.\n\
                                     keys in order: {keys:?}\nat full width: {wide:?}\nhint bar: {line:?}"
                                );
                            } else if keys.iter().any(|k| k == "n")
                                && after_n.iter().any(|k| k != "?")
                            {
                                shed_after_n += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    assert!(named_wide > 0, "fixture: no kitten row was checked");
    assert!(
        beside_an_elder > 0,
        "fixture: no kitten row was checked while another kitten was the Elder"
    );
    assert!(
        shed_after_n > 0,
        "fixture: no width shed the keys after `n` while keeping `n`, so its ranking was never \
         under test"
    );
}

/// While the cursor is on the row of the kitten the core holds as the Elder,
/// the hint bar does not name `n` — at any width, on the bare screen, with the
/// inspector asked for, or with a report banner up — and otherwise names
/// exactly the keys, in the order, that it names on that same row while its
/// kitten is not the Elder. The key itself stays live there: a press still
/// issues AppointElder for the row's kitten, as on every kitten's row
/// (TUI-NAV-094). Quantified over every kitten of the roster as the Elder.
///
/// @spec TUI-NAV-096
#[test]
fn the_hint_bar_leaves_n_unnamed_on_the_elders_own_row_where_the_key_stays_live() {
    let base = every_section_revealed();
    let kittens = kitten_rows(&base);
    assert!(kittens.len() > 1, "fixture: several kitten rows are listed");

    let widths = [WIDE_W, 120, 100, FLOOR_W, 60, 44, 32, 24, 16];
    let mut checked = 0u32;
    for (ix, key) in kittens {
        let mut state = base.clone();
        state.elder = Some(ix);
        assert!(
            state.command_gate_open(&Command::AppointElder(ix)),
            "fixture: the core reports the gate of AppointElder open for the Elder, kitten {ix}"
        );
        let mut elsewhere = base.clone();
        elsewhere.elder = None;

        let bare = focus_on(&state, Section::Clowder, key.clone());
        let contexts: [(&str, ViewState); 3] = [
            ("nothing else", bare.clone()),
            (
                "the inspector asked for",
                ViewState {
                    inspector_wanted: true,
                    ..bare.clone()
                },
            ),
            (
                "a report banner up",
                ViewState {
                    reports: vec![PendingReport {
                        entry: 0,
                        dismissed: false,
                    }],
                    ..bare.clone()
                },
            ),
        ];
        for (what, view) in contexts {
            for w in widths {
                let line = hint_line(&state, &view, w);
                let whr = format!("at {w} columns with the cursor on the row of the Elder, kitten {ix}, and {what}");
                let n_entries: Vec<String> = hint_entries(&line)
                    .into_iter()
                    .filter(|e| {
                        e.split_whitespace().next() == Some("n") || e.contains("appoint Elder")
                    })
                    .collect();
                assert!(
                    n_entries.is_empty(),
                    "{whr}, the hint bar names `n` ({n_entries:?}); on the Elder's own row it \
                     leaves the key unnamed, there being nothing left for it to offer.\nhint bar: {line:?}"
                );
                checked += 1;
            }

            // Where nothing need be dropped, the bar is this row's bar with
            // `n` left out and nothing else changed.
            let on_the_elder = named_keys(&hint_line(&state, &view, WIDE_W));
            let as_any_kitten: Vec<String> = named_keys(&hint_line(&elsewhere, &view, WIDE_W))
                .into_iter()
                .filter(|k| k != "n")
                .collect();
            assert_eq!(
                on_the_elder, as_any_kitten,
                "at {WIDE_W} columns with the cursor on the row of the Elder, kitten {ix}, and \
                 {what}, the hint bar names other keys than it names on that row while its kitten \
                 is not the Elder, `n` aside"
            );
            assert!(
                on_the_elder.iter().any(|k| k == "Enter"),
                "fixture: the focused row's keys are shown, Enter among them: {on_the_elder:?}"
            );
        }

        // Unnamed, never unbound: the press is issued as on any kitten's row.
        let mut view = bare.clone();
        let out = update(Action::AppointElder, &mut view, &state);
        assert_eq!(
            out,
            clowder_tui::nav::Outcome::Apply(Command::AppointElder(ix)),
            "with the cursor on the row of the Elder, kitten {ix}, `n` answered {out:?}; the hint \
             bar leaves the key unnamed there but the key stays live, issuing AppointElder for the \
             row's kitten (TUI-NAV-096, TUI-NAV-094)"
        );
    }
    assert!(checked > 0, "fixture: the Elder's own row was checked");
}

/// The help overlay lists the row-bound key among the Clowder screen's keys,
/// worded `n appoint Elder (kitten)`.
///
/// @spec TUI-NAV-096
#[test]
fn the_help_overlay_lists_n_appoint_elder_kitten_among_the_clowder_keys() {
    let (w, h) = (120u16, 60u16);
    let mut term = terminal(w, h);
    term.draw(|f| {
        let area = f.area();
        overlays::help(f, area);
    })
    .expect("TestBackend is infallible");
    let lines = region_lines(term.backend().buffer(), Rect::new(0, 0, w, h));
    let clowder = lines
        .iter()
        .find(|l| l.trim_start().starts_with(Section::Clowder.title()))
        .unwrap_or_else(|| panic!("the help overlay has no line for the Clowder screen's keys"));
    assert!(
        clowder.contains("n appoint Elder (kitten)"),
        "the help overlay's Clowder line does not list the row-bound key as `n appoint Elder \
         (kitten)`.\nline: {clowder:?}"
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-109 — the Elder's mark in the glyph legend
// ---------------------------------------------------------------------------

/// The help overlay's glyph legend lists `◆` as the mark of the Elder: one
/// legend line headed by the glyph, set out as every other legend line is —
/// the glyph, then its meaning — whose meaning names the Elder, and no other
/// line of the overlay headed by that glyph.
///
/// @spec TUI-NAV-109
#[test]
fn the_help_overlays_glyph_legend_lists_the_elders_mark() {
    let (w, h) = (120u16, 60u16);
    let mut term = terminal(w, h);
    term.draw(|f| {
        let area = f.area();
        overlays::help(f, area);
    })
    .expect("TestBackend is infallible");
    let lines = region_lines(term.backend().buffer(), Rect::new(0, 0, w, h));

    // How the legend sets out a glyph it already lists: the glyph heading its
    // line, blank columns, then the meaning.
    let legend_line = |glyph: char| -> Vec<String> {
        lines
            .iter()
            .filter(|l| l.trim_start().starts_with(glyph))
            .map(|l| l.trim().to_string())
            .collect()
    };
    let cursor = legend_line('▸');
    assert_eq!(
        cursor.len(),
        1,
        "fixture: the legend lists the cursor's glyph on one line: {cursor:?}"
    );

    let elder = legend_line('◆');
    assert_eq!(
        elder.len(),
        1,
        "the help overlay's glyph legend lists `◆` on {} lines ({elder:?}); it lists the glyph \
         once, as the mark of the Elder (TUI-NAV-109)",
        elder.len()
    );
    let meaning = elder[0].trim_start_matches('◆');
    assert!(
        meaning.starts_with(' ') && meaning.contains("Elder"),
        "the legend's line for `◆` reads {:?}; set out as the legend's other lines are, it says \
         the glyph is the mark of the Elder (TUI-NAV-109)",
        elder[0]
    );
    assert_eq!(
        elder[0].chars().take_while(|c| *c == '◆' || *c == ' ').count(),
        cursor[0].chars().take_while(|c| *c == '▸' || *c == ' ').count(),
        "the legend's line for `◆` ({:?}) is not set out as its line for the cursor's glyph is ({:?})",
        elder[0],
        cursor[0]
    );
}

// ---------------------------------------------------------------------------
// TUI-RENDER-019 / TUI-RENDER-022 — the sidebar's lines, and which stores it
// draws
// ---------------------------------------------------------------------------

/// A store's name as its row in the stores group spells it: the first line of
/// the inspector's description of that row (TUI-NAV-018 names the row in
/// full), read in a state where the store is listed.
fn stores_row_name(state: &GameState, r: ResourceId) -> String {
    use clowder_tui::render::widgets::inspector;
    let key = RowKey::Resource(r);
    assert!(
        rows(state, Section::Meadow)
            .iter()
            .any(|row| row.key == key),
        "fixture: {r:?} must be listed in the stores group to be named"
    );
    let (w, h) = (160u16, 12u16);
    let mut view = ViewState {
        section: Some(Section::Meadow),
        ..Default::default()
    };
    view.focus[Section::Meadow.index()] = Some(key);
    let mut term = terminal(w, h);
    term.draw(|f| inspector::draw(f, Rect::new(0, 0, w, h), state, &view))
        .expect("TestBackend is infallible");
    region_lines(term.backend().buffer(), Rect::new(0, 0, w, h))[0]
        .trim()
        .to_string()
}

/// Every resource's stores-row spelling, in vocabulary order.
fn stores_row_names() -> Vec<(ResourceId, String)> {
    let mut state = GameState::new(11);
    for r in ResourceId::ALL {
        state.resources[r.ix()] = 1.0;
    }
    ResourceId::ALL
        .into_iter()
        .map(|r| (r, stores_row_name(&state, r)))
        .collect()
}

/// The narrowest name column at which every two names in the resource
/// vocabulary that differ whole still differ when shortened to it.
fn name_floor(names: &[String]) -> usize {
    (1..=64)
        .find(|w| {
            names.iter().enumerate().all(|(i, a)| {
                names
                    .iter()
                    .skip(i + 1)
                    .all(|b| a == b || fmt::fit(a, *w) != fmt::fit(b, *w))
            })
        })
        .expect("fixture: some width tells every name apart")
}

/// The stores the stores group lists, in its order.
fn listed_stores(state: &GameState) -> Vec<ResourceId> {
    rows(state, Section::Meadow)
        .into_iter()
        .filter_map(|row| match row.key {
            RowKey::Resource(r) => Some(r),
            _ => None,
        })
        .collect()
}

/// Whether a store wears `full` (TUI-NAV-022): a cap above zero, reached.
fn wears_full(state: &GameState, r: ResourceId) -> bool {
    let cap = state.cap(r);
    cap.is_finite() && cap > 0.0 && state.stock(r) >= cap
}

/// Draw the sidebar alone into a rectangle of exactly `w` × `h`.
fn sidebar_in(state: &GameState, w: u16, h: u16) -> (Vec<String>, Buffer) {
    let mut term = terminal(w, h);
    term.draw(|f| chrome::sidebar(f, Rect::new(0, 0, w, h), state))
        .expect("TestBackend is infallible");
    let buf = term.backend().buffer().clone();
    (region_lines(&buf, Rect::new(0, 0, w, h)), buf)
}

/// A store's two lines exactly as TUI-RENDER-019 lays them out in a sidebar
/// `width` columns wide: the name at the left edge, shortened only as far as
/// its column needs, then the stock right-aligned in six columns, `/`, and the
/// cap left-aligned in six — the `/` and cap blank where the core reports the
/// store uncapped; then the bar (read from the drawing, its four cells being
/// TUI-NAV-022's), the rate right-aligned in seven columns, and `full` while
/// capped, a space between each.
fn expected_store_lines(
    state: &GameState,
    r: ResourceId,
    name: &str,
    rate: f64,
    width: usize,
    drawn_bar: &str,
) -> (String, String) {
    let name_col = width - 14;
    let cap = state.cap(r);
    let cap_part = if cap.is_infinite() {
        " ".repeat(7)
    } else {
        format!("/{:<6}", fmt::compact(cap))
    };
    let first = format!(
        "{:<name_col$} {:>6}{cap_part}",
        fmt::fit(name, name_col),
        fmt::compact(state.stock(r)),
    );
    let tag = if wears_full(state, r) { " full" } else { "" };
    let second = format!("{drawn_bar} {:>7}{tag}", fmt::rate(rate));
    (first, format!("{second:<width$}"))
}

/// Stocks that between them reach every width the compact form takes.
const SIDEBAR_STOCKS: [f64; 10] = [
    0.0004, 3.0, 12.5, 999.95, 1234.5, 99_999.0, 1.0e7, 4.2e13, 1.0e20, 0.5,
];

/// Every resource held, at a spread of stocks: some capped stores at their
/// cap, some not, uncapped stores large and small, and zero-cap stores with
/// something in them.
fn every_store_held(seed: usize) -> GameState {
    let mut s = every_section_revealed();
    for (i, r) in ResourceId::ALL.into_iter().enumerate() {
        let cap = s.cap(r);
        s.resources[r.ix()] = if cap.is_finite() && cap > 0.0 && (i + seed).is_multiple_of(3) {
            cap
        } else {
            SIDEBAR_STOCKS[(i + seed) % SIDEBAR_STOCKS.len()]
        };
    }
    s
}

/// The sidebar is wide enough for its bar line — seventeen columns — and for
/// the stock-and-cap pair's thirteen, a blank column, and the narrowest name
/// column at which every two resource names that differ still differ when
/// shortened: at every terminal size the body is drawn.
/// @spec TUI-RENDER-019
#[test]
fn the_sidebar_is_as_wide_as_its_bar_line_and_its_name_floor_beside_the_pair() {
    let names: Vec<String> = stores_row_names().into_iter().map(|(_, n)| n).collect();
    let floor = name_floor(&names);
    let least = 17.max(13 + 1 + floor);
    for w in 80..=320u16 {
        for h in [24u16, 40] {
            let (_, _, side, _) = regions(w, h);
            assert!(
                side.width as usize >= least,
                "at {w}x{h} the sidebar is {} columns; it must be at least {least}: seventeen for \
                 the bar line, and the pair's thirteen, a blank column, and the {floor}-column name \
                 floor (TUI-RENDER-019)",
                side.width
            );
        }
    }
}

/// Every store the sidebar draws is two lines laid out field by field — name,
/// stock, `/`, cap; bar, rate, `full` — so every figure and the tag stand at
/// one column down the sidebar, the bar line is never wider than seventeen
/// columns, and only the name gives way. Quantified over every resource, at
/// stocks reaching every compact width, capped and uncapped and zero-capped,
/// with rates of both signs and none, at the floor and wide.
/// @spec TUI-RENDER-019, TUI-RENDER-012
#[test]
fn every_store_in_the_sidebar_stands_field_by_field_at_one_column() {
    let names: std::collections::HashMap<ResourceId, String> =
        stores_row_names().into_iter().collect();
    let mut signs = [false; 3];
    let mut uncapped_seen = false;
    let mut full_seen = false;
    for seed in 0..3 {
        let mut state = every_store_held(seed);
        // Kittens at work, so some stores gain and some drain.
        state.kittens = employed(6);
        let rates = state.net_rates();
        let listed = listed_stores(&state);
        assert_eq!(
            listed.len(),
            ResourceId::ALL.len(),
            "fixture: every store is listed"
        );
        for w in [FLOOR_W, 130, WIDE_W] {
            let (_, _, side, _) = regions(w, 24);
            let width = side.width as usize;
            let height = (2 * listed.len() + 2) as u16;
            let (lines, _) = sidebar_in(&state, side.width, height);
            for (i, r) in listed.iter().enumerate() {
                let rate = rates[r.ix()];
                signs[if fmt::rate(rate) == "0" {
                    0
                } else if rate > 0.0 {
                    1
                } else {
                    2
                }] = true;
                uncapped_seen |= state.cap(*r).is_infinite();
                full_seen |= wears_full(&state, *r);
                let bar: String = lines[2 * i + 1].chars().take(4).collect();
                assert!(
                    bar.chars().all(|c| c == '▓' || c == '░'),
                    "{r:?} at {w} columns: the second line starts with its four-cell fill bar; it \
                     read {:?}",
                    lines[2 * i + 1]
                );
                let (first, second) =
                    expected_store_lines(&state, *r, &names[r], rate, width, &bar);
                assert_eq!(
                    lines[2 * i], first,
                    "{r:?} at {w} columns (sidebar {width} wide): the name at the left edge, then \
                     the stock right-aligned in six columns, `/`, and the cap left-aligned in six at \
                     the right edge (TUI-RENDER-019)"
                );
                assert_eq!(
                    lines[2 * i + 1], second,
                    "{r:?} at {w} columns (sidebar {width} wide): the bar, the rate right-aligned in \
                     seven columns, and `full` while capped, a space between each (TUI-RENDER-019)"
                );
                assert!(
                    lines[2 * i + 1].trim_end().chars().count() <= 17,
                    "{r:?} at {w} columns: the bar line is wider than seventeen columns"
                );
            }
        }
    }
    assert!(
        signs.iter().all(|s| *s),
        "fixture: rates of every sign and none, got {signs:?}"
    );
    assert!(
        uncapped_seen && full_seen,
        "fixture: uncapped and full stores both drawn"
    );
}

/// The sidebar draws the stores the stores group lists, in its order, each
/// whole or not at all. Where its lines cannot hold every listed store, its
/// last line is `+N more` in `dim`, N written in full, and the stores drawn are
/// as many as the lines above the note hold — chosen first from the stores
/// wearing `full`, then from the rest, each in the stores group's order, and
/// drawn in that order. Checked at every sidebar height from one line to room
/// for all, over meadows whose full stores sit early, late, and nowhere.
/// @spec TUI-RENDER-022
#[test]
fn the_sidebar_draws_whole_stores_full_ones_first_and_counts_the_rest() {
    let names: std::collections::HashMap<ResourceId, String> =
        stores_row_names().into_iter().collect();
    let dim = clowder_tui::theme::Role::Dim.color();
    let (_, _, side, _) = regions(FLOOR_W, FLOOR_H);
    let width = side.width as usize;
    let name_col = width.saturating_sub(14).max(1);

    let mut meadows: Vec<GameState> = (0..3).map(every_store_held).collect();
    // Only the last capped stores full, the rest held below their caps, and a
    // handful of stores held at all.
    let mut late = GameState::new(11);
    let capped: Vec<ResourceId> = ResourceId::ALL
        .into_iter()
        .filter(|r| late.cap(*r).is_finite() && late.cap(*r) > 0.0)
        .collect();
    for r in ResourceId::ALL {
        late.resources[r.ix()] = 0.0;
    }
    for (i, r) in capped.iter().enumerate() {
        late.resources[r.ix()] = if i + 2 >= capped.len() {
            late.cap(*r)
        } else {
            late.cap(*r) / 2.0
        };
    }
    let uncapped: Vec<ResourceId> = ResourceId::ALL
        .into_iter()
        .filter(|r| late.cap(*r).is_infinite())
        .take(5)
        .collect();
    for r in uncapped {
        late.resources[r.ix()] = 7.0;
    }
    meadows.push(late);
    // Nothing full at all.
    let mut none_full = GameState::new(11);
    for r in ResourceId::ALL {
        let cap = none_full.cap(r);
        none_full.resources[r.ix()] = if cap.is_finite() && cap > 0.0 {
            cap / 3.0
        } else {
            2.0
        };
    }
    meadows.push(none_full);

    for state in &meadows {
        let listed = listed_stores(state);
        let k = listed.len();
        assert!(
            k >= 4,
            "fixture: enough stores listed to overflow a short sidebar"
        );
        for height in 1..=(2 * k + 3) {
            let (lines, buf) = sidebar_in(state, side.width, height as u16);
            let drawn_n = if 2 * k <= height { k } else { (height - 1) / 2 };
            let chosen: Vec<ResourceId> = {
                let mut pick: Vec<ResourceId> = listed
                    .iter()
                    .copied()
                    .filter(|r| wears_full(state, *r))
                    .take(drawn_n)
                    .collect();
                let room = drawn_n - pick.len();
                pick.extend(
                    listed
                        .iter()
                        .copied()
                        .filter(|r| !wears_full(state, *r))
                        .take(room),
                );
                listed
                    .iter()
                    .copied()
                    .filter(|r| pick.contains(r))
                    .collect()
            };
            for (i, r) in chosen.iter().enumerate() {
                let head = fmt::fit(&names[r], name_col);
                assert!(
                    lines[2 * i].starts_with(&head),
                    "height {height}: line {} should begin store {i} of those drawn, {r:?}, chosen \
                     first from the stores wearing `full` and kept in the stores group's order \
                     (TUI-RENDER-022); it read {:?}.\n{}",
                    2 * i,
                    lines[2 * i],
                    lines.join("\n")
                );
                assert!(
                    draws_fill_bar(&lines[2 * i + 1]),
                    "height {height}: {r:?} is drawn whole — its second line too (TUI-RENDER-022).\n{}",
                    lines.join("\n")
                );
            }
            let below_stores = &lines[2 * drawn_n..];
            if drawn_n == k {
                assert!(
                    below_stores.iter().all(|l| l.trim().is_empty()),
                    "height {height}: every listed store fits, so nothing follows them.\n{}",
                    lines.join("\n")
                );
                continue;
            }
            let note = format!("+{} more", k - drawn_n);
            let last = lines.last().expect("a sidebar has lines");
            assert_eq!(
                last.trim(),
                note,
                "height {height}: {k} stores listed and {drawn_n} drawn, so the last line is the \
                 note {note:?} (TUI-RENDER-022).\n{}",
                lines.join("\n")
            );
            assert!(
                below_stores[..below_stores.len() - 1]
                    .iter()
                    .all(|l| l.trim().is_empty()),
                "height {height}: only the note stands below the stores drawn — no store is drawn \
                 in part.\n{}",
                lines.join("\n")
            );
            let y = (height - 1) as u16;
            for x in 0..side.width {
                let cell = &buf[(x, y)];
                if !cell.symbol().trim().is_empty() {
                    assert_eq!(
                        cell.fg,
                        dim.unwrap_or(cell.fg),
                        "height {height}: the note wears `dim` (TUI-RENDER-022)"
                    );
                }
            }
            assert!(dim.is_some(), "fixture: dim carries a colour");
        }
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-049 — a store at nothing stays among the stores the sidebar draws
//
// The sidebar draws from the stores the stores group lists, and the stores
// group lists what the core reports revealed — at any stock. So a store spent
// to nothing keeps its two lines where they stood, reading zero, wherever the
// sidebar holds every listed store; and where it cannot, the store is still
// one of those it chooses from and counts.
//
// The meadows here are *played* into their state — a tick, a command — since
// that is where the core records a reveal.
// ---------------------------------------------------------------------------

/// A meadow part-way along: a handful of stores in hand, a tick on, so the
/// core has recorded each. One of them holds exactly the price of a forage.
fn a_few_stores_met() -> GameState {
    let mut s = GameState::new(11);
    s.resources[ResourceId::Catnip.ix()] = content::FORAGE_COST;
    s.resources[ResourceId::Twigs.ix()] = 5.0;
    s.resources[ResourceId::Fluff.ix()] = 3.0;
    s.resources[ResourceId::Pebbles.ix()] = 7.0;
    s.tick();
    assert_eq!(
        s.stock(ResourceId::Catnip),
        content::FORAGE_COST,
        "fixture: nothing in a meadow this bare moves its stores"
    );
    assert_eq!(
        listed_stores(&s).len(),
        4,
        "fixture: four stores met, four listed"
    );
    s
}

/// Every listed store's two lines, at their place in the sidebar, exactly as
/// TUI-RENDER-019 lays them out for the stock the state holds now.
fn assert_every_listed_store_is_drawn(
    state: &GameState,
    listed: &[ResourceId],
    names: &std::collections::HashMap<ResourceId, String>,
    lines: &[String],
    width: usize,
    what: &str,
) {
    assert!(
        lines.len() >= 2 * listed.len(),
        "fixture: the sidebar holds every listed store ({what})"
    );
    let rates = state.net_rates();
    for (i, r) in listed.iter().enumerate() {
        let bar: String = lines[2 * i + 1].chars().take(4).collect();
        assert!(
            draws_fill_bar(&lines[2 * i + 1]),
            "{what}: {r:?} (stock {}) should be drawn whole at lines {} and {}; the sidebar read\n{}",
            state.stock(*r),
            2 * i,
            2 * i + 1,
            lines.join("\n")
        );
        let (first, second) =
            expected_store_lines(state, *r, &names[r], rates[r.ix()], width, &bar);
        assert_eq!(
            lines[2 * i],
            first,
            "{what}: {r:?} (stock {}) should stand at line {} of the sidebar, reading its stock; \
             the sidebar read\n{}",
            state.stock(*r),
            2 * i,
            lines.join("\n")
        );
        assert_eq!(lines[2 * i + 1], second, "{what}: {r:?}'s second line");
    }
}

/// The player's own report, at the floor and wide: a listed store spent to
/// exactly nothing by a command is still listed, and still drawn — both its
/// lines, where they stood, reading zero — with every other store unmoved.
///
/// @spec TUI-NAV-049
#[test]
fn a_store_spent_to_exactly_nothing_stays_in_the_sidebar_where_it_stood() {
    for (w, h) in [(FLOOR_W, FLOOR_H), (WIDE_W, WIDE_H)] {
        let mut state = a_few_stores_met();
        let spent = ResourceId::Catnip;
        let listed = listed_stores(&state);
        let names: std::collections::HashMap<ResourceId, String> = listed
            .iter()
            .map(|r| (*r, stores_row_name(&state, *r)))
            .collect();
        let (_, _, side, _) = regions(w, h);
        let width = side.width as usize;
        assert!(
            2 * listed.len() <= side.height as usize,
            "fixture: at {w}x{h} the sidebar holds every listed store"
        );
        assert!(
            listed.contains(&spent),
            "fixture: {spent:?} is listed before it is spent"
        );

        let before = sidebar_lines(&state, w, h);
        assert_every_listed_store_is_drawn(
            &state,
            &listed,
            &names,
            &before,
            width,
            &format!("{w}x{h}, before the spend"),
        );

        state
            .apply(Command::Forage)
            .expect("fixture: the meadow holds a forage's price");
        assert_eq!(
            state.stock(spent),
            0.0,
            "fixture: the forage spends {spent:?} to exactly nothing"
        );

        assert_eq!(
            listed_stores(&state),
            listed,
            "at {w}x{h}: spending {spent:?} to nothing changed the stores the sidebar draws from"
        );
        let after = sidebar_lines(&state, w, h);
        assert_every_listed_store_is_drawn(
            &state,
            &listed,
            &names,
            &after,
            width,
            &format!("{w}x{h}, after the spend"),
        );
        let at = listed.iter().position(|r| *r == spent).expect("listed");
        assert!(
            after[2 * at].contains(&format!("{:>6}", fmt::compact(0.0))),
            "at {w}x{h}: {spent:?}'s line should read zero, it read {:?}",
            after[2 * at]
        );

        // And it is still there to watch refill, ticks later.
        for _ in 0..20 {
            state.tick();
        }
        assert_eq!(
            state.stock(spent),
            0.0,
            "fixture: nothing refills {spent:?} here"
        );
        let later = sidebar_lines(&state, w, h);
        assert_every_listed_store_is_drawn(
            &state,
            &listed,
            &names,
            &later,
            width,
            &format!("{w}x{h}, ticks later"),
        );
    }
}

/// Where the sidebar cannot hold every listed store, the stores at nothing
/// are still among those it chooses from: the note counts them, and a sidebar
/// with room draws each in its place, reading zero.
///
/// @spec TUI-NAV-049
#[test]
fn stores_at_nothing_stay_among_the_stores_the_sidebar_chooses_from_and_counts() {
    let mut met = every_store_held(0);
    met.tick();
    let listed = listed_stores(&met);
    assert_eq!(
        listed,
        ResourceId::ALL.to_vec(),
        "fixture: every store is met, held, and listed"
    );
    let names: std::collections::HashMap<ResourceId, String> =
        stores_row_names().into_iter().collect();

    let spent: Vec<ResourceId> = listed.iter().copied().step_by(3).collect();
    let mut state = met.clone();
    for r in &spent {
        state.resources[r.ix()] = 0.0;
    }
    assert_eq!(
        listed_stores(&state),
        listed,
        "spending {} stores to nothing took them out of the stores the sidebar draws from",
        spent.len()
    );

    for (w, h) in [(FLOOR_W, FLOOR_H), (WIDE_W, WIDE_H)] {
        let (_, _, side, _) = regions(w, h);
        let height = side.height as usize;
        let k = listed.len();
        assert!(
            2 * k > height,
            "fixture: at {w}x{h} the sidebar cannot hold every store"
        );
        let lines = sidebar_lines(&state, w, h);
        let note = format!("+{} more", k - (height - 1) / 2);
        assert_eq!(
            lines.last().expect("a sidebar has lines").trim(),
            note,
            "at {w}x{h}: {k} stores are listed, {} of them at nothing, so the note counts them \
             all.\n{}",
            spent.len(),
            lines.join("\n")
        );

        let (tall, _) = sidebar_in(&state, side.width, (2 * k) as u16);
        assert_every_listed_store_is_drawn(
            &state,
            &listed,
            &names,
            &tall,
            side.width as usize,
            &format!("a sidebar {} wide with room for all", side.width),
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-122, TUI-NAV-123 — the done key, named
// ---------------------------------------------------------------------------

/// Whether the core names `row`'s item already the player's.
fn is_done(state: &GameState, row: &Row) -> bool {
    match &row.key {
        RowKey::Content(id, _) => state.lock_reason(*id) == Some(LockReason::AlreadyYours),
        _ => false,
    }
}

/// A view on the Ideas screen with the cursor where a player entering it
/// would find it, its done group open or folded.
fn on_ideas(state: &GameState, open: bool) -> ViewState {
    let mut v = calm_view(state);
    update(Action::SectionJump(Section::Ideas), &mut v, state);
    if open {
        press_done_key(state, &mut v);
    }
    v
}

/// Every view of the Ideas screen with nothing holding the hint bar: the
/// cursor on each drawn row, with the group folded and then open.
fn ideas_hint_views(state: &GameState) -> Vec<(bool, ViewState)> {
    let mut out = Vec::new();
    for open in [false, true] {
        let base = on_ideas(state, open);
        out.push((open, base.clone()));
        for row in drawn_rows(state, Section::Ideas, &base)
            .into_iter()
            .filter(|r| r.selectable())
        {
            let mut v = base.clone();
            v.focus[Section::Ideas.index()] = Some(row.key);
            out.push((open, v));
        }
    }
    out
}

/// The every-section meadow with every row of its Ideas screen done.
fn ideas_all_done(state: &GameState) -> GameState {
    let mut s = state.clone();
    s.refinements = clowder_core::RefinementId::ALL.iter().copied().collect();
    let mut paired: Vec<RuleId> = Vec::new();
    for r in RuleId::ALL {
        if !paired.contains(&r) {
            paired.push(r);
            paired.push(content::rule_def(r).blocks);
            s.rules.insert(r);
        }
    }
    assert!(
        rows(&s, Section::Ideas)
            .iter()
            .filter(|r| r.selectable())
            .all(|r| is_done(&s, r)),
        "fixture: every Ideas row is done"
    );
    s
}

/// The every-section meadow with nothing on its Ideas screen done.
fn ideas_none_done(state: &GameState) -> GameState {
    let mut s = state.clone();
    s.ideas = BTreeSet::new();
    s.refinements = BTreeSet::new();
    s.rules = BTreeSet::new();
    let list = rows(&s, Section::Ideas);
    assert!(
        list.iter().any(|r| r.selectable()),
        "fixture: the Ideas screen lists rows"
    );
    assert!(
        !list.iter().any(|r| is_done(&s, r)),
        "fixture: no Ideas row is done"
    );
    s
}

/// Wherever the hint bar shows the focused row's keys and `d` is live, it
/// names the done key — `d show done` while the group is folded, `d fold
/// done` while it is open — after every other context key and directly before
/// the help key: on every drawn row of the Ideas screen, folded and open, and
/// with the cursor resting on no row because every row is folded. Since it is
/// named last, it is the first key the bar drops for width: at every width
/// the bar's keys are the head of the widest bar's.
///
/// @spec TUI-NAV-122
#[test]
fn the_hint_bar_names_the_done_key_last_before_help_wherever_it_is_live() {
    let far = every_section_revealed();
    let all_done = ideas_all_done(&far);
    let mut checked = 0;
    for (label, state) in [("far along", &far), ("every Ideas row done", &all_done)] {
        assert!(
            rows(state, Section::Ideas)
                .iter()
                .any(|r| is_done(state, r)),
            "fixture: {label} has a done group"
        );
        for (open, view) in ideas_hint_views(state) {
            let want = if open { "d fold done" } else { "d show done" };
            let line = hint_line(state, &view, WIDE_W);
            let entries = hint_entries(&line);
            let n = entries.len();
            assert!(
                n >= 2 && entries[n - 2] == want && entries[n - 1] == "? help",
                "{label}, Ideas screen (group open: {open}, cursor {:?}): the hint bar does not name \
                 `{want}` directly before the help key.\nhint bar: {line:?}",
                view.focus[Section::Ideas.index()]
            );
            let wide: Vec<String> = entries[..n - 1].to_vec();
            for w in (20..WIDE_W).step_by(3) {
                let narrow = hint_entries(&hint_line(state, &view, w));
                let kept = &narrow[..narrow.len().saturating_sub(1)];
                assert!(
                    wide.starts_with(kept),
                    "{label}, at {w} columns the hint bar keeps {kept:?}, which is not the head of \
                     {wide:?}; the done key, named last, is the first dropped"
                );
            }
            checked += 1;
        }
    }
    let rested = on_ideas(&all_done, false);
    assert!(
        !drawn_rows(&all_done, Section::Ideas, &rested)
            .iter()
            .any(|r| r.selectable()),
        "with every Ideas row done and the group folded, a row is still drawn (TUI-NAV-113)"
    );
    assert!(checked > 10, "fixture: too few Ideas views were checked");
}

/// The done key is named nowhere it is not live, nor while anything else holds
/// the hint bar: on every other screen with every row focused, on an Ideas
/// screen with nothing done, and on the Ideas screen under an open field, a
/// full-screen overlay, or an armed row.
///
/// @spec TUI-NAV-122
#[test]
fn the_hint_bar_names_the_done_key_nowhere_it_is_not_live() {
    let far = every_section_revealed();
    let named = |line: &str| hint_entries(line).iter().any(|e| e.starts_with("d "));
    for section in Section::ALL.into_iter().filter(|s| *s != Section::Ideas) {
        for row in rows(&far, section).into_iter().filter(|r| r.selectable()) {
            let line = hint_line(&far, &focus_on(&far, section, row.key.clone()), WIDE_W);
            assert!(
                !named(&line),
                "on the {} screen with {:?} focused the hint bar names d: {line:?}",
                section.title(),
                row.key
            );
        }
    }
    let none = ideas_none_done(&far);
    for (_, view) in ideas_hint_views(&none) {
        let line = hint_line(&none, &view, WIDE_W);
        assert!(
            !named(&line),
            "with nothing done on the Ideas screen the hint bar names d: {line:?}"
        );
    }

    let base = on_ideas(&far, false);
    let mut palette = base.clone();
    palette.overlay = Some(Overlay::Palette {
        query: String::new(),
        selected: 0,
    });
    let mut prompt = base.clone();
    prompt.prompt = Some(CountPrompt {
        digits: "3".to_string(),
        ..Default::default()
    });
    let mut help = base.clone();
    help.overlay = Some(Overlay::Help);
    let mut journal = base.clone();
    journal.overlay = Some(Overlay::Journal { anchor: 0 });
    let rule = drawn_rows(&far, Section::Ideas, &base)
        .into_iter()
        .find(|r| r.heavy && !is_done(&far, r))
        .expect("fixture: a rule is drawn to arm");
    let mut armed = base.clone();
    armed.focus[Section::Ideas.index()] = Some(rule.key.clone());
    armed.armed = Some(ArmedRow {
        key: rule.key.clone(),
        command: match rule.kind {
            RowKind::Act { command, .. } => command,
            _ => panic!("fixture: a rule row acts"),
        },
        expires_at: Instant::now() + Duration::from_secs(8),
    });
    for (what, view) in [
        ("the palette", palette),
        ("the count prompt", prompt),
        ("help", help),
        ("the Journal", journal),
        ("an armed row", armed),
    ] {
        let line = hint_line(&far, &view, WIDE_W);
        assert!(
            !named(&line),
            "with {what} holding the hint bar on the Ideas screen it names d: {line:?}"
        );
    }
}

/// The help overlay lists the done key among the Ideas screen's keys, worded
/// `d show or fold done`, and lists `d` among no other screen's keys.
///
/// @spec TUI-NAV-122, TUI-NAV-123
#[test]
fn the_help_overlay_lists_d_show_or_fold_done_among_the_ideas_keys_alone() {
    let (w, h) = (120u16, 60u16);
    let mut term = terminal(w, h);
    term.draw(|f| {
        let area = f.area();
        overlays::help(f, area);
    })
    .expect("TestBackend is infallible");
    let lines = region_lines(term.backend().buffer(), Rect::new(0, 0, w, h));
    let screen_line = |s: Section| {
        lines
            .iter()
            .find(|l| l.trim_start().starts_with(s.title()))
            .cloned()
    };
    let ideas = screen_line(Section::Ideas)
        .unwrap_or_else(|| panic!("the help overlay has no line for the Ideas screen's keys"));
    assert!(
        ideas.contains("d show or fold done"),
        "the help overlay's Ideas line does not list the done key as `d show or fold done`.\nline: {ideas:?}"
    );
    for section in Section::ALL.into_iter().filter(|s| *s != Section::Ideas) {
        if let Some(line) = screen_line(section) {
            let rest = line
                .trim_start()
                .strip_prefix(section.title())
                .unwrap_or("");
            assert!(
                !rest.split('/').any(|e| e.trim_start().starts_with("d ")),
                "the help overlay lists d among the {} screen's keys: {line:?}",
                section.title()
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Coats in the sidebar — TUI-THEME-019, TUI-THEME-020, TUI-THEME-021
// ---------------------------------------------------------------------------

/// The colours a resource's coat puts on each character of `drawn`, read by
/// position in the whole name: a kept character wears its own position's
/// colour, and an ellipsis the colour of the character it stands in place of.
fn coat_on(r: ResourceId, full: &str, drawn: &str) -> Vec<ratatui::style::Color> {
    let colours =
        clowder_tui::theme::coat_colours(clowder_tui::theme::coat(r), full.chars().count());
    drawn.chars().enumerate().map(|(i, _)| colours[i]).collect()
}

fn fg_at(buf: &Buffer, x: u16, y: u16) -> ratatui::style::Color {
    buf.cell((x, y)).map(|c| c.fg).unwrap_or_default()
}

/// Every store the sidebar draws wears its coat on its name — every character,
/// a space included — and on nothing else: its stock, `/`, cap, bar, rate, and
/// blanks wear no colour, and the `full` tag alone wears attention.
///
/// @spec TUI-THEME-019, TUI-THEME-020
#[test]
fn every_store_in_the_sidebar_wears_its_coat_on_its_name_and_nowhere_else() {
    use ratatui::style::Color;
    let state = every_store_held(0);
    let listed = listed_stores(&state);
    assert_eq!(
        listed.len(),
        ResourceId::ALL.len(),
        "fixture: every store held, every store listed"
    );
    let w = 40u16;
    let h = 2 * listed.len() as u16;
    let (lines, buf) = sidebar_in(&state, w, h);
    let name_col = w as usize - 14;

    for (i, r) in listed.iter().enumerate() {
        let full = stores_row_name(&state, *r);
        assert!(
            full.chars().count() <= name_col,
            "fixture: {full:?} fits the name column whole"
        );
        let y = (2 * i) as u16;
        let line = &lines[2 * i];
        assert!(
            line.starts_with(&full),
            "line {y} should begin with {full:?}: {line:?}"
        );

        let expected = coat_on(*r, &full, &full);
        for (x, want) in expected.iter().enumerate() {
            let got = fg_at(&buf, x as u16, y);
            assert_ne!(
                got,
                Color::Reset,
                "{full:?}: character {x} wears no colour at all"
            );
            assert_eq!(
                &got, want,
                "{full:?}: character {x} wears {got:?}, not its coat's {want:?}"
            );
        }
        for x in expected.len()..w as usize {
            let got = fg_at(&buf, x as u16, y);
            assert_eq!(
                got,
                Color::Reset,
                "{full:?}: cell {x} of its name line — a figure, the `/`, or a blank — wears \
                 {got:?}; only the name wears the coat"
            );
        }

        let bar = &lines[2 * i + 1];
        let tag = bar.find("full").map(|at| {
            let start = bar[..at].chars().count();
            start..start + 4
        });
        for x in 0..w as usize {
            let got = fg_at(&buf, x as u16, y + 1);
            if tag.as_ref().is_some_and(|t| t.contains(&x)) {
                assert_eq!(
                    got,
                    attention_colour(),
                    "{full:?}: its `full` tag wears attention"
                );
            } else {
                assert_eq!(
                    got,
                    Color::Reset,
                    "{full:?}: cell {x} of its bar line wears {got:?}; the bar line has no name to coat"
                );
            }
        }
    }
}

/// A store cut to the name column keeps, on every character drawn, the colour
/// that character has in the whole name, and its ellipsis wears the colour of
/// the character it stands in place of — so a store looks like itself whether
/// or not it was cut.
///
/// @spec TUI-THEME-021
#[test]
fn a_store_cut_short_keeps_its_colours_by_position_and_the_ellipsis_wears_its_own() {
    let state = every_store_held(0);
    let listed = listed_stores(&state);
    let w = 18u16;
    let h = 2 * listed.len() as u16;
    let name_col = w as usize - 14;
    let (lines, buf) = sidebar_in(&state, w, h);

    let mut cut = 0;
    for (i, r) in listed.iter().enumerate() {
        let full = stores_row_name(&state, *r);
        let drawn = fmt::fit(&full, name_col);
        if drawn == full {
            continue;
        }
        cut += 1;
        assert!(
            drawn.ends_with('…'),
            "fixture: a cut name ends in the ellipsis: {drawn:?}"
        );
        let y = (2 * i) as u16;
        let line = &lines[2 * i];
        assert!(
            line.starts_with(&drawn),
            "line {y} should begin with {drawn:?}: {line:?}"
        );
        let expected = coat_on(*r, &full, &drawn);
        for (x, want) in expected.iter().enumerate() {
            let got = fg_at(&buf, x as u16, y);
            assert_eq!(
                &got, want,
                "{full:?} cut to {drawn:?}: cell {x} wears {got:?}, not the colour character {x} \
                 of the whole name wears"
            );
        }
    }
    assert!(
        cut > 0,
        "fixture: some name is longer than {name_col} characters"
    );
}
