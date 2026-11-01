//! Section reveal as a sitting lives it: which tabs the bar shows, which of
//! them wear `★`, what the opening of a game takes as known, and what happens
//! to a section that empties while the player stands in it.
//!
//! @spec TUI-NAV-006, TUI-NAV-024, TUI-NAV-085
//!
//! **Why through the loop.** A tab's badge runs from the moment it *appears*
//! in the tab bar — a change between one frame and the next, with no key
//! pressed — until its section is next visited. Appearance is only
//! observable across frames, so every badge claim here is driven through
//! `App::step`, with the game state changed between frames the way a tick, a
//! reveal, or a gate would change it. The frames are taken with a still clock
//! so nothing ticks beneath the test: every change to the state is the test's
//! own.
//!
//! **The opening's seam.** Every sitting opens on the Meadow before its first
//! frame (TUI-NAV-024). `runtime::run` reaches for a real terminal, so it
//! cannot be driven here; its seam is `nav::opening`, which start-up runs on a
//! fresh view and the state the game opened with, before it queues any load
//! report and before the first frame. Everything below that begins a sitting
//! begins it there.
//!
//! **How the tab bar is read.** From what `chrome::tabs` draws, never from the
//! view's own bookkeeping, so these tests hold whatever shape that
//! bookkeeping takes. At a width where every tab is named, a tab is its digit
//! (section names hold no digits); a badge is a `★` attached to the tab's own
//! words — before its digit or after its name.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use clowder_core::{
    content, load_string, save_string, ColonyId, Command, ContentId, GameState, IdeaId, JobId,
    Kitten, KittenTrait, LoadedGame, Location, OfflineReport, ResourceId, SiteId, StructureId,
};
use clowder_tui::nav::rows::rows;
use clowder_tui::nav::{
    opening, update, Action, CountPrompt, Outcome, Overlay, PendingReport, RowKey, Section,
    ViewState,
};
use clowder_tui::render::widgets::chrome;
use clowder_tui::runtime::clock::Reading;
use clowder_tui::runtime::{App, TickAccumulator};
use crossterm::event::{Event as TermEvent, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;

// ---------------------------------------------------------------------------
// Sizes
// ---------------------------------------------------------------------------

/// Wide enough that every tab is drawn with its name, so the bar is read in
/// its full form.
const WIDE_W: u16 = 200;
const WIDE_H: u16 = 60;

/// Below the pane breakpoint, where the inspector swaps in over the list.
const NARROW_W: u16 = 80;
const NARROW_H: u16 = 24;

// ---------------------------------------------------------------------------
// States
// ---------------------------------------------------------------------------

fn kitten(name: &str) -> Kitten {
    Kitten {
        name: name.to_string(),
        job: None,
        location: Location::Home,
        arrived_day: 0,
        job_seq: None,
        away_seq: None,
        skills: [0.0; 8],
        exp: 0.0,
        rank: 0,
        trait_id: KittenTrait::None,
        tinkering: None,
        profile: None,
    }
}

/// A young meadow in which the Sky section's rows can be switched on and off
/// without touching any other section: every site already reached, so none is
/// listed, and the sky's summoning row gated shut. Dreams lists its row, so a
/// tab sits after the Sky's slot in the ring.
fn sky_base() -> GameState {
    let mut s = GameState::new(3);
    s.ideas.insert(IdeaId::SkywardLonging);
    s.ideas.insert(IdeaId::SkywardSong);
    s.ideas.insert(IdeaId::DeepNapTheory);
    s.sites_reached = SiteId::ALL.into_iter().collect();
    s
}

/// The Sky lists nothing.
fn sky_quiet() -> GameState {
    let s = sky_base();
    assert!(
        rows(&s, Section::Sky).iter().all(|r| !r.selectable()),
        "fixture: the quiet sky lists no selectable row"
    );
    assert!(
        rows(&s, Section::Dreams).iter().any(|r| r.selectable()),
        "fixture: a tab sits after the Sky's slot"
    );
    s
}

/// The Sky lists one row: the first site, reachable again.
fn sky_site_only() -> GameState {
    let mut s = sky_base();
    s.sites_reached = BTreeSet::new();
    let keys: Vec<RowKey> = rows(&s, Section::Sky).into_iter().map(|r| r.key).collect();
    assert_eq!(
        keys,
        vec![RowKey::Fixed("sites"), RowKey::Site(SiteId::ALL[0])],
        "fixture: the sky lists the first site alone, beneath its group header"
    );
    s
}

/// The Sky lists one row: the summoning row, its gate open.
fn sky_whale_only() -> GameState {
    let mut s = sky_base();
    s.structures[StructureId::SkyDock.ix()] = 1;
    let keys: Vec<RowKey> = rows(&s, Section::Sky).into_iter().map(|r| r.key).collect();
    assert_eq!(
        keys,
        vec![RowKey::Fixed("things to do"), whale_key()],
        "fixture: the sky lists the summoning row alone, beneath its group header"
    );
    s
}

/// The Sky lists both: the site's row above the summoning row.
fn sky_both() -> GameState {
    let mut s = sky_base();
    s.sites_reached = BTreeSet::new();
    s.structures[StructureId::SkyDock.ix()] = 1;
    let keys: Vec<RowKey> = rows(&s, Section::Sky).into_iter().map(|r| r.key).collect();
    assert_eq!(
        keys,
        vec![
            RowKey::Fixed("sites"),
            RowKey::Site(SiteId::ALL[0]),
            RowKey::Fixed("things to do"),
            whale_key()
        ],
        "fixture: the site's row sits above the summoning row"
    );
    s
}

fn whale_key() -> RowKey {
    RowKey::Fixed("SummonStarWhale")
}

/// A meadow far enough along that every section lists something.
fn every_section_revealed() -> GameState {
    let mut s = GameState::new(7);
    s.ticks = 362 * content::TICKS_PER_DAY;
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
    s.kittens = (0..6)
        .map(|i| Kitten {
            job: Some(JobId::ALL[0]),
            ..kitten(&format!("resident-{i}"))
        })
        .collect();
    s.roster_high_water = 100;
    for section in Section::ALL {
        assert!(
            rows(&s, section).iter().any(|r| r.selectable()),
            "fixture: the {} section should list a row in a meadow this far along",
            section.title()
        );
    }
    s
}

/// A real load of `state`: saved and loaded at the same instant, so the load
/// runs no catch-up and hands back the state as it stood.
fn really_loaded(state: &GameState) -> LoadedGame {
    let text = save_string(state, 1_000_000);
    load_string(&text, 1_000_000).expect("fixture: a save this binary wrote loads")
}

/// What the core's loader returns after retaining a catch-up report over a
/// long absence — the report kept in the journal at the tick the catch-up
/// ended on, and handed back beside the state.
fn loaded_with_report(mut state: GameState) -> LoadedGame {
    let mut report = OfflineReport {
        elapsed_seconds: 3 * 86_400,
        ..OfflineReport::default()
    };
    report.resource_deltas[ResourceId::ALL[0].ix()] = 480.0;
    state.journal.record_away(state.ticks, &report);
    LoadedGame { state, report }
}

// ---------------------------------------------------------------------------
// The loop
// ---------------------------------------------------------------------------

fn save_path(name: &str) -> PathBuf {
    std::env::temp_dir()
        .join(format!("clowder-reveal-{}-{name}", std::process::id()))
        .join("save.json")
}

/// A sitting opened on `state` exactly as start-up opens one.
fn opened(state: GameState, name: &str) -> App {
    let mut view = ViewState::default();
    opening(&mut view, &state);
    App {
        state,
        view,
        clock: TickAccumulator::default(),
        save_path: save_path(name),
        last_save: Instant::now(),
        prev_layout: None,
        pointer_record: None,
        quitting: false,
        save_failed: false,
    }
}

fn terminal(w: u16, h: u16) -> Terminal<TestBackend> {
    Terminal::new(TestBackend::new(w, h)).expect("TestBackend is infallible")
}

fn still() -> Reading {
    Reading {
        monotonic: Duration::ZERO,
        wall: Duration::ZERO,
    }
}

/// One frame with the clock held still: whatever changed, the test changed.
fn frame(app: &mut App, term: &mut Terminal<TestBackend>, input: &[TermEvent]) {
    app.step(term, still(), input)
        .expect("a step against a TestBackend does not fail");
}

fn press(code: KeyCode) -> TermEvent {
    TermEvent::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn press_char(c: char) -> TermEvent {
    let modifiers = if c.is_ascii_uppercase() {
        KeyModifiers::SHIFT
    } else {
        KeyModifiers::NONE
    };
    TermEvent::Key(KeyEvent::new(KeyCode::Char(c), modifiers))
}

fn digit(section: Section) -> TermEvent {
    press_char(section.digit())
}

// ---------------------------------------------------------------------------
// Reading the tab bar
// ---------------------------------------------------------------------------

fn region_line(buf: &Buffer, w: u16) -> String {
    (0..w)
        .map(|x| buf.cell((x, 0)).map(|c| c.symbol()).unwrap_or(" "))
        .collect()
}

fn tab_line(state: &GameState, view: &ViewState) -> String {
    let mut term = terminal(WIDE_W, 1);
    term.draw(|f| {
        let area = f.area();
        chrome::tabs(f, area, state, view);
    })
    .expect("TestBackend is infallible");
    region_line(term.backend().buffer(), WIDE_W)
}

/// The tabs the bar draws, in the order drawn, each as (section, badged).
fn tab_bar(state: &GameState, view: &ViewState) -> Vec<(Section, bool)> {
    let line = tab_line(state, view);
    let mut out: Vec<(Section, bool)> = Vec::new();
    let mut pending_star = false;
    for token in line.split_whitespace() {
        let starred = token.contains('★');
        let digit = token.chars().find(|c| c.is_ascii_digit());
        match digit {
            Some(d) => {
                let section = Section::from_digit(d).unwrap_or_else(|| {
                    panic!("the tab bar drew {d:?}, which is no section's digit")
                });
                out.push((section, starred || pending_star));
                pending_star = false;
            }
            None if token == "★" => pending_star = true,
            None if starred => {
                if let Some(last) = out.last_mut() {
                    last.1 = true;
                }
            }
            None => {}
        }
    }
    out
}

fn tabs_shown(state: &GameState, view: &ViewState) -> Vec<Section> {
    tab_bar(state, view).into_iter().map(|(s, _)| s).collect()
}

fn badged(state: &GameState, view: &ViewState) -> Vec<Section> {
    tab_bar(state, view)
        .into_iter()
        .filter(|(_, b)| *b)
        .map(|(s, _)| s)
        .collect()
}

/// The sections that list a selectable row, in digit order — what the bar
/// shows for them before focus adds its one tab.
fn listing(state: &GameState) -> Vec<Section> {
    Section::ALL
        .into_iter()
        .filter(|s| rows(state, *s).iter().any(|r| r.selectable()))
        .collect()
}

fn header_line(state: &GameState, view: &ViewState) -> String {
    let mut term = terminal(WIDE_W, 1);
    term.draw(|f| {
        let area = f.area();
        chrome::header(f, area, state, view);
    })
    .expect("TestBackend is infallible");
    region_line(term.backend().buffer(), WIDE_W)
}

/// The new-content chip is its glyph beside its word.
fn new_chip_shown(header: &str) -> bool {
    header.contains('★') && header.to_ascii_lowercase().contains("new")
}

// ===========================================================================
// TUI-NAV-024 — the opening
// ===========================================================================

/// Every sitting — a new game, a load of a young meadow, a load of one far
/// along — opens on the Meadow with the cursor on the gather row, before a
/// frame is drawn, so the first Enter already has a row to act on.
///
/// @spec TUI-NAV-024
#[test]
fn every_sitting_opens_on_the_meadow_with_the_cursor_on_the_gather_row() {
    let sittings: Vec<(&str, GameState)> = vec![
        ("a new game", GameState::new(3)),
        (
            "a load of a young meadow",
            really_loaded(&sky_site_only()).state,
        ),
        (
            "a load of a meadow far along",
            really_loaded(&every_section_revealed()).state,
        ),
        (
            "a load whose catch-up kept a report",
            loaded_with_report(every_section_revealed()).state,
        ),
    ];
    for (what, state) in sittings {
        let mut view = ViewState::default();
        opening(&mut view, &state);

        assert_eq!(
            view.section,
            Some(Section::Meadow),
            "{what} opened on {:?}; every sitting opens on the Meadow, home, the one section \
             certain to have a tab",
            view.section
        );
        assert_eq!(
            view.focus[Section::Meadow.index()],
            Some(RowKey::Fixed("GatherCatnip")),
            "{what} opened with the cursor somewhere other than the gather row"
        );
        let outcome = update(Action::Activate, &mut view.clone(), &state);
        assert_eq!(
            outcome,
            Outcome::Apply(Command::GatherCatnip),
            "on {what}, the first Enter does not act on the gather row the opening landed on"
        );
    }
}

/// The opening takes what it finds as known. No tab present at the opening
/// wears `★`, and the new-content chip counts nothing then listed — on the
/// opened view before any frame, and on the first frame drawn — whether the
/// sitting holds one tab or ten.
///
/// @spec TUI-NAV-024, TUI-NAV-006
#[test]
fn the_opening_badges_no_tab_and_counts_nothing_it_finds_as_new() {
    let sittings: Vec<(&str, GameState)> = vec![
        ("a new game", GameState::new(3)),
        (
            "a load of a young meadow",
            really_loaded(&sky_site_only()).state,
        ),
        (
            "a load of a meadow far along",
            really_loaded(&every_section_revealed()).state,
        ),
        (
            "a load whose catch-up kept a report",
            loaded_with_report(every_section_revealed()).state,
        ),
    ];
    for (what, state) in sittings {
        let mut app = opened(state, "opening-known");

        let check = |app: &App, when: &str| {
            let shown = tabs_shown(&app.state, &app.view);
            assert_eq!(
                shown,
                listing(&app.state),
                "fixture: on {what} {when}, the bar shows exactly the listing sections"
            );
            let marked = badged(&app.state, &app.view);
            assert!(
                marked.is_empty(),
                "on {what} {when}, the tabs {:?} wear ★ though they were all present when the game \
                 opened; the opening takes what it finds as known.\ntab bar: {:?}",
                marked.iter().map(|s| s.digit()).collect::<Vec<_>>(),
                tab_line(&app.state, &app.view)
            );
            let header = header_line(&app.state, &app.view);
            assert!(
                !new_chip_shown(&header),
                "on {what} {when}, the new-content chip counts what the sections listed at the \
                 opening as new.\nheader: {header:?}"
            );
        };

        check(&app, "before the first frame");
        let mut term = terminal(WIDE_W, WIDE_H);
        frame(&mut app, &mut term, &[]);
        check(&app, "on the first frame");
        frame(&mut app, &mut term, &[]);
        check(&app, "on the second frame");
    }
}

/// On a new game the Meadow is the only section with a tab, and it is not
/// badged: the first screen is one section with one focused row.
///
/// @spec TUI-NAV-024
#[test]
fn a_new_game_opens_with_the_meadows_tab_alone_and_unbadged() {
    let mut app = opened(GameState::new(3), "new-game-one-tab");
    let mut term = terminal(WIDE_W, WIDE_H);
    frame(&mut app, &mut term, &[]);
    assert_eq!(
        tab_bar(&app.state, &app.view),
        vec![(Section::Meadow, false)],
        "a new game's first frame shows a tab bar other than the Meadow's one unbadged tab: {:?}",
        tab_line(&app.state, &app.view)
    );
}

/// The opening is not a navigation key: it retires no report banner, so a
/// load's report is still unread when the returning player first looks —
/// whichever of the two the start-up does first.
///
/// @spec TUI-NAV-024
#[test]
fn the_opening_retires_no_report_banner() {
    let loaded = loaded_with_report(every_section_revealed());

    // The report queued first, then the opening.
    let mut queued_first = ViewState::default();
    clowder_tui::nav::queue_load_report(&mut queued_first, &loaded);
    assert!(
        queued_first.reports.iter().any(|r| !r.dismissed),
        "fixture: the load's report queues a banner"
    );
    let before = format!("{:?}", queued_first.reports);
    opening(&mut queued_first, &loaded.state);
    assert_eq!(
        format!("{:?}", queued_first.reports),
        before,
        "the opening retired or reordered a queued report banner; it is not a navigation key"
    );

    // The opening first, then the report — as start-up runs them.
    let mut opened_first = ViewState::default();
    opening(&mut opened_first, &loaded.state);
    clowder_tui::nav::queue_load_report(&mut opened_first, &loaded);
    assert!(
        opened_first.reports.iter().any(|r| !r.dismissed),
        "the load's report is not unread after the opening"
    );

    // And the first frame leaves it unread too.
    let mut app = opened(loaded.state.clone(), "opening-banner");
    clowder_tui::nav::queue_load_report(&mut app.view, &loaded);
    let mut term = terminal(WIDE_W, WIDE_H);
    frame(&mut app, &mut term, &[]);
    assert!(
        app.view.reports.iter().any(|r| !r.dismissed),
        "the load's report was read before the returning player pressed anything"
    );
}

// ===========================================================================
// TUI-NAV-006 — the badge
// ===========================================================================

const PROBE: &str = "Quillifer Probe";

/// A tab that appears during play wears `★` until its section is visited —
/// by its digit, by any of the section-cycle keys, or by the palette's jump —
/// and not before: frames passing and keys that do not enter the section
/// leave it badged. Only the tab that appeared is badged.
///
/// @spec TUI-NAV-006
#[test]
fn a_tab_that_appears_during_play_is_badged_until_any_entry_into_its_section() {
    type Route = (&'static str, Vec<TermEvent>);
    let mut palette_jump = vec![press_char('/')];
    palette_jump.extend(PROBE.chars().map(press_char));
    palette_jump.push(press(KeyCode::Enter));
    let routes: Vec<Route> = vec![
        ("its digit", vec![digit(Section::Clowder)]),
        ("Tab", vec![press(KeyCode::Tab)]),
        ("BackTab", vec![press(KeyCode::BackTab)]),
        ("l", vec![press_char('l')]),
        ("h", vec![press_char('h')]),
        ("→", vec![press(KeyCode::Right)]),
        ("←", vec![press(KeyCode::Left)]),
        ("the palette's jump", palette_jump),
    ];

    for (route, keys) in routes {
        let mut app = opened(GameState::new(3), "badge-routes");
        let mut term = terminal(WIDE_W, WIDE_H);
        frame(&mut app, &mut term, &[]);
        let before = tabs_shown(&app.state, &app.view);
        assert_eq!(
            before,
            vec![Section::Meadow],
            "fixture: a new game shows the Meadow's tab alone"
        );

        // A kitten arrives: the Clowder lists its rows, and its tab appears.
        app.state.kittens.push(kitten(PROBE));
        frame(&mut app, &mut term, &[]);
        let after = tabs_shown(&app.state, &app.view);
        assert_eq!(
            after,
            vec![Section::Meadow, Section::Clowder],
            "fixture: the arrival brings the Clowder's tab and nothing else"
        );
        assert_eq!(
            badged(&app.state, &app.view),
            vec![Section::Clowder],
            "the Clowder's tab appeared during play and the bar badges {:?} instead of it alone.\n\
             tab bar: {:?}",
            badged(&app.state, &app.view),
            tab_line(&app.state, &app.view)
        );

        // Time passing and keys that do not enter the section leave it badged.
        frame(&mut app, &mut term, &[]);
        frame(&mut app, &mut term, &[press_char('j'), press_char('k')]);
        frame(&mut app, &mut term, &[digit(Section::Meadow)]);
        assert_eq!(
            badged(&app.state, &app.view),
            vec![Section::Clowder],
            "the Clowder's badge cleared before its section was visited.\ntab bar: {:?}",
            tab_line(&app.state, &app.view)
        );

        frame(&mut app, &mut term, &keys);
        assert_eq!(
            app.view.section,
            Some(Section::Clowder),
            "fixture: {route} enters the Clowder from the Meadow"
        );
        assert!(
            badged(&app.state, &app.view).is_empty(),
            "entering the Clowder by {route} left its tab badged; a visit is any entry into the \
             section.\ntab bar: {:?}",
            tab_line(&app.state, &app.view)
        );

        // Visited stays visited: leaving and further frames do not re-badge.
        frame(
            &mut app,
            &mut term,
            &[press(KeyCode::Esc), digit(Section::Meadow)],
        );
        frame(&mut app, &mut term, &[]);
        assert!(
            badged(&app.state, &app.view).is_empty(),
            "the Clowder's tab was badged again with nothing having left or entered the bar \
             (route: {route}).\ntab bar: {:?}",
            tab_line(&app.state, &app.view)
        );
    }
}

/// Every section's tab is badged when it appears, and each clears on its own
/// visit alone: nine tabs arriving at once wear nine badges, and visiting one
/// clears only that one.
///
/// @spec TUI-NAV-006
#[test]
fn every_tab_that_appears_is_badged_and_each_clears_on_its_own_visit_alone() {
    let mut app = opened(GameState::new(3), "badge-all");
    let mut term = terminal(WIDE_W, WIDE_H);
    frame(&mut app, &mut term, &[]);

    app.state = every_section_revealed();
    frame(&mut app, &mut term, &[]);

    let arrived: Vec<Section> = Section::ALL
        .into_iter()
        .filter(|s| *s != Section::Meadow)
        .collect();
    assert_eq!(
        tabs_shown(&app.state, &app.view),
        Section::ALL.to_vec(),
        "fixture: every section's tab is in the bar"
    );
    assert_eq!(
        badged(&app.state, &app.view),
        arrived,
        "nine tabs appeared at once and the bar badges a different set; the Meadow's tab never \
         left and every other tab is new.\ntab bar: {:?}",
        tab_line(&app.state, &app.view)
    );

    for (i, section) in arrived.iter().enumerate() {
        frame(&mut app, &mut term, &[digit(*section)]);
        assert_eq!(
            app.view.section,
            Some(*section),
            "fixture: the digit enters {}",
            section.title()
        );
        let still_badged: Vec<Section> = arrived[i + 1..].to_vec();
        assert_eq!(
            badged(&app.state, &app.view),
            still_badged,
            "after visiting the {} section, the badges read differently from every tab not yet \
             visited.\ntab bar: {:?}",
            section.title(),
            tab_line(&app.state, &app.view)
        );
    }
}

/// A tab that leaves the bar and returns is badged again, even when it
/// returns with exactly the rows it left with: its return is a change to the
/// bar the player has not yet looked at.
///
/// @spec TUI-NAV-006
#[test]
fn a_tab_that_leaves_and_returns_is_badged_again() {
    let mut app = opened(sky_quiet(), "badge-return");
    let mut term = terminal(WIDE_W, WIDE_H);
    frame(&mut app, &mut term, &[]);
    assert!(
        !tabs_shown(&app.state, &app.view).contains(&Section::Sky),
        "fixture: no sky tab at the opening"
    );

    app.state = sky_site_only();
    frame(&mut app, &mut term, &[]);
    assert_eq!(
        badged(&app.state, &app.view),
        vec![Section::Sky],
        "the Sky's tab appeared and is not the one badge on the bar.\ntab bar: {:?}",
        tab_line(&app.state, &app.view)
    );
    frame(&mut app, &mut term, &[digit(Section::Sky)]);
    frame(&mut app, &mut term, &[digit(Section::Meadow)]);
    assert!(
        badged(&app.state, &app.view).is_empty(),
        "fixture: the visit cleared the Sky's badge"
    );

    // Its row leaves while the player is elsewhere: the tab leaves with it.
    app.state = sky_quiet();
    frame(&mut app, &mut term, &[]);
    assert!(
        !tabs_shown(&app.state, &app.view).contains(&Section::Sky),
        "the Sky's tab stayed in the bar with nothing listed and the section out of focus.\n\
         tab bar: {:?}",
        tab_line(&app.state, &app.view)
    );

    // The same row returns.
    app.state = sky_site_only();
    frame(&mut app, &mut term, &[]);
    assert_eq!(
        badged(&app.state, &app.view),
        vec![Section::Sky],
        "the Sky's tab returned to the bar unbadged; a tab is badged each time it appears, \
         whatever rows it returns with.\ntab bar: {:?}",
        tab_line(&app.state, &app.view)
    );
}

/// The tab bar is re-derived from current state and focus: one state and one
/// focus show the same tabs whether the state was reached live — a section
/// visited, then emptied out of focus — or opened fresh from a load.
///
/// @spec TUI-NAV-006
#[test]
fn the_same_state_and_focus_show_the_same_tabs_live_and_loaded() {
    // Live: the Sky listed, visited, left, then emptied.
    let mut live = opened(sky_site_only(), "rederive-live");
    let mut term = terminal(WIDE_W, WIDE_H);
    frame(&mut live, &mut term, &[digit(Section::Sky)]);
    frame(&mut live, &mut term, &[digit(Section::Meadow)]);
    live.state = sky_quiet();
    frame(&mut live, &mut term, &[]);

    // Loaded: the same state, opened fresh.
    let mut loaded = opened(really_loaded(&sky_quiet()).state, "rederive-loaded");
    let mut term2 = terminal(WIDE_W, WIDE_H);
    frame(&mut loaded, &mut term2, &[]);

    assert_eq!(
        live.view.section, loaded.view.section,
        "fixture: both focus the Meadow"
    );
    assert_eq!(
        tabs_shown(&live.state, &live.view),
        tabs_shown(&loaded.state, &loaded.view),
        "one state and one focus show different tabs live and loaded; a tab kept once it had \
         appeared is a reveal held in view state and lost on every close"
    );
    assert_eq!(
        tabs_shown(&live.state, &live.view),
        listing(&live.state),
        "with the Meadow in focus, the bar shows exactly the sections that list a row"
    );
}

// ===========================================================================
// TUI-NAV-085 — a focused section that empties
// ===========================================================================

/// The player's view, opened on a sky that lists `start`, standing in the Sky.
fn standing_in_the_sky(
    start: GameState,
    name: &str,
    w: u16,
    h: u16,
) -> (App, Terminal<TestBackend>) {
    let mut app = opened(start, name);
    let mut term = terminal(w, h);
    frame(&mut app, &mut term, &[digit(Section::Sky)]);
    assert_eq!(
        app.view.section,
        Some(Section::Sky),
        "fixture: the digit enters the Sky"
    );
    (app, term)
}

/// Everything 085 says stands as it was, read off the view.
fn standing(view: &ViewState) -> String {
    format!(
        "section={:?} reports={:?} overlay={:?} prompt={:?} inspector={:?} journal_pinned={:?}",
        view.section,
        view.reports,
        view.overlay,
        view.prompt,
        view.inspector_wanted,
        view.journal_pinned
    )
}

/// When the section in focus stops listing any row, it stays in focus and
/// keeps its tab, frame after frame, with nothing moving on its own: the
/// report banner, an open overlay or field, and the layout all stand as they
/// were — at a width with the inspector as a pane and at one where it swaps
/// in over the list.
///
/// @spec TUI-NAV-085, TUI-NAV-006
#[test]
fn a_focused_section_that_empties_keeps_focus_and_its_tab_and_everything_else_stands() {
    type Setup = fn(&mut ViewState);
    let contexts: Vec<(&str, Setup)> = vec![
        ("a bare screen", |_| {}),
        ("a report banner showing", |v| {
            v.reports = vec![PendingReport {
                entry: 0,
                dismissed: false,
            }]
        }),
        ("the help overlay open", |v| v.overlay = Some(Overlay::Help)),
        ("the Journal open", |v| {
            v.overlay = Some(Overlay::Journal { anchor: 0 })
        }),
        ("the palette open", |v| {
            v.overlay = Some(Overlay::Palette {
                query: "zz".to_string(),
                selected: 0,
            })
        }),
        ("a count prompt open on the sky's row", |v| {
            v.prompt = Some(CountPrompt {
                digits: "3".to_string(),
                row: Some(RowKey::Site(SiteId::ALL[0])),
            })
        }),
        ("the inspector called for", |v| v.inspector_wanted = true),
    ];

    for (w, h) in [(WIDE_W, WIDE_H), (NARROW_W, NARROW_H)] {
        for (what, setup) in &contexts {
            let (mut app, mut term) = standing_in_the_sky(sky_site_only(), "empties-stands", w, h);
            setup(&mut app.view);
            frame(&mut app, &mut term, &[]);
            let before = standing(&app.view);
            let layout_before = app.prev_layout;

            app.state = sky_quiet();
            for n in 1..=3 {
                frame(&mut app, &mut term, &[]);
                assert_eq!(
                    app.view.section,
                    Some(Section::Sky),
                    "at {w} columns with {what}, the Sky emptied and focus moved on its own to \
                     {:?} by frame {n}",
                    app.view.section
                );
                assert!(
                    tabs_shown(&app.state, &app.view).contains(&Section::Sky),
                    "at {w} columns with {what}, the Sky emptied while in focus and its tab left \
                     the bar by frame {n}; the section in focus keeps its tab.\ntab bar: {:?}",
                    tab_line(&app.state, &app.view)
                );
                assert_eq!(
                    standing(&app.view),
                    before,
                    "at {w} columns with {what}, the Sky emptying changed what was standing by \
                     frame {n}"
                );
                assert_eq!(
                    app.prev_layout, layout_before,
                    "at {w} columns with {what}, the Sky emptying changed the layout by frame {n}"
                );
            }
        }
    }
}

/// Once the emptied section is out of focus it loses its tab: a section cycle
/// passes over it both ways round, and its digit answers as any section's
/// without a tab does — until a row returns, which brings the tab back.
///
/// @spec TUI-NAV-085, TUI-NAV-006
#[test]
fn an_empty_section_out_of_focus_loses_its_tab_until_a_row_returns() {
    let (mut app, mut term) =
        standing_in_the_sky(sky_site_only(), "empties-leaves", WIDE_W, WIDE_H);
    app.state = sky_quiet();
    frame(&mut app, &mut term, &[]);
    frame(&mut app, &mut term, &[digit(Section::Meadow)]);

    let ring = listing(&app.state);
    assert!(
        !ring.contains(&Section::Sky),
        "fixture: the quiet sky lists nothing"
    );
    assert_eq!(
        tabs_shown(&app.state, &app.view),
        ring,
        "the Sky, empty and out of focus, is still in the bar.\ntab bar: {:?}",
        tab_line(&app.state, &app.view)
    );

    // A section cycle passes over it, both ways round the whole ring.
    for key in [KeyCode::Tab, KeyCode::BackTab] {
        let mut landed = Vec::new();
        for _ in 0..ring.len() * 2 {
            frame(&mut app, &mut term, &[press(key)]);
            landed.push(app.view.section.expect("a section is always in focus"));
        }
        assert!(
            !landed.contains(&Section::Sky),
            "{key:?} cycled into the Sky, which has no tab: {landed:?}"
        );
        for s in &ring {
            assert!(
                landed.contains(s),
                "fixture: {key:?} reaches the {} tab",
                s.title()
            );
        }
    }
    frame(&mut app, &mut term, &[digit(Section::Meadow)]);

    // Its digit answers as the digit of a section that never had a tab.
    let never = Section::Legacy;
    assert!(
        !ring.contains(&never),
        "fixture: the Legacy section lists nothing here"
    );
    app.view.footer = None;
    frame(&mut app, &mut term, &[digit(never)]);
    let never_note = app.view.footer.clone().map(|(m, _)| m);
    assert!(
        never_note.is_some(),
        "fixture: a section without a tab answers its digit with a note"
    );
    assert_eq!(
        app.view.section,
        Some(Section::Meadow),
        "fixture: the no-tab digit moves nothing"
    );

    app.view.footer = None;
    frame(&mut app, &mut term, &[digit(Section::Sky)]);
    assert_eq!(
        app.view.section,
        Some(Section::Meadow),
        "the Sky's digit entered a section with nothing listed and no tab"
    );
    assert_eq!(
        app.view.footer.clone().map(|(m, _)| m),
        never_note,
        "the Sky's digit, out of focus with nothing listed, answered differently from any other \
         section without a tab"
    );

    // A row returns: the tab comes back, badged, and its digit enters it.
    app.state = sky_site_only();
    frame(&mut app, &mut term, &[]);
    assert!(
        tab_bar(&app.state, &app.view).contains(&(Section::Sky, true)),
        "a row returned to the Sky and its tab did not come back badged.\ntab bar: {:?}",
        tab_line(&app.state, &app.view)
    );
    frame(&mut app, &mut term, &[digit(Section::Sky)]);
    assert_eq!(
        app.view.section,
        Some(Section::Sky),
        "the returned tab's digit does not enter it"
    );
}

/// The emptied section in focus is on the ring while it holds focus, so a
/// section cycle leaves it for its neighbour by digit order — the next tab
/// after its slot going forward, the one before going back — as from any
/// section with a tab.
///
/// @spec TUI-NAV-085, TUI-NAV-006
#[test]
fn a_section_cycle_leaves_the_emptied_focused_section_for_its_neighbours_by_digit() {
    for (key, forward) in [(KeyCode::Tab, true), (KeyCode::BackTab, false)] {
        let (mut app, mut term) =
            standing_in_the_sky(sky_site_only(), "empties-cycle", WIDE_W, WIDE_H);
        app.state = sky_quiet();
        frame(&mut app, &mut term, &[]);

        let mut ring = listing(&app.state);
        ring.push(Section::Sky);
        ring.sort();
        let at = ring
            .iter()
            .position(|s| *s == Section::Sky)
            .expect("pushed above");
        let n = ring.len();
        let expected = if forward {
            ring[(at + 1) % n]
        } else {
            ring[(at + n - 1) % n]
        };

        frame(&mut app, &mut term, &[press(key)]);
        assert_eq!(
            app.view.section,
            Some(expected),
            "{key:?} from the emptied Sky landed on {:?}; the Sky's tab is in the bar while it holds \
             focus, so the cycle steps to its neighbour in digit order",
            app.view.section
        );
    }
}

/// When a row returns to the section while it still holds focus, the cursor
/// lands as it does on entering a section: on the remembered item where it
/// has returned, even with a row now listed above it; else by the landing
/// rule. The tab never left the bar, so it wears no badge.
///
/// @spec TUI-NAV-085, TUI-NAV-006
#[test]
fn a_row_returning_to_the_focused_section_takes_the_cursor_as_entering_would() {
    // The remembered row returns, a row inserted above it.
    let (mut app, mut term) =
        standing_in_the_sky(sky_whale_only(), "returns-remembered", WIDE_W, WIDE_H);
    assert_eq!(
        app.view.focus[Section::Sky.index()],
        Some(whale_key()),
        "fixture: the cursor is on the summoning row"
    );
    app.state = sky_quiet();
    frame(&mut app, &mut term, &[]);
    app.state = sky_both();
    frame(&mut app, &mut term, &[]);
    assert_eq!(
        app.view.section,
        Some(Section::Sky),
        "fixture: the Sky held focus throughout"
    );
    assert_eq!(
        app.view.focus[Section::Sky.index()],
        Some(whale_key()),
        "the remembered row returned and the cursor is not on it; a section remembers its item by \
         identity, not by index"
    );
    assert_eq!(
        update(Action::Activate, &mut app.view.clone(), &app.state),
        Outcome::Apply(Command::SummonStarWhale),
        "Enter after the remembered row returned does not act on it"
    );
    assert!(
        badged(&app.state, &app.view).is_empty(),
        "the Sky's tab never left the bar while it held focus, yet it wears ★.\ntab bar: {:?}",
        tab_line(&app.state, &app.view)
    );

    // A different row returns: the landing rule places the cursor.
    let (mut app, mut term) =
        standing_in_the_sky(sky_whale_only(), "returns-other", WIDE_W, WIDE_H);
    app.state = sky_quiet();
    frame(&mut app, &mut term, &[]);
    app.state = sky_site_only();
    frame(&mut app, &mut term, &[]);
    let site = RowKey::Site(SiteId::ALL[0]);
    assert_eq!(
        app.view.focus[Section::Sky.index()],
        Some(site.clone()),
        "a row returned to the focused Sky in place of the remembered one, and the cursor did not \
         land on it by the landing rule"
    );
    assert_eq!(
        update(Action::Activate, &mut app.view.clone(), &app.state),
        Outcome::Apply(Command::LaunchExpedition(SiteId::ALL[0])),
        "Enter after a row returned to the focused Sky does not act on the row the cursor landed on"
    );
    assert!(
        badged(&app.state, &app.view).is_empty(),
        "the Sky's tab never left the bar while it held focus, yet it wears ★.\ntab bar: {:?}",
        tab_line(&app.state, &app.view)
    );
}
