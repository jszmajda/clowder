//! The count prompt's face and the runs it starts: what an open prompt
//! previews (TUI-NAV-067), how far its count goes (TUI-NAV-068), how a run that
//! stops short reports its stop (TUI-NAV-069) and what each call of an `X` run
//! carries (TUI-NAV-012), and the one-unit price the inspector states on a
//! Pounce or Trade row (TUI-NAV-073).
//!
//! These live together, apart from `update.rs` and `inspector.rs`, because each
//! reads something the shell and core do not yet expose — the core's price of
//! one unit of a count-taking command (SIM-ENG-047, pinned as
//! `GameState::unit_price(&Command) -> Option<Vec<(ResourceId, f64)>>`) and the
//! shell-side cap that bounds `X` (pinned as the public constant
//! `nav::update::MAX_AFFORDABLE_CAP`). Kept in their own binary, they fail to
//! compile until those land without taking the existing suites down with them.
//!
//! **How a preview is found.** Where the prompt draws, and in what words, is
//! the render leaf's business; nothing here pins it. A preview is read the way
//! `inspector.rs` reads a cost: as the *numbers* on screen, each a maximal run
//! of digits and dots, compared against the crate's own renderings of the value
//! (`fmt::exact`, or the digits of `fmt::compact`). The baseline for "what the
//! count added" is the same frame with the prompt open and nothing typed, so a
//! number that appears only once the count is typed is the preview's.
//!
//! **How a run is observed.** `App::step` is driven with real key events and a
//! zero-length clock reading, so no tick runs and every change to the meadow is
//! the run's own; the footer is read from the view the step leaves behind.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use clowder_core::content::MEADOW_MOVE_MIN_KITTENS;
use clowder_core::{
    ArchId, ColonyId, Command, CommandError, ContentId, CraftId, GameState, IdeaId, JobId, Kitten,
    KittenTrait, LockReason, Location, ResourceId, RiteId, StructureId,
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::Terminal;

use clowder_tui::nav::rows::{rows, Row, RowKind};
use clowder_tui::nav::update::MAX_AFFORDABLE_CAP;
use clowder_tui::nav::{plain_error, update, Action, CountSemantics, Outcome, RowKey, Section, ViewState};
use clowder_tui::render::fmt;
use clowder_tui::render::layout::{compute, Body};
use clowder_tui::render::widgets::inspector;
use clowder_tui::runtime::clock::Reading;
use clowder_tui::runtime::{App, TickAccumulator};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn kitten(name: &str, job: Option<JobId>) -> Kitten {
    Kitten {
        name: name.to_string(),
        job,
        location: Location::Home,
        arrived_day: 0,
        job_seq: job.map(|_| 1),
        away_seq: None,
        skills: [0.0; 8],
        exp: 1.0e6,
        rank: 0,
        trait_id: KittenTrait::None,
        tinkering: None,
    }
}

/// Every surface open and everything affordable many times over — the same
/// meadow `update.rs` measures the reducer against.
fn rich() -> GameState {
    let mut s = GameState::new(7);
    for r in ResourceId::ALL {
        s.resources[r.ix()] = 1.0e12;
    }
    for b in StructureId::ALL {
        s.structures[b.ix()] = 5;
    }
    s.ideas = IdeaId::ALL.iter().copied().collect::<BTreeSet<_>>();
    s.revealed = StructureId::ALL
        .iter()
        .map(|b| ContentId::Structure(*b))
        .chain(IdeaId::ALL.iter().map(|i| ContentId::Idea(*i)))
        .collect::<BTreeSet<_>>();
    s.roster_high_water = MEADOW_MOVE_MIN_KITTENS + 9;
    let mut roster: Vec<Kitten> = (0..MEADOW_MOVE_MIN_KITTENS + 9)
        .map(|i| kitten(&format!("kitten-{i}"), None))
        .collect();
    for k in roster.iter_mut().take(4) {
        k.job = Some(JobId::Tinkerer);
    }
    roster[0].tinkering = Some(CraftId::ALL[0]);
    s.kittens = roster;
    s.elder = Some(0);
    s.peak_devotion = 1.0e12;
    s.devotion = 1.0e6;
    s.serenity = 1.0e12;
    s.rites = [RiteId::TheReverie, RiteId::Enlightenment].into_iter().collect::<BTreeSet<_>>();
    s.arches_bought = [ArchId::EverdreamArch].into_iter().collect::<BTreeSet<_>>();
    s.discovered = ColonyId::ALL.iter().copied().collect::<BTreeSet<_>>();
    s.whale_visit_years = 5;
    s.whale_contentment = 1;
    s.generation = 1;
    s.elder_wisdom = 1.0e9;
    s.warm_memories = 1.0e3;
    s
}

/// A meadow one keypress old: almost nothing a rich meadow lists is listed here.
fn fresh() -> GameState {
    GameState::new(3)
}

fn focused(section: Section, key: RowKey) -> ViewState {
    let mut v = ViewState { section: Some(section), ..Default::default() };
    v.focus[section.index()] = Some(key);
    v
}

fn every_row(state: &GameState) -> Vec<(Section, Row)> {
    Section::ALL
        .into_iter()
        .flat_map(|section| {
            rows(state, section)
                .into_iter()
                .filter(|r| r.selectable())
                .map(move |r| (section, r))
        })
        .collect()
}

fn owned(state: &GameState, row: &Row) -> bool {
    match &row.key {
        RowKey::Content(id, _) => state.lock_reason(*id) == Some(LockReason::AlreadyYours),
        _ => false,
    }
}

/// Whether `x` opens the count prompt on this row (TUI-NAV-011).
fn opens_prompt(state: &GameState, row: &Row) -> bool {
    !owned(state, row)
        && match &row.kind {
            RowKind::Act { semantics, .. } => {
                matches!(semantics, CountSemantics::CountTaking | CountSemantics::RepeatN)
            }
            RowKind::Count { .. } => true,
            RowKind::Display | RowKind::Group => false,
        }
}

fn prompt_rows(state: &GameState) -> Vec<(Section, Row)> {
    every_row(state).into_iter().filter(|(_, r)| opens_prompt(state, r)).collect()
}

fn with_count(command: Command, n: u32) -> Command {
    match command {
        Command::Pounce(_) => Command::Pounce(n),
        Command::Trade(colony, _) => Command::Trade(colony, n),
        Command::DeepNap(_) => Command::DeepNap(n),
        other => other,
    }
}

/// What the prompt's Enter with a count of `n` asks of the core on a row that
/// does not arm. @spec TUI-NAV-070
fn accepted(row: &Row, n: u32) -> Outcome {
    match &row.kind {
        RowKind::Act { command, semantics: CountSemantics::CountTaking } => {
            Outcome::Apply(with_count(*command, n))
        }
        RowKind::Act { command, semantics: CountSemantics::RepeatN } => Outcome::Repeat(*command, n),
        RowKind::Count { increment, .. } => Outcome::Repeat(*increment, n),
        other => panic!("fixture: {:?} is not a row the count prompt opens on ({other:?})", row.key),
    }
}

/// Open the prompt on `row` as a player does, and type `digits`.
fn prompt_on(section: Section, row: &Row, digits: &str, state: &GameState) -> ViewState {
    let mut view = focused(section, row.key.clone());
    update(Action::CountPrompt, &mut view, state);
    assert!(view.prompt.is_some(), "fixture: x on {:?} opened no count prompt", row.key);
    for c in digits.chars() {
        update(Action::Type(c), &mut view, state);
    }
    view
}

fn assert_footer_says_something_plain(view: &ViewState, context: &str) {
    let (message, until) = view
        .footer
        .as_ref()
        .unwrap_or_else(|| panic!("{context} left no footer note"));
    assert!(!message.trim().is_empty(), "{context} left an empty footer note");
    assert!(
        message.contains(' '),
        "{context} left the footer note {message:?}; it is shown to a player, in words"
    );
    for artefact in ['{', '}', '(', ')'] {
        assert!(
            !message.contains(artefact),
            "{context} left the footer note {message:?}, which reads like a debug-printed value"
        );
    }
    let left = until.saturating_duration_since(Instant::now());
    assert!(
        left > Duration::from_millis(500) && left <= Duration::from_secs(10),
        "{context} left a footer note lasting {left:?}; a few seconds, per TUI-NAV-014"
    );
}

// ---------------------------------------------------------------------------
// Reading a frame
// ---------------------------------------------------------------------------

/// Wide and tall enough that nothing a preview says is cut for room.
const W: u16 = 200;
const H: u16 = 50;

fn frame_text(state: &GameState, view: &ViewState) -> String {
    let mut term = Terminal::new(TestBackend::new(W, H)).expect("TestBackend is infallible");
    term.draw(|f| {
        clowder_tui::render::frame(f, state, view, None);
    })
    .expect("TestBackend is infallible");
    buffer_text(term.backend().buffer(), Rect::new(0, 0, W, H))
}

fn buffer_text(buf: &Buffer, rect: Rect) -> String {
    (rect.y..rect.bottom())
        .map(|y| {
            (rect.x..rect.right())
                .map(|x| buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "))
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Every number in a text, as a maximal run of digits and dots.
fn numbers(s: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut cur = String::new();
    for c in s.chars().chain(std::iter::once(' ')) {
        if c.is_ascii_digit() || c == '.' {
            cur.push(c);
        } else {
            let token = cur.trim_matches('.').to_string();
            if !token.is_empty() {
                out.insert(token);
            }
            cur.clear();
        }
    }
    out
}

/// The ways the crate renders an amount: exactly, or compactly (whose digits
/// are what `numbers` reads, the magnitude letter aside).
fn renderings(amount: f64) -> [String; 2] {
    let compact = fmt::compact(amount);
    [fmt::exact(amount), compact.trim_end_matches(|c: char| c.is_alphabetic()).to_string()]
}

fn shows_amount(on_screen: &BTreeSet<String>, amount: f64) -> bool {
    renderings(amount).iter().any(|r| on_screen.contains(r))
}

/// What typing a count added to the frame: the numbers on screen with the count
/// typed that were not there with the prompt open and empty.
fn typed_numbers(state: &GameState, section: Section, row: &Row, digits: &str) -> (BTreeSet<String>, BTreeSet<String>, String) {
    let empty = frame_text(state, &prompt_on(section, row, "", state));
    let typed = frame_text(state, &prompt_on(section, row, digits, state));
    let before = numbers(&empty);
    let after = numbers(&typed);
    let added: BTreeSet<String> = after.difference(&before).cloned().collect();
    (after, added, typed)
}

/// A count-alone preview: the count is on screen, and nothing else the count
/// added is a number.
fn assert_count_alone(state: &GameState, section: Section, row: &Row, what: &str) {
    const N: &str = "37";
    let baseline = numbers(&frame_text(state, &prompt_on(section, row, "", state)));
    assert!(
        !baseline.contains(N),
        "fixture: {N} is already on screen beside {:?}, so the count cannot be told apart",
        row.key
    );
    let (after, added, frame) = typed_numbers(state, section, row, N);
    assert!(
        after.contains(N),
        "with {N} typed into the prompt on {what} {:?}, the count is not on screen; the preview \
         there is the count alone.\n{frame}",
        row.key
    );
    let extra: Vec<&String> = added.iter().filter(|t| t.as_str() != N).collect();
    assert!(
        extra.is_empty(),
        "with {N} typed into the prompt on {what} {:?}, the count brought {extra:?} to the screen \
         as well; the preview there is the count alone.\n{frame}",
        row.key
    );
}

// ===========================================================================
// TUI-NAV-067 — the preview
// ===========================================================================

/// On a repeat-N row the prompt previews the core's total for taking the count
/// from current state, escalation included, per component — or the count alone
/// where that total names no resource. Quantified over every repeat-N row.
///
/// @spec TUI-NAV-067
#[test]
fn a_repeat_n_count_previews_the_cores_total_for_taking_it() {
    let state = rich();
    let mut seen = 0;
    for (section, row) in prompt_rows(&state) {
        if !matches!(row.kind, RowKind::Act { semantics: CountSemantics::RepeatN, .. }) {
            continue;
        }
        seen += 1;
        let RowKey::Content(id, _) = row.key else {
            panic!("fixture: the repeat-N row {:?} is not a content item", row.key);
        };
        let total = state.cost_of_next(id, 7);
        if total.is_empty() {
            assert_count_alone(&state, section, &row, "the unpriced repeat-N row");
            continue;
        }
        let (after, added, frame) = typed_numbers(&state, section, &row, "7");
        for (r, amount) in &total {
            assert!(
                shows_amount(&after, *amount),
                "with 7 typed into the prompt on {:?}, the core's total of {} {r:?} for seven is \
                 not on screen; the preview is the core's total, per component.\n{frame}",
                row.key,
                fmt::exact(*amount)
            );
        }
        assert!(
            total.iter().any(|(_, a)| renderings(*a).iter().any(|t| added.contains(t))),
            "with 7 typed into the prompt on {:?}, nothing of the core's total for seven appeared \
             that was not already on screen.\n{frame}",
            row.key
        );
    }
    assert!(seen > 0, "fixture: no repeat-N row is listed");
}

/// On a Pounce or Trade row the prompt previews N times the price of one unit
/// the core reports, per component — or the count alone where the core reports
/// none. Quantified over every such row.
///
/// @spec TUI-NAV-067
#[test]
fn a_per_unit_priced_count_previews_n_times_the_cores_unit_price() {
    let state = rich();
    let mut seen = 0;
    for (section, row) in prompt_rows(&state) {
        let RowKind::Act { command, semantics: CountSemantics::CountTaking } = row.kind else {
            continue;
        };
        if row.heavy {
            continue;
        }
        seen += 1;
        let Some(unit) = state.unit_price(&command) else {
            assert_count_alone(&state, section, &row, "the count-taking row the core prices not");
            continue;
        };
        let (after, added, frame) = typed_numbers(&state, section, &row, "7");
        for (r, each) in &unit {
            let amount = each * 7.0;
            assert!(
                shows_amount(&after, amount),
                "with 7 typed into the prompt on {:?}, seven times the core's one-unit price of \
                 {r:?} ({}) is not on screen.\n{frame}",
                row.key,
                fmt::exact(amount)
            );
        }
        assert!(
            unit.iter().any(|(_, each)| renderings(each * 7.0).iter().any(|t| added.contains(t))),
            "with 7 typed into the prompt on {:?}, nothing of seven units' price appeared that was \
             not already on screen.\n{frame}",
            row.key
        );
    }
    assert!(seen > 0, "fixture: no Pounce or Trade row is listed");
}

/// On the DeepNap row — whose price escalates with its years and whose prompt
/// only arms — and on a count row, whose assign costs nothing, the preview is
/// the count alone. Quantified over every such row.
///
/// @spec TUI-NAV-067
#[test]
fn the_heavy_count_taking_and_count_rows_preview_the_count_alone() {
    let state = rich();
    let (mut nap, mut counts) = (0, 0);
    for (section, row) in prompt_rows(&state) {
        let what = match row.kind {
            RowKind::Count { .. } => {
                counts += 1;
                "the count row"
            }
            RowKind::Act { command: Command::DeepNap(_), .. } => {
                nap += 1;
                "the nap row"
            }
            _ => continue,
        };
        assert_count_alone(&state, section, &row, what);
    }
    assert!(nap > 0, "fixture: the DeepNap row is not listed");
    assert!(counts > 0, "fixture: no count row is listed");
}

/// An empty or zero count previews nothing, on every row the prompt opens on;
/// nor does a prompt whose row is no longer listed, whatever count it holds —
/// there is nothing current to price.
///
/// @spec TUI-NAV-067
#[test]
fn an_empty_or_zero_count_or_an_unlisted_row_previews_nothing() {
    let state = rich();
    let gone = fresh();
    let mut vanished = 0;

    for (section, row) in prompt_rows(&state) {
        for zero in ["0", "00"] {
            let (_, added, frame) = typed_numbers(&state, section, &row, zero);
            let extra: Vec<&String> = added.iter().filter(|t| t.as_str() != zero).collect();
            assert!(
                extra.is_empty(),
                "with {zero:?} typed into the prompt on {:?}, {extra:?} came to the screen; a \
                 zero count previews nothing.\n{frame}",
                row.key
            );
        }

        if rows(&gone, section).iter().any(|r| r.key == row.key) {
            continue;
        }
        vanished += 1;
        let empty = frame_text(&gone, &prompt_on(section, &row, "", &state));
        let typed = frame_text(&gone, &prompt_on(section, &row, "7", &state));
        let added: Vec<String> =
            numbers(&typed).difference(&numbers(&empty)).filter(|t| t.as_str() != "7").cloned().collect();
        assert!(
            added.is_empty(),
            "a prompt opened on {:?} and drawn once that row had left its list brought {added:?} \
             to the screen; a prompt whose row is not listed previews nothing.\n{typed}",
            row.key
        );
    }
    assert!(vanished > 0, "fixture: no prompt row leaves its list in a fresh meadow");
}

// ===========================================================================
// TUI-NAV-068 — the bound
// ===========================================================================

/// The prompt's count is bounded by the cap that bounds `X`: every digit up to
/// the cap is taken, a digit that would carry the count past it is not — and
/// says so in a footer note — and Enter then uses exactly the count shown.
/// Quantified over every row the prompt opens on, and every digit.
///
/// @spec TUI-NAV-068
#[test]
fn the_count_prompt_takes_no_digit_past_the_cap_that_bounds_x() {
    let state = rich();
    let cap = MAX_AFFORDABLE_CAP;
    let cap_digits = cap.to_string();
    let prefix = (cap / 10).to_string();
    let mut seen = 0;

    for (section, row) in prompt_rows(&state) {
        seen += 1;

        // Up to the cap, every digit is taken, silently.
        let mut view = focused(section, row.key.clone());
        update(Action::CountPrompt, &mut view, &state);
        for (i, c) in cap_digits.char_indices() {
            update(Action::Type(c), &mut view, &state);
            assert_eq!(
                view.prompt.as_ref().map(|p| p.digits.clone()),
                Some(cap_digits[..=i].to_string()),
                "typing the cap ({cap}) into the prompt on {:?}, digit {c:?} was not taken",
                row.key
            );
            assert!(view.footer.is_none(), "a digit within the cap left a footer note on {:?}", row.key);
        }

        // Past the cap, no digit is taken, and each refusal says so.
        for d in '0'..='9' {
            view.footer = None;
            update(Action::Type(d), &mut view, &state);
            assert_eq!(
                view.prompt.as_ref().map(|p| p.digits.clone()),
                Some(cap_digits.clone()),
                "with the cap ({cap}) typed on {:?}, digit {d:?} was taken and carried the count \
                 past it",
                row.key
            );
            assert_footer_says_something_plain(
                &view,
                &format!("a digit past the cap on {:?}", row.key),
            );
        }

        // Enter uses the count the prompt shows.
        let out = update(Action::Activate, &mut view, &state);
        if row.heavy {
            assert_eq!(
                view.armed.as_ref().map(|a| a.command),
                match row.kind {
                    RowKind::Act { command, .. } => Some(with_count(command, cap)),
                    _ => None,
                },
                "the capped prompt on the heavy {:?} did not arm with the count it showed",
                row.key
            );
        } else {
            assert_eq!(
                out,
                accepted(&row, cap),
                "the capped prompt on {:?} did not act with the count it showed",
                row.key
            );
        }

        // One digit short of the cap, each digit is taken exactly when the
        // count it makes is within the cap.
        for d in '0'..='9' {
            let mut view = prompt_on(section, &row, &prefix, &state);
            let would_be: u64 = format!("{prefix}{d}").parse().expect("digits");
            update(Action::Type(d), &mut view, &state);
            let now = view.prompt.as_ref().map(|p| p.digits.clone()).unwrap_or_default();
            if would_be <= cap as u64 {
                assert_eq!(now, format!("{prefix}{d}"), "{would_be} is within the cap ({cap}) and was refused");
                assert!(view.footer.is_none(), "{would_be} is within the cap and left a footer note");
            } else {
                assert_eq!(now, prefix, "{would_be} is past the cap ({cap}) and was taken");
                assert_footer_says_something_plain(&view, &format!("typing {would_be} on {:?}", row.key));
            }
        }
    }
    assert!(seen > 0, "fixture: no row opens the count prompt");
}

// ===========================================================================
// TUI-NAV-069, TUI-NAV-012 — runs, driven through the loop
// ===========================================================================

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("clowder-count-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("save.json")
}

fn app_on(state: GameState, section: Section, key: RowKey, name: &str) -> App {
    App {
        state,
        view: focused(section, key),
        clock: TickAccumulator::default(),
        save_path: scratch(name),
        last_save: Instant::now(),
        prev_layout: None,
        quitting: false,
        save_failed: false,
    }
}

fn press(c: char) -> Event {
    Event::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE))
}

fn enter() -> Event {
    Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
}

/// x, the digits of `n`, Enter.
fn times(n: u32) -> Vec<Event> {
    let mut out = vec![press('x')];
    out.extend(n.to_string().chars().map(press));
    out.push(enter());
    out
}

/// One frame with these keys and no time passing: every change is the keys'.
fn step(app: &mut App, input: &[Event]) {
    let mut term = Terminal::new(TestBackend::new(120, 40)).expect("TestBackend is infallible");
    let still = Reading { monotonic: Duration::ZERO, wall: Duration::ZERO };
    app.step(&mut term, still, input).expect("an in-memory step does not fail");
}

fn footer(app: &App) -> Option<String> {
    app.view
        .footer
        .as_ref()
        .filter(|(_, until)| *until > Instant::now())
        .map(|(m, _)| m.clone())
}

/// The first Build row whose structure is open, and its structure.
fn build_row(state: &GameState) -> (RowKey, StructureId) {
    rows(state, Section::Meadow)
        .into_iter()
        .find_map(|r| match r.kind {
            RowKind::Act { command: Command::Build(s), .. } => Some((r.key.clone(), s)),
            _ => None,
        })
        .expect("fixture: a Build row is listed")
}

/// `rich()`, with stock of the Build row's components for exactly `k` more.
fn stocked_for_builds(k: u32) -> (GameState, RowKey, StructureId) {
    let mut state = rich();
    let (key, s) = build_row(&state);
    let id = ContentId::Structure(s);
    for (r, _) in state.cost_of_next(id, 1) {
        state.resources[r.ix()] = 0.0;
    }
    for (r, amount) in state.cost_of_next(id, k) {
        state.resources[r.ix()] = amount;
    }
    (state, key, s)
}

/// The first count row in Clowder, and the job it assigns to.
fn job_row(state: &GameState) -> (RowKey, Command) {
    rows(state, Section::Clowder)
        .into_iter()
        .find_map(|r| match r.kind {
            RowKind::Count { increment, .. } => Some((r.key.clone(), increment)),
            _ => None,
        })
        .expect("fixture: a job count row is listed")
}

/// `rich()`, with exactly `k` kittens free to take a job.
fn with_idle(k: usize) -> GameState {
    let mut state = rich();
    let busy = JobId::ALL[0];
    for (i, kit) in state.kittens.iter_mut().enumerate() {
        kit.tinkering = None;
        kit.job = if i < k { None } else { Some(busy) };
        kit.job_seq = kit.job.map(|_| 1);
    }
    state
}

fn pounce_row(state: &GameState) -> RowKey {
    rows(state, Section::Clowder)
        .into_iter()
        .find(|r| matches!(r.kind, RowKind::Act { command: Command::Pounce(_), .. }))
        .map(|r| r.key)
        .expect("fixture: the Pounce row is listed")
}

/// `rich()`, with the price of exactly `k` squads in stock, as the core
/// reports one squad's price.
fn stocked_for_squads(k: u32) -> GameState {
    let mut state = rich();
    let unit = state.unit_price(&Command::Pounce(1)).expect("fixture: a pounce is priced here");
    for (r, each) in unit {
        state.resources[r.ix()] = each * k as f64;
    }
    state
}

/// The rejection the core gives the call after `done` calls of `command`.
fn rejection_after(state: &GameState, command: Command, done: u32) -> CommandError {
    let mut s = state.clone();
    for i in 0..done {
        s.apply(command).unwrap_or_else(|e| panic!("fixture: call {} of {command:?} was refused: {e:?}", i + 1));
    }
    s.apply(command).expect_err("fixture: the call after the last affordable one is refused")
}

/// The footer a single Enter on the same row, in the same meadow, leaves.
fn enter_footer(state: &GameState, section: Section, key: RowKey) -> String {
    let mut app = app_on(state.clone(), section, key, "enter");
    step(&mut app, &[enter()]);
    footer(&app).expect("fixture: Enter on a row the stock cannot cover leaves a rejection footer")
}

/// A ×N run on a repeat-N row that the stock stops partway states the core's
/// rejection in plain words, and how many of how many were done.
///
/// @spec TUI-NAV-069
#[test]
fn a_repeat_n_count_stopped_partway_says_why_and_how_many_of_how_many() {
    let (state, key, s) = stocked_for_builds(3);
    let why = plain_error(&rejection_after(&state, Command::Build(s), 3));
    let before = state.structures[s.ix()];

    let mut app = app_on(state, Section::Meadow, key, "repeat-partial");
    step(&mut app, &times(10));

    assert_eq!(app.state.structures[s.ix()], before + 3, "fixture: the core took three of ten");
    let said = footer(&app).expect("a ×10 run the core stopped after three left no footer");
    assert!(said.contains(&why), "the run's footer {said:?} does not state the core's rejection {why:?}");
    let on = numbers(&said);
    assert!(
        on.contains("3") && on.contains("10"),
        "the footer {said:?} does not say that 3 of 10 were done"
    );
}

/// A ×N run on a count row stopped partway says the same: the rejection, and
/// how many of how many.
///
/// @spec TUI-NAV-069
#[test]
fn a_count_row_count_stopped_partway_says_why_and_how_many_of_how_many() {
    let state = with_idle(2);
    let (key, assign) = job_row(&state);
    let why = plain_error(&rejection_after(&state, assign, 2));

    let mut app = app_on(state, Section::Clowder, key, "count-partial");
    step(&mut app, &times(5));

    let said = footer(&app).expect("a ×5 run the core stopped after two left no footer");
    assert!(said.contains(&why), "the run's footer {said:?} does not state the core's rejection {why:?}");
    let on = numbers(&said);
    assert!(on.contains("2") && on.contains("5"), "the footer {said:?} does not say that 2 of 5 were done");
}

/// `X` stopped by the core states the rejection and how many were done — and
/// never "of" its cap, which is no count the player asked for. On every kind of
/// row `X` runs on: a repeat-N row, a count-taking row, a count row. On the
/// count-taking row the run takes one squad per call, so it takes as many as
/// the stock covers rather than being refused whole.
///
/// @spec TUI-NAV-069, TUI-NAV-012
#[test]
fn x_max_stopped_by_the_core_says_why_and_how_many() {
    let cap = MAX_AFFORDABLE_CAP.to_string();

    // A repeat-N row.
    let (state, key, s) = stocked_for_builds(4);
    let why = plain_error(&rejection_after(&state, Command::Build(s), 4));
    let before = state.structures[s.ix()];
    let mut app = app_on(state, Section::Meadow, key, "max-build");
    step(&mut app, &[press('X')]);
    assert_eq!(app.state.structures[s.ix()], before + 4, "fixture: X built the four the stock covered");
    let said = footer(&app).expect("X stopped by the core on a repeat-N row left no footer");
    assert!(said.contains(&why), "X's footer {said:?} does not state the core's rejection {why:?}");
    assert!(numbers(&said).contains("4"), "X's footer {said:?} does not say 4 were done");
    assert!(!numbers(&said).contains(&cap), "X's footer {said:?} states its cap as if it were a count");

    // A count-taking row: one squad per call.
    let state = stocked_for_squads(3);
    let key = pounce_row(&state);
    let why = plain_error(&rejection_after(&state, Command::Pounce(1), 3));
    let unit = state.unit_price(&Command::Pounce(1)).expect("fixture: priced");
    let mut app = app_on(state, Section::Clowder, key, "max-pounce");
    step(&mut app, &[press('X')]);
    for (r, _) in &unit {
        assert!(
            app.state.stock(*r) < 1e-6,
            "X on the Pounce row with three squads' price in stock left {} {r:?}; X runs one squad \
             per call, so it takes all three before the core refuses",
            app.state.stock(*r)
        );
    }
    let said = footer(&app).expect("X stopped by the core on a count-taking row left no footer");
    assert!(said.contains(&why), "X's footer {said:?} does not state the core's rejection {why:?}");
    assert!(numbers(&said).contains("3"), "X's footer {said:?} does not say 3 were done");
    assert!(!numbers(&said).contains(&cap), "X's footer {said:?} states its cap as if it were a count");

    // A count row.
    let state = with_idle(3);
    let (key, assign) = job_row(&state);
    let why = plain_error(&rejection_after(&state, assign, 3));
    let mut app = app_on(state, Section::Clowder, key, "max-count");
    step(&mut app, &[press('X')]);
    let said = footer(&app).expect("X stopped by the core on a count row left no footer");
    assert!(said.contains(&why), "X's footer {said:?} does not state the core's rejection {why:?}");
    assert!(numbers(&said).contains("3"), "X's footer {said:?} does not say 3 were done");
    assert!(!numbers(&said).contains(&cap), "X's footer {said:?} states its cap as if it were a count");
}

/// A run refused on its very first call reads exactly as Enter's refusal on
/// the same row does — for ×N on a repeat-N or count row, and for `X` on every
/// kind of row it runs on.
///
/// @spec TUI-NAV-069
#[test]
fn a_run_refused_on_its_first_call_reads_as_enters_refusal() {
    let (build_state, build_key, _) = stocked_for_builds(0);
    let idle_state = with_idle(0);
    let (job_key, _) = job_row(&idle_state);
    let pounce_state = stocked_for_squads(0);
    let pounce_key = pounce_row(&pounce_state);

    let cases: Vec<(&str, &GameState, Section, RowKey, Vec<Event>)> = vec![
        ("×10 on a repeat-N row", &build_state, Section::Meadow, build_key.clone(), times(10)),
        ("×5 on a count row", &idle_state, Section::Clowder, job_key.clone(), times(5)),
        ("X on a repeat-N row", &build_state, Section::Meadow, build_key, vec![press('X')]),
        ("X on a count-taking row", &pounce_state, Section::Clowder, pounce_key, vec![press('X')]),
        ("X on a count row", &idle_state, Section::Clowder, job_key, vec![press('X')]),
    ];

    for (what, state, section, key, keys) in cases {
        let expected = enter_footer(state, section, key.clone());
        let mut app = app_on(state.clone(), section, key, "first-call");
        step(&mut app, &keys);
        assert_eq!(
            footer(&app).as_deref(),
            Some(expected.as_str()),
            "{what}, refused on its first call, did not read as Enter's refusal on the same row"
        );
    }
}

/// A run that reaches its count unrefused leaves no footer — the rows show what
/// it did — and so does an `X` run that reaches its cap, which is the same cap
/// the prompt's count is bounded by: `X` on a Pounce row with stock for far
/// more takes exactly the cap's worth of single squads.
///
/// @spec TUI-NAV-069, TUI-NAV-012
#[test]
fn a_run_that_reaches_its_count_or_the_cap_unrefused_leaves_no_footer() {
    let (state, key, s) = stocked_for_builds(10);
    let before = state.structures[s.ix()];
    let mut app = app_on(state, Section::Meadow, key, "full-build");
    step(&mut app, &times(3));
    assert_eq!(app.state.structures[s.ix()], before + 3, "fixture: ×3 built three");
    assert_eq!(footer(&app), None, "a ×3 run that built all three left a footer");

    let state = with_idle(8);
    let (key, _) = job_row(&state);
    let mut app = app_on(state, Section::Clowder, key, "full-count");
    step(&mut app, &times(4));
    assert_eq!(footer(&app), None, "a ×4 run that assigned all four left a footer");

    let state = rich();
    let key = pounce_row(&state);
    let unit = state.unit_price(&Command::Pounce(1)).expect("fixture: priced");
    let had: Vec<f64> = unit.iter().map(|(r, _)| state.stock(*r)).collect();
    let mut app = app_on(state, Section::Clowder, key, "full-max");
    step(&mut app, &[press('X')]);
    assert_eq!(footer(&app), None, "an X run that reached its cap unrefused left a footer");
    for ((r, each), had) in unit.iter().zip(had) {
        let spent = had - app.state.stock(*r);
        let expected = each * MAX_AFFORDABLE_CAP as f64;
        assert!(
            (spent - expected).abs() <= expected * 1e-9,
            "X on the Pounce row spent {spent} {r:?}; the cap's worth of single squads is {expected}"
        );
    }
}

// ===========================================================================
// TUI-NAV-073 — the inspector's one-unit price
// ===========================================================================

const TALL: u16 = 40;
const WIDE: u16 = 200;

fn pane_text(state: &GameState, view: &ViewState) -> String {
    let rect = match compute(Rect::new(0, 0, WIDE, TALL), None, false).body {
        Body::Wide { inspector, .. } => inspector,
        other => panic!("fixture: {WIDE}x{TALL} should lay out three columns, but laid out {other:?}"),
    };
    let mut term = Terminal::new(TestBackend::new(WIDE, TALL)).expect("TestBackend is infallible");
    term.draw(|f| inspector::draw(f, rect, state, view)).expect("TestBackend is infallible");
    buffer_text(term.backend().buffer(), rect)
}

/// On a focused Pounce or Trade row, the inspector's next cost is the price of
/// one unit — one squad, or one caravan to that row's colony — as the core
/// reports it, per component with have and need. Quantified over the Pounce row
/// and every colony's trade row, with each component's stock set to a distinct,
/// exactly representable amount so have and need can each be found.
///
/// @spec TUI-NAV-073
#[test]
fn a_per_unit_priced_rows_next_cost_is_the_cores_price_of_one_unit() {
    const STOCKS: [f64; 4] = [4_096.5, 0.25, 777.75, 31.125];
    let base = rich();
    let mut seen = 0;

    for (section, row) in every_row(&base) {
        let RowKind::Act { command, .. } = row.kind else { continue };
        if !matches!(command, Command::Pounce(_) | Command::Trade(..)) {
            continue;
        }
        seen += 1;
        let unit = base
            .unit_price(&command)
            .unwrap_or_else(|| panic!("fixture: {command:?} is open here, so the core prices it"));
        let mut state = base.clone();
        for (i, (r, _)) in unit.iter().enumerate() {
            state.resources[r.ix()] = STOCKS[i % STOCKS.len()];
        }
        assert_eq!(state.unit_price(&command), Some(unit.clone()), "fixture: stock does not move the price");

        let shown = pane_text(&state, &focused(section, row.key.clone()));
        let on_screen = numbers(&shown);
        for (r, need) in &unit {
            assert!(
                on_screen.contains(&fmt::exact(*need)),
                "the inspector on {:?} does not state the core's one-unit price of {r:?} ({}); it \
                 read:\n{shown}",
                row.key,
                fmt::exact(*need)
            );
            assert!(
                on_screen.contains(&fmt::exact(state.stock(*r))),
                "the inspector on {:?} does not state what the player has of {r:?} ({}) beside \
                 what one unit needs; it read:\n{shown}",
                row.key,
                fmt::exact(state.stock(*r))
            );
        }
    }
    assert!(seen > 1, "fixture: the Pounce row and the trade rows should be listed");
}
