//! The armed row on screen — the consequence summary a heavy row expands to
//! when armed, where it is drawn, and that it is gone the moment the arming
//! ends.
//!
//! Everything here is asserted on whole frames drawn by `render::frame` into
//! an in-memory terminal, because the claims are about what the player sees:
//! the summary is lines beneath the armed row in the list (or the inspector,
//! where the narrow-mode swap shows it in the list's place), and "nothing on
//! screen still shows it armed" is a claim about every cell.
//!
//! **Finding the summary.** A frame with the row armed is compared against
//! the same frame with nothing armed. The lines above the row must be
//! identical; the armed row's own line is the cursor line; the summary is the
//! run of lines between it and the next row's line as the unarmed frame draws
//! it. That reads the expansion out of the drawing without guessing at its
//! wording or its layout.
//!
//! **Checking a figure.** A figure is stated when the summary carries it as a
//! number of its own — delimited, not a digit run inside another number — in
//! any of the forms the shell's formatters produce for it (compact, exact, or
//! the plain integer). The figures themselves come from the core's own
//! queries, never from a rule restated here.
//!
//! Scroll-keeping of the expanded lines near the viewport's bottom is deferred
//! to the render leaf (navigation design, Deferred 5), so the placement claims
//! at small sizes are made on rows near the top of their lists, and the
//! AdoptRule rows — last in a long list — are checked on a terminal tall
//! enough that nothing scrolls.
//!
//! @spec TUI-NAV-015, TUI-NAV-075, TUI-NAV-077

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use clowder_core::{
    content, Command, ContentId, GameState, IdeaId, Kitten, KittenTrait, Location, ResourceId,
    RuleId, StructureId, TraditionId,
};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::Terminal;

use clowder_tui::nav::rows::{rows, RowKind};
use clowder_tui::nav::{update, Action, ArmedRow, Outcome, RowKey, Section, ViewState};
use clowder_tui::render;
use clowder_tui::render::fmt;
use clowder_tui::render::layout::{Body, Layout};

// ---------------------------------------------------------------------------
// Sizes
// ---------------------------------------------------------------------------

/// The floor, the top of the narrow range, the first pane width, the list
/// cap, and one past both breakpoints.
const SIZES: [(u16, u16); 5] = [(80, 24), (99, 24), (100, 30), (120, 40), (150, 50)];

/// Narrow sizes, where the inspector is a swap in the list's place.
const NARROW: [(u16, u16); 3] = [(80, 24), (99, 24), (99, 40)];

/// Wide enough for the pane layout, and tall enough that no list here scrolls.
fn tall(state: &GameState) -> (u16, u16) {
    let longest = Section::ALL.iter().map(|s| rows(state, *s).len()).max().unwrap_or(0);
    (150, (longest + 40).min(u16::MAX as usize) as u16)
}

// ---------------------------------------------------------------------------
// Fixture
// ---------------------------------------------------------------------------

const HOME: usize = 76;
const AWAY: usize = 7;
const YEAR: u64 = 41;
const WISDOM: f64 = 4321.0;

fn kitten(location: Location) -> Kitten {
    Kitten {
        name: String::new(),
        job: None,
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

/// A meadow where every heavy row is listed, every heavy action is one the
/// core would accept, and each figure a summary states is distinct from the
/// others: a clowder with some of its kittens away, a basket with room, a
/// chest carrying a share, an elder-wisdom stock, and a calendar year.
fn meadow() -> GameState {
    let mut s = GameState::new(7);
    s.ticks = (YEAR * content::DAYS_PER_YEAR + 50) * content::TICKS_PER_DAY;
    for r in ResourceId::ALL {
        s.resources[r.ix()] = 1.0e12;
    }
    for b in StructureId::ALL {
        s.structures[b.ix()] = 5;
    }
    s.structures[StructureId::MemoryChest.ix()] = 1;
    s.ideas = IdeaId::ALL.iter().copied().collect::<BTreeSet<_>>();
    s.revealed = StructureId::ALL
        .iter()
        .map(|b| ContentId::Structure(*b))
        .chain(IdeaId::ALL.iter().map(|i| ContentId::Idea(*i)))
        .collect::<BTreeSet<_>>();
    s.generation = 1;
    s.traditions = [
        TraditionId::TravelingBasketsI,
        TraditionId::TravelingBasketsII,
        TraditionId::TravelingBasketsIII,
        TraditionId::MemoryChestI,
        TraditionId::MemoryChestII,
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    s.kittens = (0..HOME)
        .map(|_| kitten(Location::Home))
        .chain((0..AWAY).map(|_| kitten(Location::NeighborsYard)))
        .collect();
    s.roster_high_water = HOME + AWAY;
    s.elder_wisdom = WISDOM;
    s.stirred_dreams = 0.0;

    assert_eq!(s.year(), YEAR, "fixture: the calendar year");
    assert_eq!(s.kittens.len(), HOME + AWAY, "fixture: the clowder, away kittens included");
    assert_eq!(s.basket_capacity(), 7, "fixture: the basket's room");
    assert!((s.memory_chest_ratio() - 0.06).abs() < 1e-9, "fixture: the chest's share");
    assert!(s.clone().apply(Command::GreatMeadowMove).is_ok(), "fixture: a Move is accepted");
    s
}

// ---------------------------------------------------------------------------
// Rows and arming
// ---------------------------------------------------------------------------

fn find(state: &GameState, section: Section, pred: impl Fn(&Command) -> bool) -> RowKey {
    rows(state, section)
        .into_iter()
        .find(|r| matches!(&r.kind, RowKind::Act { command, .. } if pred(command)))
        .map(|r| r.key)
        .unwrap_or_else(|| panic!("fixture: no row in {} offers the wanted command", section.title()))
}

fn move_key(state: &GameState) -> RowKey {
    find(state, Section::Legacy, |c| matches!(c, Command::GreatMeadowMove))
}

fn pass_key(state: &GameState) -> RowKey {
    find(state, Section::Legacy, |c| matches!(c, Command::PassItDown))
}

fn nap_key(state: &GameState) -> RowKey {
    find(state, Section::Dreams, |c| matches!(c, Command::DeepNap(_)))
}

fn rule_key(state: &GameState, rule: RuleId) -> RowKey {
    find(state, Section::Ideas, |c| *c == Command::AdoptRule(rule))
}

fn focused(section: Section, key: &RowKey) -> ViewState {
    let mut v = ViewState { section: Some(section), ..Default::default() };
    v.focus[section.index()] = Some(key.clone());
    v
}

/// The row focused and armed by a real first Enter.
fn armed_by_enter(state: &GameState, section: Section, key: &RowKey) -> ViewState {
    let mut v = focused(section, key);
    assert_eq!(update(Action::Activate, &mut v, state), Outcome::Handled);
    assert_eq!(v.armed.as_ref().map(|a| &a.key), Some(key), "fixture: Enter armed the row");
    v
}

/// The nap row armed through its count prompt with `years`.
fn nap_armed(state: &GameState, years: u32) -> ViewState {
    let key = nap_key(state);
    let mut v = focused(Section::Dreams, &key);
    update(Action::CountPrompt, &mut v, state);
    for c in years.to_string().chars() {
        update(Action::Type(c), &mut v, state);
    }
    update(Action::Activate, &mut v, state);
    assert_eq!(
        v.armed.as_ref().map(|a| a.command),
        Some(Command::DeepNap(years)),
        "fixture: the prompt armed the nap with the typed count"
    );
    v
}

/// The same view with nothing armed — the baseline a summary is read against.
fn unarmed(view: &ViewState) -> ViewState {
    ViewState { armed: None, ..view.clone() }
}

// ---------------------------------------------------------------------------
// Drawing and reading
// ---------------------------------------------------------------------------

struct Drawn {
    buffer: Buffer,
    layout: Layout,
}

fn draw(state: &GameState, view: &ViewState, (w, h): (u16, u16)) -> Drawn {
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("TestBackend is infallible");
    let mut layout = None;
    term.draw(|f| layout = Some(render::frame(f, state, view, None)))
        .expect("TestBackend is infallible");
    Drawn { buffer: term.backend().buffer().clone(), layout: layout.expect("a layout") }
}

fn region_lines(buf: &Buffer, rect: Rect) -> Vec<String> {
    (rect.y..rect.bottom())
        .map(|y| {
            (rect.x..rect.right())
                .map(|x| buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "))
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect()
}

/// The rectangle the list — or the swapped-in inspector — is drawn in.
fn list_rect(l: &Layout) -> Rect {
    match l.body {
        Body::Narrow { main, .. } => main,
        Body::Wide { list, .. } => list,
        Body::TooSmall(r) => panic!("fixture: {r:?} is below the floor"),
    }
}

/// The summary lines an armed row expands to: the lines directly beneath the
/// cursor row that the unarmed frame does not have there. Asserts the lines
/// above the row are untouched and the expansion starts on the very next line.
fn expansion(state: &GameState, view: &ViewState, size: (u16, u16)) -> Vec<String> {
    let armed = draw(state, view, size);
    let plain = draw(state, &unarmed(view), size);
    let a = region_lines(&armed.buffer, list_rect(&armed.layout));
    let u = region_lines(&plain.buffer, list_rect(&plain.layout));

    let r = u.iter().position(|l| l.contains('▸')).expect("fixture: the unarmed list shows the cursor");
    assert_eq!(
        a[..r],
        u[..r],
        "at {size:?}, arming the row changed the list above it; the row expands in place"
    );
    assert!(
        a[r].contains('▸'),
        "at {size:?}, the armed row is no longer where it was: {:?}",
        a[r]
    );
    let end = match u.get(r + 1).filter(|l| !l.trim().is_empty()) {
        Some(next) => a[r + 1..].iter().position(|l| l == next).map(|p| r + 1 + p).unwrap_or(a.len()),
        None => a[r + 1..].iter().position(|l| l.trim().is_empty()).map(|p| r + 1 + p).unwrap_or(a.len()),
    };
    a[r + 1..end].to_vec()
}

fn summary_text(state: &GameState, view: &ViewState, size: (u16, u16)) -> String {
    let lines = expansion(state, view, size);
    assert!(
        !lines.is_empty(),
        "at {size:?}, the armed {:?} row drew no summary beneath itself",
        view.armed.as_ref().map(|a| a.command)
    );
    lines.join("\n")
}

/// The textual forms a figure can take on screen.
fn forms(value: f64) -> Vec<String> {
    let mut out = vec![fmt::compact(value), fmt::exact(value)];
    if value.fract() == 0.0 && value.abs() < 1.0e15 {
        out.push(format!("{}", value as i64));
    }
    out
}

/// Whether `text` carries `form` as a number of its own — not a digit run
/// inside a longer number.
fn has_form(text: &str, form: &str) -> bool {
    let bytes = text.as_bytes();
    let mut from = 0;
    while let Some(at) = text[from..].find(form) {
        let start = from + at;
        let end = start + form.len();
        let before_ok = start == 0 || !(bytes[start - 1].is_ascii_digit() || bytes[start - 1] == b'.');
        let after_ok = end >= bytes.len()
            || !(bytes[end].is_ascii_digit()
                || (bytes[end] == b'.' && bytes.get(end + 1).is_some_and(|b| b.is_ascii_digit())));
        if before_ok && after_ok {
            return true;
        }
        from = start + 1;
    }
    false
}

fn states(text: &str, value: f64) -> bool {
    forms(value).iter().any(|f| has_form(text, f))
}

/// Words, lower-cased, one space apart and padded — for whole-word checks.
fn words(text: &str) -> String {
    let w: Vec<String> = text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| w.to_lowercase())
        .collect();
    format!(" {} ", w.join(" "))
}

fn humanize(tag: &str) -> String {
    let mut out = String::new();
    for (i, c) in tag.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

/// The summary with every figure blanked, and whitespace collapsed — what is
/// left is the summary's words alone.
fn shape(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_digit() {
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.' || chars[i] == ',') {
                i += 1;
            }
            // A single magnitude letter glued to the number is part of it.
            if i < chars.len()
                && chars[i].is_ascii_alphabetic()
                && !chars.get(i + 1).is_some_and(|c| c.is_alphanumeric())
            {
                i += 1;
            }
            out.push('#');
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ---------------------------------------------------------------------------
// TUI-NAV-015 / TUI-NAV-077 — the row expands in place, beneath itself
// ---------------------------------------------------------------------------

/// The first Enter on a heavy row expands it in place: the consequence
/// summary is drawn on the lines directly beneath the row, the rows above it
/// untouched — at every supported size, for every heavy row.
///
/// @spec TUI-NAV-015, TUI-NAV-077
#[test]
fn the_first_enter_expands_every_heavy_row_in_place_beneath_itself() {
    let state = meadow();

    let near_top = [
        ("GreatMeadowMove", Section::Legacy, move_key(&state)),
        ("PassItDown", Section::Legacy, pass_key(&state)),
        ("DeepNap", Section::Dreams, nap_key(&state)),
    ];
    for (name, section, key) in near_top {
        let view = armed_by_enter(&state, section, &key);
        for size in SIZES {
            let lines = expansion(&state, &view, size);
            assert!(
                !lines.is_empty(),
                "at {size:?}, the armed {name} row drew nothing beneath itself; the first Enter \
                 expands the row in place to its consequence summary"
            );
        }
    }

    let size = tall(&state);
    for rule in RuleId::ALL {
        let key = rule_key(&state, rule);
        let view = armed_by_enter(&state, Section::Ideas, &key);
        let lines = expansion(&state, &view, size);
        assert!(
            !lines.is_empty(),
            "the armed AdoptRule({rule:?}) row drew nothing beneath itself"
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-075 — what the summary states, row by row
//
// The tests are numbered by the spec's clauses in its own order: clause 1 the
// Move, clause 2 PassItDown, clause 3 the AdoptRule rows, clause 4 the nap.
// ---------------------------------------------------------------------------

/// The Move's summary states the clowder's kitten count with its away kittens
/// included, the basket's room, and the share of stores the chest carries —
/// each as the core reports it.
///
/// @spec TUI-NAV-075
#[test]
fn summary_clause_1_states_every_figure_it_lists() {
    let state = meadow();
    let view = armed_by_enter(&state, Section::Legacy, &move_key(&state));
    let text = summary_text(&state, &view, tall(&state));

    let all = state.kittens.len() as f64;
    assert!(
        states(&text, all),
        "the Move's summary does not state the clowder's {all} kittens (away ones included): \
         {text:?}"
    );
    assert!(
        !states(&text, HOME as f64),
        "the Move's summary counts only the kittens at home: {text:?}"
    );
    let basket = state.basket_capacity() as f64;
    assert!(states(&text, basket), "the Move's summary does not state the basket's room: {text:?}");
    let share = state.memory_chest_ratio();
    let pct = (share * 100.0 * 1e6).round() / 1e6;
    assert!(
        states(&text, pct) || states(&text, share),
        "the Move's summary does not state the chest's share of stores: {text:?}"
    );
}

/// The Move's summary states nothing of what the Move earns — the core's
/// legacy formulas decide that — nor which stocks it keeps whole: no figure
/// the Move would add to a legacy stock, no word naming that stock, and no
/// resource named at all.
///
/// @spec TUI-NAV-075
#[test]
fn summary_clause_1_states_nothing_it_leaves_unstated() {
    let state = meadow();
    let view = armed_by_enter(&state, Section::Legacy, &move_key(&state));
    let text = summary_text(&state, &view, tall(&state));

    let mut after = state.clone();
    after.apply(Command::GreatMeadowMove).expect("fixture: the Move is accepted");
    let stated = [state.kittens.len() as f64, state.basket_capacity() as f64, 6.0];
    for (what, gain) in [
        ("elder wisdom", after.elder_wisdom - state.elder_wisdom),
        ("warm memories", after.warm_memories - state.warm_memories),
    ] {
        assert!(gain > 0.0, "fixture: the Move earns some {what}");
        if stated.iter().any(|s| (s - gain).abs() < 1e-9) {
            continue;
        }
        assert!(
            !states(&text, gain),
            "the Move's summary states the {what} the Move would earn ({gain}): {text:?}"
        );
    }
    let w = words(&text);
    assert!(!w.contains(" wisdom "), "the Move's summary speaks of what it earns: {text:?}");
    for r in ResourceId::ALL {
        let debug = format!("{r:?}");
        let spaced = humanize(&debug).to_lowercase();
        assert!(
            !w.contains(&format!(" {spaced} ")) && !w.contains(&format!(" {} ", debug.to_lowercase())),
            "the Move's summary names a stock ({debug}); which stocks a Move keeps whole is the \
             core's rule and goes unstated: {text:?}"
        );
    }
}

/// PassItDown's summary states the elder wisdom that passes down: the whole
/// of the stock the core holds.
///
/// @spec TUI-NAV-075
#[test]
fn summary_clause_2_states_the_whole_stock() {
    let state = meadow();
    let view = armed_by_enter(&state, Section::Legacy, &pass_key(&state));
    let text = summary_text(&state, &view, tall(&state));
    assert!(
        states(&text, state.elder_wisdom),
        "PassItDown's summary does not state the elder wisdom stock ({}): {text:?}",
        state.elder_wisdom
    );
}

/// Every AdoptRule row's summary names the sibling rule its definition says
/// it closes for the meadow.
///
/// @spec TUI-NAV-075
#[test]
fn summary_clause_3_names_what_each_rows_definition_says_it_closes() {
    let state = meadow();
    let size = tall(&state);
    for rule in RuleId::ALL {
        let sibling = content::rule_def(content::rule_def(rule).blocks).name;
        let view = armed_by_enter(&state, Section::Ideas, &rule_key(&state, rule));
        let text = summary_text(&state, &view, size);
        assert!(
            text.contains(sibling),
            "the AdoptRule({rule:?}) summary does not name the sibling it closes ({sibling:?}): \
             {text:?}"
        );
    }
}

/// The nap's summary states the armed year count, the core's cost for a nap
/// of that length, and the wake-year — the core's current year plus the
/// count — for a plain Enter's one-year nap and for counts armed through the
/// prompt alike.
///
/// @spec TUI-NAV-075, TUI-NAV-036
#[test]
fn summary_clause_4_states_the_armed_count_its_cost_and_its_end_point() {
    let state = meadow();
    let size = tall(&state);

    let mut cases = vec![(1u32, armed_by_enter(&state, Section::Dreams, &nap_key(&state)))];
    assert_eq!(cases[0].1.armed.as_ref().map(|a| a.command), Some(Command::DeepNap(1)));
    for years in [4u32, 12] {
        cases.push((years, nap_armed(&state, years)));
    }

    for (years, view) in cases {
        let text = summary_text(&state, &view, size);
        let cost = state.starshard_cost(years);
        let wake = (state.year() + years as u64) as f64;
        assert!(states(&text, years as f64), "the {years}-year nap's summary lacks its count: {text:?}");
        assert!(
            states(&text, cost),
            "the {years}-year nap's summary lacks the core's cost ({cost}): {text:?}"
        );
        assert!(
            states(&text, wake),
            "the {years}-year nap's summary lacks the wake-year ({wake}): {text:?}"
        );
    }
}

/// The summary is re-derived from current state on every frame: the same
/// armed view over a state that has moved on states the new figures, not the
/// ones it was armed over.
///
/// @spec TUI-NAV-075
#[test]
fn the_summary_is_re_derived_from_current_state_on_every_frame() {
    let before = meadow();
    let size = tall(&before);

    // The Move.
    let view = armed_by_enter(&before, Section::Legacy, &move_key(&before));
    let mut after = before.clone();
    after.kittens.extend((0..9).map(|_| kitten(Location::Home)));
    after.traditions.remove(&TraditionId::TravelingBasketsIII);
    after.structures[StructureId::MemoryChest.ix()] = 3;
    let text = summary_text(&after, &view, size);
    assert!(states(&text, after.kittens.len() as f64), "stale kitten count: {text:?}");
    assert!(!states(&text, before.kittens.len() as f64), "stale kitten count: {text:?}");
    assert!(states(&text, after.basket_capacity() as f64), "stale basket room: {text:?}");
    let pct = (after.memory_chest_ratio() * 100.0 * 1e6).round() / 1e6;
    assert!(
        states(&text, pct) || states(&text, after.memory_chest_ratio()),
        "stale chest share: {text:?}"
    );

    // PassItDown.
    let view = armed_by_enter(&before, Section::Legacy, &pass_key(&before));
    let mut after = before.clone();
    after.elder_wisdom = 8765.0;
    let text = summary_text(&after, &view, size);
    assert!(states(&text, after.elder_wisdom), "stale elder wisdom: {text:?}");
    assert!(!states(&text, before.elder_wisdom), "stale elder wisdom: {text:?}");

    // The nap.
    let view = nap_armed(&before, 4);
    let mut after = before.clone();
    after.stirred_dreams = 300.0;
    after.ticks += 11 * content::DAYS_PER_YEAR * content::TICKS_PER_DAY;
    assert_ne!(after.starshard_cost(4), before.starshard_cost(4), "fixture: the cost moved");
    let text = summary_text(&after, &view, size);
    assert!(states(&text, after.starshard_cost(4)), "stale nap cost: {text:?}");
    assert!(states(&text, (after.year() + 4) as f64), "stale wake-year: {text:?}");
    assert!(!states(&text, (before.year() + 4) as f64), "stale wake-year: {text:?}");
}

/// The summary makes no judgement of whether the commit will be accepted: a
/// heavy action the core would refuse now is summarised in exactly the words
/// an accepted one is, only its figures differing.
///
/// @spec TUI-NAV-075
#[test]
fn the_summary_makes_no_judgement_of_whether_the_commit_will_be_accepted() {
    let ok = meadow();
    let size = tall(&ok);

    // The Move, with the clowder fallen below what a Move needs.
    let view = armed_by_enter(&ok, Section::Legacy, &move_key(&ok));
    let mut short = ok.clone();
    short.kittens.truncate(20);
    assert!(short.clone().apply(Command::GreatMeadowMove).is_err(), "fixture: refused");
    assert_eq!(
        shape(&summary_text(&short, &view, size)),
        shape(&summary_text(&ok, &view, size)),
        "the Move's summary reads differently when the core would refuse it"
    );

    // An AdoptRule row with nothing to pay. (Adopting its sibling takes the
    // row out of its list, which ends the arming rather than summarising it.)
    let rule = RuleId::ALL[0];
    let view = armed_by_enter(&ok, Section::Ideas, &rule_key(&ok, rule));
    let mut broke = ok.clone();
    broke.resources[ResourceId::Charm.ix()] = 0.0;
    {
        let (why, world) = ("nothing to pay", &broke);
        assert!(world.clone().apply(Command::AdoptRule(rule)).is_err(), "fixture: refused");
        assert!(
            rows(world, Section::Ideas).iter().any(|r| Some(&r.key) == view.armed.as_ref().map(|a| &a.key)),
            "fixture: the rule's row is still listed with {why}"
        );
        assert_eq!(
            shape(&summary_text(world, &view, size)),
            shape(&summary_text(&ok, &view, size)),
            "the AdoptRule summary reads differently with {why}"
        );
    }

    // The nap: unaffordable, and longer than the core allows.
    let four = nap_armed(&ok, 4);
    let mut poor = ok.clone();
    poor.resources[ResourceId::Starshards.ix()] = 0.0;
    assert!(poor.clone().apply(Command::DeepNap(4)).is_err(), "fixture: refused");
    assert_eq!(
        shape(&summary_text(&poor, &four, size)),
        shape(&summary_text(&ok, &four, size)),
        "the nap's summary reads differently when the nap is unaffordable"
    );
    let too_long_years = ok.max_nap_length() + 1;
    let too_long = nap_armed(&ok, too_long_years);
    assert!(ok.clone().apply(Command::DeepNap(too_long_years)).is_err(), "fixture: refused");
    assert!(ok.starshard_cost(too_long_years).is_finite(), "fixture: a finite cost");
    assert_eq!(
        shape(&summary_text(&ok, &too_long, size)),
        shape(&summary_text(&ok, &four, size)),
        "the nap's summary reads differently for a count longer than the core allows"
    );
}

/// A figure the core reports that is not finite is stated in plain words as
/// beyond reach and never printed — for every figure the core can report
/// that way: the elder wisdom stock and the nap's cost.
///
/// @spec TUI-NAV-075
#[test]
fn a_figure_that_is_not_finite_is_stated_as_beyond_reach_and_never_printed() {
    let base = meadow();
    let size = tall(&base);

    let printed = |text: &str| {
        let w = words(text);
        text.contains('∞') || w.contains(" inf ") || w.contains(" infinity ") || w.contains(" nan ")
    };

    for wisdom in [f64::INFINITY, f64::NAN] {
        let mut state = base.clone();
        state.elder_wisdom = wisdom;
        // Elder wisdom feeds production, so a non-finite stock makes every
        // held store's rate non-finite too; with nothing held, the sidebar
        // formats no rate, and this frame is about the summary alone.
        for r in ResourceId::ALL {
            state.resources[r.ix()] = 0.0;
        }
        let view = armed_by_enter(&state, Section::Legacy, &pass_key(&state));
        let text = summary_text(&state, &view, size);
        assert!(!printed(&text), "a {wisdom} elder wisdom stock was printed: {text:?}");
        assert!(
            text.to_lowercase().contains("beyond reach"),
            "a {wisdom} elder wisdom stock was not stated as beyond reach: {text:?}"
        );
    }

    // A count far past any real nap: its cost overflows.
    let huge = clowder_tui::nav::update::MAX_AFFORDABLE_CAP;
    assert!(!base.starshard_cost(huge).is_finite(), "fixture: the cost is not finite");
    let view = nap_armed(&base, huge);
    let text = summary_text(&base, &view, size);
    assert!(!printed(&text), "an unbounded nap cost was printed: {text:?}");
    assert!(text.to_lowercase().contains("beyond reach"), "not stated as beyond reach: {text:?}");
    assert!(states(&text, huge as f64), "the armed count went unstated: {text:?}");
    assert!(
        states(&text, (base.year() + huge as u64) as f64),
        "the wake-year went unstated beside an unbounded cost: {text:?}"
    );

    // A cost the core reports as not a number at all.
    let mut hazy = base.clone();
    hazy.stirred_dreams = f64::NAN;
    assert!(hazy.starshard_cost(4).is_nan(), "fixture: the cost is not a number");
    let view = nap_armed(&hazy, 4);
    let text = summary_text(&hazy, &view, size);
    assert!(!printed(&text), "a not-a-number nap cost was printed: {text:?}");
    assert!(text.to_lowercase().contains("beyond reach"), "not stated as beyond reach: {text:?}");
}

// ---------------------------------------------------------------------------
// TUI-NAV-077 — where the summary shows, and that it goes when the arming does
// ---------------------------------------------------------------------------

/// While the narrow-mode swap shows the inspector in the list's place, the
/// inspector carries the same summary the list would have shown.
///
/// @spec TUI-NAV-077
#[test]
fn the_swapped_inspector_carries_the_same_summary() {
    let state = meadow();
    let cases: Vec<(&str, ViewState)> = vec![
        ("GreatMeadowMove", armed_by_enter(&state, Section::Legacy, &move_key(&state))),
        ("PassItDown", armed_by_enter(&state, Section::Legacy, &pass_key(&state))),
        ("DeepNap", nap_armed(&state, 4)),
        (
            "AdoptRule",
            armed_by_enter(&state, Section::Ideas, &rule_key(&state, RuleId::ALL[0])),
        ),
    ];

    for (name, view) in cases {
        let listed = summary_text(&state, &view, tall(&state));
        let tokens: Vec<&str> = listed.split_whitespace().collect();

        for size in NARROW {
            let swapped = ViewState { inspector_wanted: true, ..view.clone() };
            let armed = draw(&state, &swapped, size);
            let plain = draw(&state, &unarmed(&swapped), size);
            assert!(
                matches!(armed.layout.body, Body::Narrow { .. }),
                "fixture: {size:?} is a swap size"
            );
            let shown = region_lines(&armed.buffer, list_rect(&armed.layout)).join(" ");
            let shown_tokens: BTreeSet<&str> = shown.split_whitespace().collect();
            for t in &tokens {
                assert!(
                    shown_tokens.contains(t),
                    "at {size:?}, the swapped inspector over the armed {name} row lacks {t:?} of \
                     the summary {listed:?}; it read {shown:?}"
                );
            }
            assert_ne!(
                region_lines(&armed.buffer, list_rect(&armed.layout)),
                region_lines(&plain.buffer, list_rect(&plain.layout)),
                "at {size:?}, the swapped inspector reads the same armed as unarmed"
            );
        }
    }
}

/// The moment an arming ends the row collapses and nothing on screen still
/// shows it armed: a frame over an arming past its expiry, or over one whose
/// row is no longer listed, is cell for cell the frame with nothing armed —
/// at every size, list or swapped inspector.
///
/// @spec TUI-NAV-077, TUI-NAV-076
#[test]
fn the_moment_an_arming_ends_nothing_on_screen_shows_it_armed() {
    let state = meadow();
    let past = Instant::now().checked_sub(Duration::from_secs(1)).expect("clock has a past");

    let cases: Vec<(&str, ViewState)> = vec![
        ("GreatMeadowMove", armed_by_enter(&state, Section::Legacy, &move_key(&state))),
        ("PassItDown", armed_by_enter(&state, Section::Legacy, &pass_key(&state))),
        ("DeepNap", nap_armed(&state, 4)),
        (
            "AdoptRule",
            armed_by_enter(&state, Section::Ideas, &rule_key(&state, RuleId::ALL[0])),
        ),
    ];

    for (name, view) in cases {
        let mut expired = view.clone();
        expired.armed.as_mut().expect("armed").expires_at = past;
        for swap in [false, true] {
            let expired = ViewState { inspector_wanted: swap, ..expired.clone() };
            for size in SIZES {
                assert!(
                    draw(&state, &expired, size).buffer == draw(&state, &unarmed(&expired), size).buffer,
                    "at {size:?} (inspector asked for: {swap}), an expired {name} arming still \
                     shows on screen"
                );
            }
        }
    }

    // The nap row, armed, then no longer listed anywhere.
    let armed = nap_armed(&state, 4);
    let mut gone = state.clone();
    gone.ideas.remove(&IdeaId::DeepNapTheory);
    let nap = nap_key(&state);
    assert!(
        Section::ALL.iter().all(|s| rows(&gone, *s).iter().all(|r| r.key != nap)),
        "fixture: the nap row is listed nowhere"
    );
    for (section, key) in [(Section::Dreams, nap.clone()), (Section::Legacy, move_key(&gone))] {
        let mut v = focused(section, &key);
        v.armed = armed.armed.clone();
        let _: &ArmedRow = v.armed.as_ref().expect("armed");
        for size in SIZES {
            assert!(
                draw(&gone, &v, size).buffer == draw(&gone, &unarmed(&v), size).buffer,
                "at {size:?}, an arming whose row is no longer listed still shows on screen \
                 (viewing {})",
                section.title()
            );
        }
    }
}
