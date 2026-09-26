//! The returning player's report banners as a session lives them: which
//! reports queue a banner, the order they show in, what reads one, what
//! retires the one showing and what leaves it up, and where each report's
//! pinned Journal entry sits before and after it is read.
//!
//! @spec TUI-NAV-027, TUI-NAV-050, TUI-NAV-055, TUI-NAV-083, TUI-NAV-084
//!
//! **Why through the loop.** A report reaches the queue two ways: a load's
//! catch-up the core kept, and an accepted nap. The nap's route runs through
//! `App::step` — the loop is the only place a command's result is seen — so
//! naps are taken here by a real Enter on an armed nap row. The load's route
//! runs through `runtime::run`, which reaches for a real terminal; its seam is
//! `nav::queue_load_report`, which the loop's start-up calls with what the
//! core's loader returned. Every key is pressed through `App::step` too, so
//! what a key means is resolved exactly as it is in play, at the width the
//! terminal has — which is what makes the narrow-mode inspector swap real.
//!
//! **How a banner is identified.** By what the frame draws in the banner row,
//! never by the view's own bookkeeping, so the tests hold whatever shape the
//! queue takes. Two reports whose lines differ are queued (a load's and a
//! nap's), and "the next unread one came up" is the second line appearing
//! where the first was.
//!
//! **How a pinned entry is located.** By a name only it carries — an arrival
//! named in the load's report — or, for a nap's report, by the lines the
//! Journal draws for that report alone. The events around a report are
//! arrivals with names of their own, stamped before and after it, so "at the
//! top" and "in its place by when it was retained" are both positions among
//! named lines.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use clowder_core::{
    load_string, save_string, Command, Event, GameState, IdeaId, JournalItem, LoadedGame,
    OfflineReport, ResourceId, StructureId, MIN_REPORTABLE_ABSENCE_SECONDS,
};
use clowder_tui::nav::rows::{rows, RowKind};
use clowder_tui::nav::{ArmedRow, RowKey, Section, ViewState};
use clowder_tui::render;
use clowder_tui::render::widgets::inspector::{placement, Placement};
use clowder_tui::render::widgets::overlays;
use clowder_tui::runtime::clock::Reading;
use clowder_tui::runtime::{App, TickAccumulator};
use crossterm::event::{Event as TermEvent, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;

// ---------------------------------------------------------------------------
// The load seam
// ---------------------------------------------------------------------------

/// Queue whatever report a load retained, as start-up does with what the
/// core's loader returned.
fn queue_load(view: &mut ViewState, loaded: &LoadedGame) {
    clowder_tui::nav::queue_load_report(view, loaded);
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("clowder-reports-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("save.json")
}

/// A meadow with plenty of everything, every structure revealed, and a nap
/// that is listed and affordable.
fn meadow() -> GameState {
    let mut s = GameState::new(4242);
    for r in ResourceId::ALL {
        s.resources[r.ix()] = 1.0e9;
    }
    s.revealed = StructureId::ALL.iter().map(|b| clowder_core::ContentId::Structure(*b)).collect();
    s.ideas.insert(IdeaId::DeepNapTheory);
    s.resources[ResourceId::Starshards.ix()] = 1.0e12;
    s.ticks = 5_000;
    s
}

/// The name only the load's report carries, so its pinned entry can be found.
const LOAD_ARRIVAL: &str = "Quenby";

/// A report a load's catch-up would fold over three days away.
fn load_report() -> OfflineReport {
    let mut r = OfflineReport { elapsed_seconds: 3 * 86_400, ..OfflineReport::default() };
    r.resource_deltas[ResourceId::Feathers.ix()] = 4_800.0;
    r.arrived = vec![(LOAD_ARRIVAL.to_string(), 1)];
    r
}

/// What the core's loader returns for `state` after it retained `report` —
/// exactly as `load_string` does: the report kept in the journal at the tick
/// the catch-up ended on, and handed back beside the state.
fn loaded(mut state: GameState, report: OfflineReport) -> LoadedGame {
    state.journal.record_away(state.ticks, &report);
    LoadedGame { state, report }
}

fn app(state: GameState, view: ViewState, name: &str) -> App {
    App {
        state,
        view,
        clock: TickAccumulator::default(),
        save_path: scratch(name),
        last_save: Instant::now(),
        prev_layout: None,
        quitting: false,
        save_failed: false,
    }
}

/// The view a session opens in, already on the Meadow, with the load's report
/// queued.
fn opened(loaded: &LoadedGame) -> ViewState {
    let mut view = ViewState { section: Some(Section::Meadow), ..ViewState::default() };
    queue_load(&mut view, loaded);
    view
}

fn still() -> Reading {
    Reading { monotonic: Duration::ZERO, wall: Duration::ZERO }
}

fn elapsed(secs: f64) -> Reading {
    let d = Duration::from_secs_f64(secs);
    Reading { monotonic: d, wall: d }
}

fn press(code: KeyCode) -> TermEvent {
    TermEvent::Key(KeyEvent::from(code))
}

fn press_char(c: char) -> TermEvent {
    let modifiers = if c.is_ascii_uppercase() { KeyModifiers::SHIFT } else { KeyModifiers::NONE };
    TermEvent::Key(KeyEvent::new(KeyCode::Char(c), modifiers))
}

fn term(w: u16, h: u16) -> Terminal<TestBackend> {
    Terminal::new(TestBackend::new(w, h)).expect("TestBackend is infallible")
}

/// Step once with these keys.
fn keys(a: &mut App, t: &mut Terminal<TestBackend>, input: &[TermEvent]) {
    a.step(t, still(), input).expect("a step against a TestBackend does not fail");
}

fn nap_key(state: &GameState) -> RowKey {
    rows(state, Section::Dreams)
        .into_iter()
        .find(|r| matches!(r.kind, RowKind::Act { command: Command::DeepNap(_), .. }))
        .map(|r| r.key)
        .expect("fixture: the nap row is listed")
}

/// Take a nap of `years` the way a player does: the nap row armed with that
/// count, then a distinct Enter through the loop to commit it. Leaves the
/// cursor where it was.
fn nap(a: &mut App, t: &mut Terminal<TestBackend>, years: u32) {
    let (section, focus) = (a.view.section, a.view.focus.clone());
    let key = nap_key(&a.state);
    a.view.section = Some(Section::Dreams);
    a.view.focus[Section::Dreams.index()] = Some(key.clone());
    a.view.armed = Some(ArmedRow {
        key,
        command: Command::DeepNap(years),
        expires_at: Instant::now() + Duration::from_secs(8),
    });
    a.view.last_repeat = None;
    keys(a, t, &[press(KeyCode::Enter)]);
    a.view.section = section;
    a.view.focus = focus;
}

fn away_entries(state: &GameState) -> usize {
    state.journal.kept.iter().filter(|e| matches!(e.item, JournalItem::Away(_))).count()
}

// ---------------------------------------------------------------------------
// Reading the screen
// ---------------------------------------------------------------------------

/// The banner the frame draws for this state and view, wide enough for its
/// whole line — or `None` where the frame gives no banner row at all.
fn banner_of(state: &GameState, view: &ViewState) -> Option<String> {
    let mut t = term(200, 40);
    let mut layout = None;
    t.draw(|f| layout = Some(render::frame(f, state, view, None))).expect("TestBackend is infallible");
    let rect = layout.expect("the frame reports its layout").banner?;
    let buf = t.backend().buffer();
    let line: String =
        (rect.x..rect.right()).map(|x| buf.cell((x, rect.y)).map(|c| c.symbol()).unwrap_or(" ")).collect();
    Some(line.trim_end().to_string())
}

fn showing(a: &App) -> Option<String> {
    banner_of(&a.state, &a.view)
}

/// The Journal as the open overlay draws it, tall and wide enough that nothing
/// is scrolled or clipped away, one entry per element, blank lines dropped.
fn journal_lines(a: &App) -> Vec<String> {
    let rect = Rect::new(0, 0, 300, 200);
    let mut t = term(300, 200);
    t.draw(|f| overlays::journal(f, rect, &a.state, &a.view)).expect("TestBackend is infallible");
    let buf = t.backend().buffer();
    (0..rect.height)
        .map(|y| (0..rect.width).map(|x| buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(" ")).collect::<String>())
        .map(|l| l.trim_end().to_string())
        .filter(|l| !l.trim().is_empty())
        .collect()
}

fn first_line_with(lines: &[String], needle: &str) -> Option<usize> {
    lines.iter().position(|l| l.contains(needle))
}

/// The lines the Journal draws for `report` when it is the only entry and no
/// report is queued — how a report's own entry is recognised among others.
fn own_lines(report: &OfflineReport) -> Vec<String> {
    let mut s = GameState::new(11);
    s.journal.record_away(100, report);
    let a = app(s, ViewState { overlay: Some(clowder_tui::nav::Overlay::Journal { anchor: 0 }), ..Default::default() }, "own");
    journal_lines(&a).into_iter().map(|l| l.trim().to_string()).filter(|l| l.chars().count() >= 8).collect()
}

fn first_line_of_entry(lines: &[String], own: &[String]) -> Option<usize> {
    lines.iter().position(|l| own.iter().any(|o| l.contains(o.as_str())))
}

fn arrived(name: &str) -> Event {
    Event::KittenArrived { name: name.to_string() }
}

// ---------------------------------------------------------------------------
// A session with two unread reports: a load's, then a nap's
// ---------------------------------------------------------------------------

/// A session opened on the Meadow with the load's report queued, and then a
/// nap taken — so two reports are unread and the nap's, the newer, shows.
/// Returns the state and view to build cases from, and the two banners.
struct TwoUnread {
    state: GameState,
    view: ViewState,
    load_banner: String,
    nap_banner: String,
}

fn two_unread() -> TwoUnread {
    let l = loaded(meadow(), load_report());
    let mut a = app(l.state.clone(), opened(&l), "two");
    let load_banner = showing(&a).expect("fixture: the load's report shows a banner");
    let mut t = term(160, 40);
    let before = away_entries(&a.state);
    nap(&mut a, &mut t, 1);
    assert_eq!(away_entries(&a.state), before + 1, "fixture: the nap was accepted and retained");
    let nap_banner = showing(&a).expect("the accepted nap left no banner at all");
    assert_ne!(
        nap_banner, load_banner,
        "the load's banner is still the one showing after a nap was accepted; the nap's report, \
         the newest unread, shows first"
    );
    TwoUnread { state: a.state, view: a.view, load_banner, nap_banner }
}

/// A case's session: the two-unread state and view, focused where `focus`
/// puts the Meadow's cursor.
fn case(two: &TwoUnread, focus: Option<RowKey>, name: &str) -> App {
    let mut view = two.view.clone();
    view.section = Some(Section::Meadow);
    view.focus[Section::Meadow.index()] = focus;
    view.last_repeat = None;
    app(two.state.clone(), view, name)
}

/// A structure's row on the Meadow: an ordinary row that acts, and takes a
/// count.
fn structure_row(state: &GameState) -> RowKey {
    meadow_rows(state)
        .into_iter()
        .find(|k| matches!(k, RowKey::Content(clowder_core::ContentId::Structure(_), "")))
        .expect("fixture: a structure row is listed on the Meadow")
}

fn meadow_rows(state: &GameState) -> Vec<RowKey> {
    rows(state, Section::Meadow).into_iter().filter(|r| r.selectable()).map(|r| r.key).collect()
}

// ---------------------------------------------------------------------------
// TUI-NAV-027 — which reports get a banner
// ---------------------------------------------------------------------------

/// A load's report gets a banner exactly where the core kept it in the
/// journal: a catch-up one second short of the core's minimum leaves none,
/// and one at the minimum or longer queues one.
/// @spec TUI-NAV-027
#[test]
fn a_loads_report_gets_a_banner_exactly_where_the_core_kept_it() {
    let saved_at = 1_000_000;
    let text = save_string(&GameState::new(7), saved_at);
    for gap in [MIN_REPORTABLE_ABSENCE_SECONDS - 1, MIN_REPORTABLE_ABSENCE_SECONDS, 10 * MIN_REPORTABLE_ABSENCE_SECONDS] {
        let l = load_string(&text, saved_at + gap).expect("fixture: the save loads");
        let kept = away_entries(&l.state) > 0;
        let mut view = ViewState::default();
        queue_load(&mut view, &l);
        assert_eq!(
            banner_of(&l.state, &view).is_some(),
            kept,
            "a load {gap}s after the save {} kept its report in the journal, and the shell {} a \
             banner; a report gets a banner exactly where the core retained it",
            if kept { "" } else { "never" },
            if kept { "gave it no" } else { "gave it" }
        );
    }
}

/// Which reports get a banner is judged from the journal, never from a
/// minimum span the shell holds: a report the journal does not hold gets none
/// however long its span, and one the journal holds gets one however short.
/// @spec TUI-NAV-027
#[test]
fn whether_a_loads_report_gets_a_banner_is_judged_from_the_journal() {
    // Long enough by any span rule, but the journal did not keep it.
    let report = load_report();
    let unkept = LoadedGame { state: meadow(), report: report.clone() };
    let mut view = ViewState::default();
    queue_load(&mut view, &unkept);
    assert_eq!(
        banner_of(&unkept.state, &view),
        None,
        "a report the core did not retain got a banner; the shell gives a banner only to a \
         report the journal holds"
    );

    // Short by the core's own minimum, but the journal holds it.
    let mut short = load_report();
    short.elapsed_seconds = MIN_REPORTABLE_ABSENCE_SECONDS / 2;
    let mut state = meadow();
    state.journal.kept.push(clowder_core::JournalEntry { tick: state.ticks, item: JournalItem::Away(short.clone()) });
    let kept = LoadedGame { state, report: short };
    let mut view = ViewState::default();
    queue_load(&mut view, &kept);
    assert!(
        banner_of(&kept.state, &view).is_some(),
        "a report the journal holds got no banner because its span is short; whether a report \
         gets one is read from the journal, never from a minimum span the shell keeps"
    );

    // An earlier session's report, still in the journal, is not this load's.
    let mut state = meadow();
    state.journal.record_away(100, &load_report());
    let mut quick = load_report();
    quick.elapsed_seconds = 1;
    let old_only = LoadedGame { state, report: quick };
    let mut view = ViewState::default();
    queue_load(&mut view, &old_only);
    assert_eq!(
        banner_of(&old_only.state, &view),
        None,
        "a load whose own report was not retained brought up a banner for a report an earlier \
         session retained"
    );
}

/// Every accepted nap queues a banner the moment it is accepted, however short
/// the nap; a refused one queues nothing.
/// @spec TUI-NAV-027
#[test]
fn every_accepted_retaining_command_queues_a_banner_and_a_refused_one_does_not() {
    let mut a = app(meadow(), ViewState { section: Some(Section::Meadow), ..Default::default() }, "naps");
    let mut t = term(160, 40);
    assert_eq!(showing(&a), None, "fixture: nothing is queued before the first nap");

    for years in [1, 2, 5] {
        let before = away_entries(&a.state);
        nap(&mut a, &mut t, years);
        assert_eq!(away_entries(&a.state), before + 1, "fixture: a {years}-year nap was accepted");
        assert!(
            showing(&a).is_some(),
            "an accepted {years}-year nap brought up no banner; the core keeps every nap, so \
             every accepted nap queues one"
        );
        // Read it, so the next nap's banner is its own.
        keys(&mut a, &mut t, &[press(KeyCode::Esc)]);
        assert_eq!(showing(&a), None, "the nap's banner did not retire on Esc from a bare screen");
    }

    a.state.resources[ResourceId::Starshards.ix()] = 0.0;
    let before = away_entries(&a.state);
    nap(&mut a, &mut t, 1);
    assert_eq!(away_entries(&a.state), before, "fixture: a nap with no starshards is refused");
    assert_eq!(showing(&a), None, "a refused nap brought up a banner; it retained no report");
}

// ---------------------------------------------------------------------------
// TUI-NAV-055 — a nap's banner says how long the nap was
// ---------------------------------------------------------------------------

/// A nap's banner opens with the nap's length in years, as the core's nap
/// event reports it — not the report's span, which is attended time.
/// @spec TUI-NAV-055
#[test]
fn a_command_retained_reports_banner_states_its_length_in_years() {
    for years in [3u32, 7] {
        let mut a = app(meadow(), ViewState { section: Some(Section::Meadow), ..Default::default() }, "years");
        let mut t = term(160, 40);
        nap(&mut a, &mut t, years);
        let line = showing(&a).expect("an accepted nap brought up no banner");
        let words: Vec<String> = line
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .map(|w| w.to_lowercase())
            .collect();
        let n = years.to_string();
        let says_years = words.windows(2).any(|p| p[0] == n && p[1].starts_with('y'))
            || words.iter().any(|w| w.starts_with(&format!("{n}y")));
        assert!(
            says_years,
            "a {years}-year nap's banner does not say {years} years; a nap's line opens with the \
             nap's length in years as the core's nap event reports it. It read as {line:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-050 — the queue
// ---------------------------------------------------------------------------

/// One banner at a time, newest first: the nap taken after the load shows
/// first, retiring it brings up the load's, and retiring that leaves none —
/// and nothing read comes back.
/// @spec TUI-NAV-050
#[test]
fn reports_queue_newest_first_and_show_one_at_a_time() {
    let two = two_unread();
    let mut a = case(&two, None, "queue");
    let mut t = term(160, 40);

    assert_eq!(showing(&a).as_deref(), Some(two.nap_banner.as_str()), "the newest unread report is not the one showing");
    keys(&mut a, &mut t, &[press(KeyCode::Esc)]);
    assert_eq!(
        showing(&a).as_deref(),
        Some(two.load_banner.as_str()),
        "retiring the newest report's banner did not bring up the one behind it; no report is \
         displaced unseen"
    );
    keys(&mut a, &mut t, &[press(KeyCode::Esc)]);
    assert_eq!(showing(&a), None, "a banner is still up after every report was read");
    keys(&mut a, &mut t, &[press(KeyCode::Esc), press(KeyCode::Down), press(KeyCode::Tab)]);
    assert_eq!(showing(&a), None, "a read report came back to the queue");
}

/// Opening the Journal reads every unread report at once: once it closes, no
/// banner is left.
/// @spec TUI-NAV-050
#[test]
fn opening_the_journal_reads_every_unread_report_at_once() {
    let two = two_unread();
    let mut a = case(&two, None, "journal-reads");
    let mut t = term(160, 40);

    keys(&mut a, &mut t, &[press_char('e')]);
    keys(&mut a, &mut t, &[press(KeyCode::Esc)]);
    assert!(a.view.overlay.is_none(), "fixture: Esc closed the Journal");
    assert_eq!(
        showing(&a),
        None,
        "a banner is still up after the Journal was opened; opening it reads every unread report"
    );
}

/// A report still unread when the session ends is not queued again on a later
/// load; that load queues only its own report, where the core kept one.
/// @spec TUI-NAV-050
#[test]
fn a_report_unread_when_the_session_ends_is_not_queued_again() {
    let saved_at = 1_000_000;
    let first = load_string(&save_string(&GameState::new(7), saved_at), saved_at + 20 * 60).expect("loads");
    let mut view = ViewState::default();
    queue_load(&mut view, &first);
    assert!(banner_of(&first.state, &view).is_some(), "fixture: the first load's report is queued");

    // The session ends with it unread.
    let resaved_at = saved_at + 20 * 60;
    let text = save_string(&first.state, resaved_at);

    let quick = load_string(&text, resaved_at + 1).expect("loads");
    let mut view = ViewState::default();
    queue_load(&mut view, &quick);
    assert_eq!(
        banner_of(&quick.state, &view),
        None,
        "a report left unread in an earlier session was queued again on a later load"
    );

    let long = load_string(&text, resaved_at + 40 * 60).expect("loads");
    let mut view = ViewState { section: Some(Section::Meadow), ..ViewState::default() };
    queue_load(&mut view, &long);
    let mut a = app(long.state, view, "unread-later");
    let mut t = term(160, 40);
    assert!(showing(&a).is_some(), "fixture: the later load's own report is queued");
    keys(&mut a, &mut t, &[press(KeyCode::Esc)]);
    assert_eq!(
        showing(&a),
        None,
        "after Esc from a bare screen a banner is still up: either Esc did not retire the later \
         load's own banner, or the earlier session's unread report was queued again behind it"
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-027 / TUI-NAV-050 — the pinned entry's place in the Journal
// ---------------------------------------------------------------------------

/// Before and after the load's report: arrivals stamped either side of the
/// tick the report was retained at.
const EARLIER: &str = "Ashbury";
const LATER: &str = "Zephyrine";

fn with_neighbours(mut state: GameState) -> GameState {
    let at = state.ticks;
    state.journal.record(at - 4_000, &arrived(EARLIER));
    state.journal.record(at + 90_000, &arrived(LATER));
    state
}

/// Whether the load's entry sits between the arrivals stamped before and
/// after it, whichever way the scrollback runs.
fn in_place_by_time(lines: &[String]) -> bool {
    let (Some(e), Some(r), Some(l)) = (
        first_line_with(lines, EARLIER),
        first_line_with(lines, LOAD_ARRIVAL),
        first_line_with(lines, LATER),
    ) else {
        return false;
    };
    (e < r && r < l) || (l < r && r < e)
}

/// Unread reports are pinned at the top of the Journal, newest first, above
/// everything in the scrollback — whatever was stamped after them — and the
/// Journal opens at its top.
/// @spec TUI-NAV-027, TUI-NAV-050
#[test]
fn unread_reports_are_pinned_at_the_journals_top_newest_first() {
    let l = loaded(meadow(), load_report());
    let mut a = app(l.state.clone(), opened(&l), "pinned");
    let mut t = term(160, 40);
    nap(&mut a, &mut t, 1);
    let nap_report = match &a.state.journal.kept.last().expect("the nap was retained").item {
        JournalItem::Away(r) => r.clone(),
        other => panic!("fixture: the last kept entry is the nap's report, not {other:?}"),
    };
    a.state = with_neighbours(a.state.clone());

    keys(&mut a, &mut t, &[press_char('e')]);
    let lines = journal_lines(&a);
    let nap_at = first_line_of_entry(&lines, &own_lines(&nap_report))
        .unwrap_or_else(|| panic!("the nap's entry is not in the Journal:\n{}", lines.join("\n")));
    let load_at = first_line_with(&lines, LOAD_ARRIVAL)
        .unwrap_or_else(|| panic!("the load's entry is not in the Journal:\n{}", lines.join("\n")));
    let later_at = first_line_with(&lines, LATER).expect("fixture: the later arrival is in the Journal");
    let earlier_at = first_line_with(&lines, EARLIER).expect("fixture: the earlier arrival is in the Journal");

    assert!(
        nap_at < load_at,
        "the Journal pins the older unread report above the newer; unread reports are pinned \
         newest first. It read as:\n{}",
        lines.join("\n")
    );
    assert!(
        load_at < later_at && load_at < earlier_at,
        "an unread report is not pinned above the scrollback — something stamped after it sits \
         above it. It read as:\n{}",
        lines.join("\n")
    );
    assert_eq!(nap_at, 0, "the Journal did not open at the top, onto the newest pinned entry:\n{}", lines.join("\n"));
}

/// The opening that reads a report does not move its entry: it stays pinned
/// while that Journal is open, the simulation running behind it, and takes
/// its place in the scrollback by when it was retained only once the Journal
/// closes.
/// @spec TUI-NAV-027, TUI-NAV-050
#[test]
fn a_report_the_journal_read_stays_pinned_until_that_journal_closes() {
    let l = loaded(meadow(), load_report());
    let mut a = app(with_neighbours(l.state.clone()), opened(&l), "held");
    let mut t = term(160, 40);

    keys(&mut a, &mut t, &[press_char('e')]);
    let pinned = |lines: &[String]| {
        first_line_with(lines, LOAD_ARRIVAL).is_some_and(|r| {
            first_line_with(lines, LATER).is_some_and(|l| r < l)
                && first_line_with(lines, EARLIER).is_some_and(|e| r < e)
        })
    };
    let lines = journal_lines(&a);
    assert!(pinned(&lines), "the unread report is not pinned at the top on opening:\n{}", lines.join("\n"));

    a.step(&mut t, elapsed(2.0), &[]).expect("steps");
    let lines = journal_lines(&a);
    assert!(
        pinned(&lines),
        "the report the opening read left the top while that Journal was still open; it stays \
         pinned until the Journal closes. It read as:\n{}",
        lines.join("\n")
    );

    keys(&mut a, &mut t, &[press(KeyCode::Esc)]);
    keys(&mut a, &mut t, &[press_char('e')]);
    let lines = journal_lines(&a);
    assert!(
        in_place_by_time(&lines),
        "once read and with the Journal that read it closed, the report did not take its place \
         in the scrollback by when it was retained. It read as:\n{}",
        lines.join("\n")
    );
}

/// A report read by its banner retiring is in its place in the scrollback at
/// once — the first time the Journal opens after, it is not pinned.
/// @spec TUI-NAV-027
#[test]
fn a_report_read_by_its_banner_takes_its_place_in_the_scrollback_at_once() {
    let l = loaded(meadow(), load_report());
    let mut a = app(with_neighbours(l.state.clone()), opened(&l), "retired");
    let mut t = term(160, 40);

    keys(&mut a, &mut t, &[press(KeyCode::Esc)]);
    assert_eq!(showing(&a), None, "Esc from a bare screen did not retire the only banner");
    keys(&mut a, &mut t, &[press_char('e')]);
    let lines = journal_lines(&a);
    assert!(
        in_place_by_time(&lines),
        "a report read by its banner retiring is still pinned at the Journal's top; once read, \
         and with no open Journal holding it, it keeps its place by when it was retained. It \
         read as:\n{}",
        lines.join("\n")
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-083 — what retires the showing banner
// ---------------------------------------------------------------------------

fn assert_retired_one(a: &App, two: &TwoUnread, what: &str) {
    assert_eq!(
        showing(a).as_deref(),
        Some(two.load_banner.as_str()),
        "{what} did not retire exactly the banner showing and bring up the next unread one"
    );
}

fn assert_still_showing(a: &App, two: &TwoUnread, what: &str) {
    assert_eq!(
        showing(a).as_deref(),
        Some(two.nap_banner.as_str()),
        "{what} did not leave the showing banner showing"
    );
}

/// Esc with nothing open above the banner retires the one showing and brings
/// up the next — never the whole queue.
/// @spec TUI-NAV-083
#[test]
fn esc_on_a_bare_screen_retires_only_the_showing_banner() {
    let two = two_unread();
    for (w, h) in [(80u16, 24u16), (160, 40)] {
        let mut a = case(&two, None, "esc-bare");
        let mut t = term(w, h);
        keys(&mut a, &mut t, &[]);
        keys(&mut a, &mut t, &[press(KeyCode::Esc)]);
        assert_retired_one(&a, &two, &format!("Esc on a bare {w}-column screen"));
    }

    // The inspector as a pane is not a swap: nothing sits above the banner.
    let mut a = case(&two, None, "esc-pane");
    a.view.inspector_wanted = true;
    let mut t = term(160, 40);
    keys(&mut a, &mut t, &[]);
    keys(&mut a, &mut t, &[press(KeyCode::Esc)]);
    assert_retired_one(&a, &two, "Esc with the inspector as a pane");
}

/// Esc closes whatever is open above the banner first — a field, an overlay,
/// an armed row — and leaves the banner exactly as it was; the next Esc
/// retires it.
/// @spec TUI-NAV-083
#[test]
fn esc_closes_what_sits_above_the_banner_before_it_retires_the_banner() {
    let two = two_unread();
    let first_row = Some(structure_row(&two.state));
    let openers: Vec<(&str, Vec<TermEvent>)> = vec![
        ("the count prompt", vec![press_char('x')]),
        ("the palette", vec![press_char('/')]),
        ("the help overlay", vec![press_char('?')]),
    ];
    for (what, open) in openers {
        let mut a = case(&two, first_row.clone(), "esc-above");
        let mut t = term(160, 40);
        keys(&mut a, &mut t, &open);
        assert!(
            a.view.prompt.is_some() || a.view.overlay.is_some(),
            "fixture: {what} opened"
        );
        keys(&mut a, &mut t, &[press(KeyCode::Esc)]);
        assert!(a.view.prompt.is_none() && a.view.overlay.is_none(), "Esc did not close {what}");
        assert_still_showing(&a, &two, &format!("Esc closing {what}"));
        keys(&mut a, &mut t, &[press(KeyCode::Esc)]);
        assert_retired_one(&a, &two, &format!("the Esc after {what} closed"));
    }

    // An armed row: the nap row, armed by a first Enter.
    let mut a = case(&two, None, "esc-armed");
    let key = nap_key(&a.state);
    a.view.section = Some(Section::Dreams);
    a.view.focus[Section::Dreams.index()] = Some(key);
    let mut t = term(160, 40);
    keys(&mut a, &mut t, &[press(KeyCode::Enter)]);
    assert!(a.view.armed.is_some(), "fixture: Enter armed the nap row");
    keys(&mut a, &mut t, &[press(KeyCode::Esc)]);
    assert!(a.view.armed.is_none(), "Esc did not disarm the armed row");
    assert_still_showing(&a, &two, "Esc disarming a row");
    keys(&mut a, &mut t, &[press(KeyCode::Esc)]);
    assert_retired_one(&a, &two, "the Esc after the row was disarmed");
}

/// With the narrow-mode inspector swapped in, Esc returns the list view
/// first and leaves the banner up; only the next Esc retires it.
/// @spec TUI-NAV-083
#[test]
fn with_the_narrow_swap_in_esc_returns_the_list_before_retiring_the_banner() {
    let two = two_unread();
    let mut a = case(&two, meadow_rows(&two.state).into_iter().next(), "esc-swap");
    a.view.inspector_wanted = true;
    let mut t = term(80, 24);
    keys(&mut a, &mut t, &[]);
    assert_eq!(placement(80, a.view.inspector_wanted), Placement::Swapped, "fixture: the swap is in");

    keys(&mut a, &mut t, &[press(KeyCode::Esc)]);
    assert_eq!(
        placement(80, a.view.inspector_wanted),
        Placement::Absent,
        "Esc with the swap in did not return the list view"
    );
    assert_still_showing(&a, &two, "Esc returning the list view from the swap");
    keys(&mut a, &mut t, &[press(KeyCode::Esc)]);
    assert_retired_one(&a, &two, "the Esc after the swap returned");
}

/// Every navigation key on the screen retires only the showing banner —
/// cursor, section, group and digit, each whether or not it moves anything.
/// @spec TUI-NAV-083
#[test]
fn every_navigation_key_retires_only_the_showing_banner_whether_or_not_it_moves() {
    let two = two_unread();
    let list = meadow_rows(&two.state);
    let navigation: Vec<(String, TermEvent)> = [
        KeyCode::Up,
        KeyCode::Down,
        KeyCode::Left,
        KeyCode::Right,
        KeyCode::Tab,
        KeyCode::BackTab,
    ]
    .into_iter()
    .map(|k| (format!("{k:?}"), press(k)))
    .chain(['k', 'j', 'h', 'l', '[', ']'].into_iter().map(|c| (format!("{c:?}"), press_char(c))))
    .chain(('0'..='9').map(|c| (format!("digit {c}"), press_char(c))))
    .collect();

    // At the top of the list (Up moves nothing), in the middle, and at the
    // bottom (Down moves nothing). A digit for a section with nothing in it
    // moves nothing wherever the cursor is.
    let spots = [list.first().cloned(), list.get(list.len() / 2).cloned(), list.last().cloned()];
    for focus in spots {
        for (what, key) in &navigation {
            let mut a = case(&two, focus.clone(), "nav");
            let mut t = term(160, 40);
            keys(&mut a, &mut t, std::slice::from_ref(key));
            assert_retired_one(&a, &two, &format!("{what} with the cursor on {focus:?}"));
        }
    }
}

/// The palette's jump is navigation: it retires only the showing banner.
/// @spec TUI-NAV-083
#[test]
fn the_palettes_jump_retires_only_the_showing_banner() {
    let two = two_unread();
    let aimed = clowder_core::content::structure_def(StructureId::ALL[2]).name;
    let mut a = case(&two, None, "palette-jump");
    let mut t = term(160, 40);
    keys(&mut a, &mut t, &[press_char('/')]);
    let typed: Vec<TermEvent> = aimed.chars().map(press_char).collect();
    keys(&mut a, &mut t, &typed);
    assert_still_showing(&a, &two, "typing into the palette");
    keys(&mut a, &mut t, &[press(KeyCode::Enter)]);
    assert!(a.view.overlay.is_none(), "fixture: the palette's Enter jumped");
    assert_retired_one(&a, &two, "the palette's jump");
}

// ---------------------------------------------------------------------------
// TUI-NAV-084 — what leaves the showing banner showing
// ---------------------------------------------------------------------------

/// Acting on a row — Enter, x, X, +/-, a mnemonic — toggling the inspector,
/// the speed keys, and opening help, the palette or a count prompt each leave
/// the showing banner showing.
/// @spec TUI-NAV-084
#[test]
fn acting_toggling_retiming_and_opening_leave_the_banner_showing() {
    let two = two_unread();
    let build_row = structure_row(&two.state);
    let presses: Vec<(&str, TermEvent)> = vec![
        ("Enter", press(KeyCode::Enter)),
        ("x", press_char('x')),
        ("X", press_char('X')),
        ("+", press_char('+')),
        ("=", press_char('=')),
        ("-", press_char('-')),
        ("Backspace", press(KeyCode::Backspace)),
        ("the mnemonic g", press_char('g')),
        ("the mnemonic f", press_char('f')),
        ("the mnemonic r", press_char('r')),
        ("Space", press_char(' ')),
        ("i", press_char('i')),
        ("<", press_char('<')),
        (">", press_char('>')),
        ("?", press_char('?')),
        ("/", press_char('/')),
    ];
    for (what, key) in presses {
        for w in [80u16, 160] {
            let mut a = case(&two, Some(build_row.clone()), "keep");
            let mut t = term(w, if w == 80 { 24 } else { 40 });
            keys(&mut a, &mut t, &[]);
            keys(&mut a, &mut t, std::slice::from_ref(&key));
            assert_still_showing(&a, &two, &format!("{what} at {w} columns"));
        }
    }
}

/// The keys a full-screen overlay takes for its own scrolling leave the
/// showing banner showing.
/// @spec TUI-NAV-084
#[test]
fn an_overlays_own_scrolling_keys_leave_the_banner_showing() {
    let two = two_unread();
    let scrolling = [
        press(KeyCode::Up),
        press(KeyCode::Down),
        press_char('j'),
        press_char('k'),
        press_char('['),
        press_char(']'),
    ];
    for key in scrolling {
        let mut a = case(&two, None, "overlay-scroll");
        let mut t = term(160, 40);
        keys(&mut a, &mut t, &[press_char('?')]);
        assert!(a.view.overlay.is_some(), "fixture: help is open");
        keys(&mut a, &mut t, std::slice::from_ref(&key));
        assert_still_showing(&a, &two, &format!("{key:?} scrolling the help overlay"));
    }
}

/// The simulation advancing leaves the showing banner showing.
/// @spec TUI-NAV-084
#[test]
fn the_simulation_advancing_leaves_the_banner_showing() {
    let two = two_unread();
    let mut a = case(&two, None, "sim");
    a.state.speed = 8.0;
    let mut t = term(160, 40);
    let ticks = a.state.ticks;
    for _ in 0..5 {
        a.step(&mut t, elapsed(2.0), &[]).expect("steps");
    }
    assert!(a.state.ticks > ticks, "fixture: the simulation advanced");
    assert_still_showing(&a, &two, "the simulation advancing");
}
