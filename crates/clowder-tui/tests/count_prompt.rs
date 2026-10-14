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
    for (i, b) in StructureId::ALL.into_iter().enumerate() {
        // Varied rather than uniform: an escalating price scales with the
        // count owned, and every structure owned the same count otherwise
        // makes unrelated structures' prices land on the same figure by
        // construction rather than by chance — a coincidence a numbers-on-
        // screen check (TUI-NAV-067's tests) cannot tell apart from a bug.
        s.structures[b.ix()] = 5 + (i % 4) as u32;
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
/// typed that were not there with the prompt open and empty. "On screen"
/// reads the wide frame's list (whose persistent inspector never substitutes
/// the prompt's preview, TUI-NAV-035) together with the narrow swap, whose
/// inspector always carries the preview whole — since a component
/// TUI-RENDER-021 cuts from the list's own row for room is still one
/// keypress away in the swap, never off screen everywhere at once.
fn typed_numbers(state: &GameState, section: Section, row: &Row, digits: &str) -> (BTreeSet<String>, BTreeSet<String>, String) {
    let empty = format!(
        "{}\n{}",
        frame_text(state, &prompt_on(section, row, "", state)),
        swapped_text(state, &swapped_prompt_on(section, row, "", state))
    );
    let typed = format!(
        "{}\n{}",
        frame_text(state, &prompt_on(section, row, digits, state)),
        swapped_text(state, &swapped_prompt_on(section, row, digits, state))
    );
    let before = numbers(&empty);
    let after = numbers(&typed);
    let added: BTreeSet<String> = after.difference(&before).cloned().collect();
    (after, added, typed)
}

/// A count-alone preview: the count is on screen, and nothing else the count
/// added is a number.
fn assert_count_alone(state: &GameState, section: Section, row: &Row, what: &str) {
    // A kitten is one row now (TUI-NAV-093), not two, so a fixture roster that
    // used to run too deep for `MEADOW_MOVE_MIN_KITTENS + 9` kittens to fit
    // the frame's viewport at `assert_count_alone`'s test height now often
    // does — and a kitten is named `kitten-{i}`, so a marker equal to any
    // roster index risks reading the same as a kitten's own name on screen.
    // 823 sits well past the roster this fixture ever grows to.
    const N: &str = "823";
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
// TUI-NAV-035 — the preview while the inspector holds the list's space
// ===========================================================================

/// The narrow-mode swap's terminal: under the pane breakpoint, so the
/// inspector takes the list's space, and tall enough that nothing the
/// inspector says is cut for room.
const SWAP_W: u16 = 99;
const SWAP_H: u16 = 60;

/// The text of the list's space — where the swapped inspector draws — for
/// this view, in a cold frame at the swap's size.
fn swapped_text(state: &GameState, view: &ViewState) -> String {
    let area = Rect::new(0, 0, SWAP_W, SWAP_H);
    let Body::Narrow { main, .. } = compute(area, None, false).body else {
        panic!("fixture: {SWAP_W} columns should lay out narrow");
    };
    let mut term = Terminal::new(TestBackend::new(SWAP_W, SWAP_H)).expect("TestBackend is infallible");
    term.draw(|f| {
        clowder_tui::render::frame(f, state, view, None);
    })
    .expect("TestBackend is infallible");
    buffer_text(term.backend().buffer(), main)
}

/// Open the prompt on `row` from inside the swap, as a player does, with the
/// keys a player might press on the way — Space and i among them, which the
/// open prompt takes as it takes any key but a digit — and type `digits`.
fn swapped_prompt_on(section: Section, row: &Row, digits: &str, state: &GameState) -> ViewState {
    let mut view = focused(section, row.key.clone());
    view.inspector_wanted = true;
    update(Action::CountPrompt, &mut view, state);
    assert!(view.prompt.is_some(), "fixture: x on {:?} in the swap opened no count prompt", row.key);
    for c in [' ', 'i'].into_iter().chain(digits.chars()) {
        update(Action::Type(c), &mut view, state);
    }
    assert_eq!(
        view.prompt.as_ref().map(|p| p.digits.as_str()),
        Some(digits),
        "fixture: the prompt on {:?} holds what was typed",
        row.key
    );
    view
}

/// While the inspector holds the list's space and describes the row an open
/// count prompt is held to, it shows the prompt's count and its preview — the
/// core's total on a repeat-N row, N times the one-unit price on a Pounce or
/// Trade row, the count alone on the nap row and a count row — and the swap
/// stays in: opening the prompt, and Space or i pressed into it, leave it.
/// Quantified over every row the prompt opens on.
///
/// @spec TUI-NAV-035
#[test]
fn the_swapped_inspector_shows_an_open_prompts_count_and_preview() {
    const N: &str = "37";
    const COUNT: u32 = 37;
    let state = rich();
    let rows_seen = prompt_rows(&state);
    assert!(!rows_seen.is_empty(), "fixture: no row takes a count");

    for (section, row) in rows_seen {
        let empty = swapped_prompt_on(section, &row, "", &state);
        let typed = swapped_prompt_on(section, &row, N, &state);
        assert!(
            typed.inspector_wanted,
            "opening the prompt on {:?} in the swap, and pressing Space and i into it, took the \
             swap away; the open prompt takes those keys, and the swap stays until it closes",
            row.key
        );

        let before = numbers(&swapped_text(&state, &empty));
        let frame = swapped_text(&state, &typed);
        let after = numbers(&frame);
        assert!(
            !before.contains(N),
            "fixture: {N} is already in the swapped inspector for {:?}, so the count cannot be \
             told apart",
            row.key
        );
        let added: BTreeSet<String> = after.difference(&before).cloned().collect();
        assert!(
            after.contains(N),
            "with {N} typed into the prompt on {:?}, the inspector holding the list's space does \
             not show the count.\n{frame}",
            row.key
        );

        let priced: Option<Vec<(ResourceId, f64)>> = match &row.kind {
            RowKind::Act { semantics: CountSemantics::RepeatN, .. } => match row.key {
                RowKey::Content(id, _) => Some(state.cost_of_next(id, COUNT)),
                _ => None,
            },
            RowKind::Act { command, semantics: CountSemantics::CountTaking } if !row.heavy => state
                .unit_price(command)
                .map(|unit| unit.into_iter().map(|(r, each)| (r, each * COUNT as f64)).collect()),
            _ => None,
        };
        match priced.filter(|p| !p.is_empty()) {
            Some(total) => {
                for (r, amount) in &total {
                    assert!(
                        shows_amount(&after, *amount),
                        "with {N} typed into the prompt on {:?}, the preview's {} {r:?} is not in \
                         the inspector holding the list's space.\n{frame}",
                        row.key,
                        fmt::exact(*amount)
                    );
                }
                assert!(
                    total.iter().any(|(_, a)| renderings(*a).iter().any(|t| added.contains(t))),
                    "with {N} typed into the prompt on {:?}, nothing of the preview appeared in the \
                     swapped inspector that was not already there.\n{frame}",
                    row.key
                );
            }
            None => {
                let extra: Vec<&String> = added.iter().filter(|t| t.as_str() != N).collect();
                assert!(
                    extra.is_empty(),
                    "with {N} typed into the prompt on {:?}, the count brought {extra:?} to the \
                     swapped inspector as well; the preview there is the count alone.\n{frame}",
                    row.key
                );
            }
        }
    }
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
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir =
        std::env::temp_dir().join(format!("clowder-count-{}-{name}-{n}", std::process::id()));
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
        pointer_record: None,
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

/// The press of `X`, as the terminal delivers it.
fn big_x() -> Event {
    press('X')
}

/// One frame with these keys, `secs` of time having passed since the last.
fn step_after(app: &mut App, secs: f64, input: &[Event]) {
    let mut term = Terminal::new(TestBackend::new(120, 40)).expect("TestBackend is infallible");
    let d = Duration::from_secs_f64(secs);
    app.step(&mut term, Reading { monotonic: d, wall: d }, input)
        .expect("an in-memory step does not fail");
}

/// `state` after a run of `command` made the way the spec has it: every call
/// against the state the call before it left, stopping at the first the core
/// refuses or after `n`. Returns the state and how many calls the core took.
fn run_at_the_press(state: &GameState, command: Command, n: u32) -> (GameState, u32) {
    let mut s = state.clone();
    let mut done = 0;
    for _ in 0..n {
        if s.apply(command).is_err() {
            break;
        }
        done += 1;
    }
    (s, done)
}

/// Advance `state` by what one frame of `secs` owes on `clock`, as the loop
/// does after the frame's input.
fn tick_frame(state: &mut GameState, clock: &mut TickAccumulator, secs: f64) {
    let d = Duration::from_secs_f64(secs);
    clock.observe(Reading { monotonic: d, wall: d }, state.speed);
    for _ in 0..clock.take(2_000) {
        state.tick();
    }
}

/// A run is made whole at the press: every call against the state the call
/// before it left, the simulation not advancing between them, and nothing of
/// the run left to spend what arrives in the frames that follow. Checked on
/// every kind of run — ×N on a repeat-N row and on a count row, and `X` on a
/// repeat-N, a count-taking, and a count row — in frames that owe ticks, by
/// holding the meadow the loop leaves to one in which the whole run was made
/// first and only then the frame's time passed, frame after frame.
///
/// @spec TUI-NAV-012, TUI-NAV-031
#[test]
fn every_run_is_made_whole_at_the_press_with_no_tick_between_its_calls() {
    let (builds, build_key, s) = stocked_for_builds(3);
    let (builds_x, build_key_x, _) = stocked_for_builds(4);
    let idle = with_idle(2);
    let (job_key, assign) = job_row(&idle);
    let idle_x = with_idle(3);
    let squads = stocked_for_squads(3);
    let squad_key = pounce_row(&squads);

    type Case = (&'static str, GameState, Section, RowKey, Vec<Event>, Command, u32);
    let cases: Vec<Case> = vec![
        ("×5 on a repeat-N row", builds, Section::Meadow, build_key, times(5), Command::Build(s), 5),
        ("×4 on a count row", idle, Section::Clowder, job_key.clone(), times(4), assign, 4),
        ("X on a repeat-N row", builds_x, Section::Meadow, build_key_x, vec![big_x()], Command::Build(s), MAX_AFFORDABLE_CAP),
        ("X on a count-taking row", squads, Section::Clowder, squad_key, vec![big_x()], Command::Pounce(1), MAX_AFFORDABLE_CAP),
        ("X on a count row", idle_x, Section::Clowder, job_key, vec![big_x()], assign, MAX_AFFORDABLE_CAP),
    ];

    for (what, state, section, key, keys, command, n) in cases {
        let (mut expected, done) = run_at_the_press(&state, command, n);
        assert!(done > 0, "fixture: {what} should take at least one call");
        let mut clock = TickAccumulator::default();
        let opened_at = state.ticks;

        let mut app = app_on(state, section, key, "at-the-press");
        step_after(&mut app, 3.0, &keys);
        tick_frame(&mut expected, &mut clock, 3.0);
        assert!(app.state.ticks > opened_at, "fixture: the frame for {what} ran no ticks");
        assert!(
            app.state == expected,
            "{what}: the meadow after the frame is not the whole run made at the press and then \
             the frame's ticks — a call was made against a state the sim had moved, or not made"
        );

        for frame in 1..=3 {
            step_after(&mut app, 3.0, &[]);
            tick_frame(&mut expected, &mut clock, 3.0);
            assert!(
                app.state == expected,
                "{what}: frame {frame} after the press differs from the ticks alone; the run went \
                 on spending after the press"
            );
        }
    }
}

/// A held `X` delivering several presses into one frame makes one run: the
/// repeats that arrive within a tenth of a second of it are dropped with no
/// footer note, so the footer the frame leaves is the run's own stop — never a
/// later repeat's refusal written over it. On every kind of row `X` runs on.
///
/// @spec TUI-NAV-012
#[test]
fn a_held_x_within_one_frame_makes_one_run_and_leaves_its_footer_standing() {
    let (builds, build_key, s) = stocked_for_builds(4);
    let squads = stocked_for_squads(3);
    let squad_key = pounce_row(&squads);
    let idle = with_idle(3);
    let (job_key, assign) = job_row(&idle);

    let cases: Vec<(&str, GameState, Section, RowKey, Command)> = vec![
        ("a repeat-N row", builds, Section::Meadow, build_key, Command::Build(s)),
        ("a count-taking row", squads, Section::Clowder, squad_key, Command::Pounce(1)),
        ("a count row", idle, Section::Clowder, job_key, assign),
    ];

    for (what, state, section, key, command) in cases {
        let (expected, _) = run_at_the_press(&state, command, MAX_AFFORDABLE_CAP);

        let mut once = app_on(state.clone(), section, key.clone(), "held-once");
        step(&mut once, &[big_x()]);
        let single = footer(&once).expect("fixture: X stopped by the core leaves a footer");

        let mut held = app_on(state, section, key, "held-many");
        step(&mut held, &[big_x(), big_x(), big_x(), big_x()]);
        assert!(held.state == expected, "a held X on {what} did not make exactly one whole run");
        assert_eq!(
            footer(&held).as_deref(),
            Some(single.as_str()),
            "a held X on {what} left a footer other than its one run's stop; a repeat within a \
             tenth of a second is dropped with no footer note"
        );
    }
}

/// How many copies of an item a buy takes is the core's per-item refusal to
/// decide, never the tag's: ×3 on every buy row the player does not already
/// own makes three calls at the press, and the meadow the loop leaves is the
/// one the core makes of three calls in a row, stopping where it first refuses.
///
/// @spec TUI-NAV-031
#[test]
fn how_many_copies_a_buy_takes_is_the_cores_per_item_refusal() {
    let base = rich();
    let mut kinds: BTreeSet<&str> = BTreeSet::new();
    let mut took_any = 0;

    for (section, row) in every_row(&base) {
        let RowKind::Act { command, .. } = row.kind else { continue };
        let kind = match command {
            Command::BuyRefinement(_) => "refinement",
            Command::BuyRite(_) => "rite",
            Command::BuyArch(_) => "arch",
            Command::BuyDreamLoom(_) => "dream loom",
            Command::BuyTradition(_) => "tradition",
            _ => continue,
        };
        if let RowKey::Content(id, _) = &row.key {
            if base.lock_reason(*id) == Some(LockReason::AlreadyYours) {
                continue;
            }
        }
        kinds.insert(kind);

        let (expected, done) = run_at_the_press(&base, command, 3);
        if done > 0 {
            took_any += 1;
        }
        let mut app = app_on(base.clone(), section, row.key.clone(), "buy");
        step(&mut app, &times(3));
        assert!(
            app.state == expected,
            "×3 on the buy row {:?} did not leave the meadow three calls in a row make of it \
             (the core took {done}); a buy is repeat-called and the core's refusal decides how \
             many it takes",
            row.key
        );
    }

    assert_eq!(
        kinds,
        BTreeSet::from(["arch", "dream loom", "refinement", "rite", "tradition"]),
        "fixture: an unowned row of every buy command must be listed"
    );
    assert!(took_any > 0, "fixture: some buy the core accepts at least once");
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

/// A drawing with every number replaced by one placeholder and whitespace
/// collapsed: two drawings that mask alike said the same things about
/// different amounts.
fn masked(s: &str) -> String {
    let mut out = String::new();
    let mut in_number = false;
    for c in s.chars() {
        if c.is_ascii_digit() || (in_number && c == '.') {
            if !in_number {
                out.push('#');
                in_number = true;
            }
        } else {
            in_number = false;
            out.push(c);
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The core reports no lock reason for a Pounce or Trade row, and only a lock
/// reason marks components, so the one-unit price's need and have stand
/// unmarked and no wait stands beside them: the row reads the same whether the
/// stock covers the unit or falls short, and the game speed — which would move
/// any real-time wait — changes nothing on it. Quantified over the Pounce row
/// and every colony's trade row.
///
/// @spec TUI-NAV-078
#[test]
fn a_per_unit_priced_rows_components_stand_unmarked_with_no_wait() {
    let base = rich();
    let mut seen = 0;
    let mut growing = 0;
    for (section, row) in every_row(&base) {
        let RowKind::Act { command, .. } = row.kind else { continue };
        if !matches!(command, Command::Pounce(_) | Command::Trade(..)) {
            continue;
        }
        seen += 1;
        let unit = base
            .unit_price(&command)
            .unwrap_or_else(|| panic!("fixture: {command:?} is open here, so the core prices it"));
        let mut short = base.clone();
        let mut enough = base.clone();
        for (r, need) in &unit {
            short.resources[r.ix()] = need * 0.25;
            enough.resources[r.ix()] = need + 100.5;
        }
        if unit.iter().any(|(r, _)| short.net_rate(*r) > 0.0) {
            growing += 1;
        }
        let view = focused(section, row.key.clone());
        let a = pane_text(&short, &view);
        let b = pane_text(&enough, &view);
        assert_eq!(
            masked(&a),
            masked(&b),
            "{:?}: stock marks nothing on a one-unit price, so short and covered must read alike but \
             for their figures.\nshort:\n{a}\ncovered:\n{b}",
            row.key
        );
        let mut fast = short.clone();
        fast.speed = 8.0;
        let mut slow = short.clone();
        slow.speed = 0.25;
        assert_eq!(
            pane_text(&fast, &view),
            pane_text(&slow, &view),
            "{:?}: no wait stands beside a one-unit price, so the speed changes nothing on the row",
            row.key
        );
    }
    assert!(seen > 1, "fixture: the Pounce row and the trade rows should be listed");
    assert!(
        growing > 0,
        "fixture: on some of these rows a component is growing, so a wait would have something to say"
    );
}

// ===========================================================================
// TUI-RENDER-021 — the prompt's row
// ===========================================================================

use std::collections::HashMap;

/// The list's space in a drawn frame, its pane divider(s) set aside.
fn list_space_of(l: &clowder_tui::render::layout::Layout) -> Rect {
    match l.body {
        Body::Narrow { main, .. } => Rect::new(main.x + 1, main.y, main.width - 1, main.height),
        Body::Wide { list, .. } => Rect::new(list.x + 1, list.y, list.width - 2, list.height),
        Body::TooSmall(r) => panic!("fixture: {r:?} is below the floor"),
    }
}

/// A drawn frame's list space, one vector of single-cell symbols per line.
fn list_cells(state: &GameState, view: &ViewState, (w, h): (u16, u16)) -> Vec<Vec<String>> {
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("TestBackend is infallible");
    let mut layout = None;
    term.draw(|f| layout = Some(clowder_tui::render::frame(f, state, view, None)))
        .expect("TestBackend is infallible");
    let space = list_space_of(&layout.expect("a layout"));
    let buf = term.backend().buffer();
    (space.y..space.bottom())
        .map(|y| {
            (space.x..space.right())
                .map(|x| buf.cell((x, y)).map(|c| c.symbol().to_string()).unwrap_or_else(|| " ".into()))
                .collect()
        })
        .collect()
}

/// The label a row draws in its name's place, where it is a later row of an
/// item that carries several (TUI-NAV-089): keyed by the row's own tag, and
/// only where an earlier row of the same item is listed before it.
fn later_row_label(state: &GameState, section: Section, key: &RowKey) -> Option<&'static str> {
    let label = match key {
        RowKey::Content(ContentId::Craft(_), "tinker") => "tinkerers",
        RowKey::Kitten(_, "elder") => "appoint Elder",
        RowKey::Colony(_, "penpal") => "send a pen-pal",
        _ => return None,
    };
    let same_item = |k: &RowKey| match (k, key) {
        (RowKey::Content(a, _), RowKey::Content(b, _)) => a == b,
        (RowKey::Kitten(a, _), RowKey::Kitten(b, _)) => a == b,
        (RowKey::Colony(a, _), RowKey::Colony(b, _)) => a == b,
        _ => false,
    };
    rows(state, section).iter().find(|r| same_item(&r.key)).filter(|first| &first.key != key).map(|_| label)
}

/// What a row draws in its name's place, in full: on a later row of an item
/// that carries several, that row's label (TUI-NAV-089); on any other row, the
/// name the inspector's first line states.
fn full_row_name(state: &GameState, section: Section, key: &RowKey) -> String {
    if let Some(label) = later_row_label(state, section, key) {
        return label.to_string();
    }
    let rect = Rect::new(0, 0, 180, 6);
    let mut term = Terminal::new(TestBackend::new(180, 6)).expect("TestBackend is infallible");
    term.draw(|f| inspector::draw(f, rect, state, &focused(section, key.clone())))
        .expect("TestBackend is infallible");
    buffer_text(term.backend().buffer(), rect).lines().next().unwrap_or("").trim().to_string()
}

/// Every resource's stores-row spelling.
fn stores_row_names() -> HashMap<ResourceId, String> {
    let mut s = GameState::new(11);
    for r in ResourceId::ALL {
        s.resources[r.ix()] = 1.0;
    }
    ResourceId::ALL
        .into_iter()
        .map(|r| (r, full_row_name(&s, Section::Meadow, &RowKey::Resource(r))))
        .collect()
}

/// Whether a section's list gives an owned-count field (TUI-RENDER-020),
/// judged by the section alone.
fn section_counts(section: Section) -> bool {
    matches!(section, Section::Meadow | Section::Clowder | Section::Crafts | Section::Sunbeams)
}

/// The owned count the inspector states for a row, if it states one.
fn owned_stated(state: &GameState, row: &Row) -> Option<u64> {
    match &row.key {
        RowKey::Content(ContentId::Structure(b), _) => Some(state.structures[b.ix()] as u64),
        RowKey::Content(ContentId::Rite(r), _) => Some(state.rite_counts[r.ix()] as u64),
        RowKey::Content(ContentId::Job(j), _) if matches!(row.kind, RowKind::Count { .. }) => {
            Some(state.effective_workers(*j) as u64)
        }
        RowKey::Content(ContentId::Craft(c), "tinker") => Some(
            state.kittens.iter().filter(|k| k.tinkering == Some(*c) && k.location == Location::Home).count()
                as u64,
        ),
        _ => None,
    }
}

/// The components TUI-NAV-067's preview prices for `digits`, or none where
/// it is the count alone or nothing.
fn preview_components(state: &GameState, row: &Row, digits: &str) -> Vec<(ResourceId, f64)> {
    let n: u32 = digits.parse().unwrap_or(0);
    if n == 0 {
        return Vec::new();
    }
    match &row.kind {
        RowKind::Act { semantics: CountSemantics::RepeatN, .. } => match row.key {
            RowKey::Content(id, _) => state.cost_of_next(id, n),
            _ => Vec::new(),
        },
        RowKind::Act { command, semantics: CountSemantics::CountTaking } if !row.heavy => state
            .unit_price(command)
            .map(|unit| unit.into_iter().map(|(r, each)| (r, each * n as f64)).collect())
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// The prompt's text: `×` and the digits as typed, then — where the preview
/// prices components — two blank columns and the components, each an exact
/// figure, a space, and the stores-row name, apart by `, `.
fn prompt_text(parts: &[String], digits: &str) -> String {
    if parts.is_empty() {
        format!("×{digits}")
    } else {
        format!("×{digits}  {}", parts.join(", "))
    }
}

/// The prompt's text cut to a place `place` columns wide: whole where it
/// fits; else the count whole and the longest leading run of components that
/// fits, `, …` after it; else the count, two blank columns, and `…`.
fn prompt_text_in(parts: &[String], digits: &str, place: usize) -> String {
    let whole = prompt_text(parts, digits);
    if whole.chars().count() <= place {
        return whole;
    }
    for k in (1..parts.len()).rev() {
        let run = format!("×{digits}  {}, …", parts[..k].join(", "));
        if run.chars().count() <= place {
            return run;
        }
    }
    format!("×{digits}  …")
}

/// While a count prompt is open on a listed row, that row draws the prompt's
/// text from the left edge of its cost field's place — or of the place one
/// would take — `×` joined to the digits exactly as typed, then two blank
/// columns and the priced components with exact figures; longer text moves
/// the count field (keeping its figure and seven columns) and the name left,
/// the name shortening to one character and its ellipsis at most; text that
/// still does not fit keeps the count whole and draws whole components then
/// `, …`, or the count, two blank columns, and `…`; every other row keeps its
/// columns; and the narrow swap's inspector carries the same text. Quantified
/// over every row the prompt opens on, over counts empty, zero, zero-led,
/// small, and past any real price, at the floor and wide.
/// @spec TUI-RENDER-021, TUI-RENDER-012
#[test]
fn an_open_prompt_draws_its_count_and_exact_preview_in_the_cost_fields_place() {
    const DIGITS: [&str; 6] = ["", "0", "007", "37", "250", "10000"];
    const SIZES: [(u16, u16); 2] = [(80, 24), (200, 60)];
    let state = rich();
    let names = stores_row_names();
    let c = 6 + 1 + names.values().map(|n| n.chars().count()).max().expect("a vocabulary") + 3;
    let targets = prompt_rows(&state);
    assert!(!targets.is_empty(), "fixture: some row takes a count");

    let mut seen = (0usize, 0usize, 0usize, 0usize); // fits, moves, cut, beyond reach
    for (section, row) in &targets {
        let full = full_row_name(&state, *section, &row.key);
        let counted = section_counts(*section);
        for size in SIZES {
            let closed = list_cells(&state, &focused(*section, row.key.clone()), size);
            let width = closed[0].len();
            for digits in DIGITS {
                let view = prompt_on(*section, row, digits, &state);
                let open = list_cells(&state, &view, size);
                let cy = open
                    .iter()
                    .position(|l| l[0] == "▸")
                    .unwrap_or_else(|| panic!("fixture: the prompt's row is on screen for {:?}", row.key));
                let at = format!("{:?} with {digits:?} typed, at {size:?}", row.key);
                let drawn = open[cy].concat();

                for (y, (o, cl)) in open.iter().zip(&closed).enumerate() {
                    if y != cy {
                        assert_eq!(
                            o.concat(),
                            cl.concat(),
                            "{at}: line {y} changed; only the prompt's row gives way, and every other \
                             row keeps its columns (TUI-RENDER-021)"
                        );
                    }
                }
                assert_eq!(cells(&open[cy], 0, 4), cells(&closed[cy], 0, 4), "{at}: the cursor and marker stay");

                let comps = preview_components(&state, row, digits);
                let parts: Vec<String> =
                    comps.iter().map(|(r, a)| format!("{} {}", fmt::exact(*a), names[r])).collect();
                if comps.iter().any(|(_, a)| !a.is_finite()) {
                    seen.3 += 1;
                }
                let text = prompt_text(&parts, digits);
                let len = text.chars().count();
                let base_name = width - 4 - if counted { 8 } else { 0 } - 1 - c;
                let (name_w, place, expected) = if len <= c {
                    seen.0 += 1;
                    (base_name, c, format!("{text:<c$}"))
                } else if base_name >= 2 + (len - c) {
                    seen.1 += 1;
                    (base_name - (len - c), len, text.clone())
                } else {
                    seen.2 += 1;
                    let place = c + base_name - 2;
                    (2, place, format!("{:<place$}", prompt_text_in(&parts, digits, place)))
                };

                assert_eq!(
                    cells(&open[cy], width - place, width),
                    expected,
                    "{at}: the prompt's text in the cost field's place, begun at its left edge — \
                     `×` and the digits as typed, then two blank columns and the priced components \
                     in exact figures — moving the fields left where it is longer, and cut to whole \
                     components and `, …` where it still does not fit (TUI-RENDER-021).\n{drawn:?}"
                );
                assert_eq!(
                    cells(&open[cy], 4, 4 + name_w),
                    format!("{:<name_w$}", fmt::fit(&full, name_w)),
                    "{at}: the name takes what the prompt's text leaves, one character and its \
                     ellipsis at least (TUI-RENDER-021).\n{drawn:?}"
                );
                assert_eq!(cells(&open[cy], 4 + name_w, 5 + name_w), " ", "{at}: a blank column after the name.\n{drawn:?}");
                if counted {
                    let count = match owned_stated(&state, row) {
                        Some(n) if n > 0 => format!("{:>7}", format!("×{}", fmt::compact(n as f64))),
                        _ => " ".repeat(7),
                    };
                    assert_eq!(
                        cells(&open[cy], 5 + name_w, 12 + name_w),
                        count,
                        "{at}: the count field keeps its figure and its seven columns, moved left by \
                         what the text needs (TUI-RENDER-021).\n{drawn:?}"
                    );
                    assert_eq!(cells(&open[cy], 12 + name_w, 13 + name_w), " ", "{at}: a blank column before the text.\n{drawn:?}");
                }
            }
        }

        // The narrow swap's inspector carries the same text, whole.
        for digits in DIGITS {
            let view = swapped_prompt_on(*section, row, digits, &state);
            // The swap's space, its divider column set aside.
            let shown: String = swapped_text(&state, &view)
                .lines()
                .map(|l| l.strip_prefix('│').unwrap_or(l))
                .collect::<Vec<_>>()
                .join("\n");
            let comps = preview_components(&state, row, digits);
            let parts: Vec<String> = comps.iter().map(|(r, a)| format!("{} {}", fmt::exact(*a), names[r])).collect();
            let text = prompt_text(&parts, digits);
            let inner = shown.lines().next().map(|l| l.chars().count()).unwrap_or(0);
            // A text longer than a line wraps, a long figure mid-digit, so the
            // fallback reads it with every break and blank taken out.
            let flat = |s: &str| s.split_whitespace().collect::<String>();
            if text.chars().count() <= inner {
                assert!(
                    shown.lines().any(|l| l.trim() == text),
                    "{:?} with {digits:?} typed, in the narrow swap: the inspector carries the \
                     prompt's text {text:?} (TUI-RENDER-021).\n{shown}",
                    row.key
                );
            } else {
                assert!(
                    flat(&shown).contains(&flat(&text)),
                    "{:?} with {digits:?} typed, in the narrow swap: the inspector carries the \
                     prompt's text {text:?} (TUI-RENDER-021).\n{shown}",
                    row.key
                );
            }
            if comps.iter().any(|(_, a)| !a.is_finite()) {
                let lower = shown.to_lowercase();
                assert!(
                    lower.contains("beyond reach")
                        && !lower.split(|ch: char| !ch.is_alphabetic()).any(|w| w == "inf" || w == "nan"),
                    "{:?} with {digits:?} typed: a preview figure that is not finite is stated in \
                     words as beyond reach, never printed (TUI-RENDER-018).\n{shown}",
                    row.key
                );
            }
        }
    }
    assert!(seen.0 > 0 && seen.1 > 0 && seen.2 > 0, "fixture: text that fits, moves, and is cut: {seen:?}");
    assert!(seen.3 > 0, "fixture: some preview figure past any real price is not finite");
}

fn cells(line: &[String], from: usize, to: usize) -> String {
    line[from..to].concat()
}
