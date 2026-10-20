//! The tinkerers' bench: the Crafts row that states the seats, who holds
//! them, and who waits — and the tinkerer count rows that say why a bind
//! would be refused.
//!
//! @spec TUI-NAV-156, TUI-NAV-157, TUI-NAV-158, TUI-NAV-159, TUI-NAV-160,
//! TUI-NAV-161, TUI-NAV-053, TUI-NAV-032, TUI-NAV-018, TUI-RENDER-028

use std::path::PathBuf;
use std::time::{Duration, Instant};

use clowder_core::content::{craft_def, structure_def};
use clowder_core::{
    ContentId, CraftId, GameState, IdeaId, JobId, Kitten, KittenTrait, Location, ResourceId,
    StructureId,
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::Terminal;

use clowder_tui::nav::rows::{rows, RowKind, BENCH_NAME};
use clowder_tui::nav::{update, Outcome, RowKey, Section, ViewState};
use clowder_tui::render::layout::{Body, Layout};
use clowder_tui::runtime::clock::Reading;
use clowder_tui::runtime::{App, TickAccumulator};

const WIDE: u16 = 130;
/// Wide enough that no inspector line the bench states wraps.
const WIDEST: u16 = 200;
const TALL: u16 = 60;
const FLOOR_W: u16 = 80;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn kitten(name: &str, job: Option<JobId>, bound: Option<CraftId>, away: bool) -> Kitten {
    Kitten {
        name: name.to_string(),
        job,
        location: if away {
            Location::NeighborsYard
        } else {
            Location::Home
        },
        arrived_day: 0,
        job_seq: None,
        away_seq: None,
        skills: [0.0; 8],
        exp: 0.0,
        rank: 0,
        trait_id: KittenTrait::None,
        tinkering: bound,
    }
}

/// A Tinkerer, bound to `bound` or waiting, at home or away.
fn tinkerer(name: &str, bound: Option<CraftId>, away: bool) -> Kitten {
    kitten(name, Some(JobId::Tinkerer), bound, away)
}

/// A meadow with the first recipes open, the Tinkerer job unlocked, the
/// seat-giving structure revealed and `workshops` of it built, and `roster`.
fn bench_world(workshops: u32, roster: Vec<Kitten>) -> GameState {
    let mut s = GameState::new(11);
    s.ideas.insert(IdeaId::Tinkering);
    s.ideas.insert(IdeaId::Toymaking);
    s.revealed
        .insert(ContentId::Structure(StructureId::ToyWorkshop));
    s.structures[StructureId::ToyWorkshop.ix()] = workshops;
    s.kittens = roster;
    s
}

fn tinker_row(c: CraftId) -> RowKey {
    RowKey::Content(ContentId::Craft(c), "tinker")
}

fn focused(section: Section, key: RowKey) -> ViewState {
    let mut v = ViewState {
        section: Some(section),
        ..Default::default()
    };
    v.focus[section.index()] = Some(key);
    v
}

fn workshop_name() -> &'static str {
    structure_def(StructureId::ToyWorkshop).name
}

// ---------------------------------------------------------------------------
// Drawing and reading
// ---------------------------------------------------------------------------

struct Drawn {
    buffer: Buffer,
    layout: Layout,
}

fn draw(state: &GameState, view: &ViewState, w: u16, h: u16) -> Drawn {
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("TestBackend is infallible");
    let mut layout = None;
    term.draw(|f| layout = Some(clowder_tui::render::frame(f, state, view, None)))
        .expect("TestBackend is infallible");
    Drawn {
        buffer: term.backend().buffer().clone(),
        layout: layout.expect("a frame was drawn"),
    }
}

fn region(buf: &Buffer, rect: Rect) -> Vec<String> {
    (rect.y..rect.bottom())
        .map(|y| {
            (rect.x..rect.right())
                .map(|x| buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "))
                .collect::<String>()
        })
        .collect()
}

/// The list's lines, inside its frame.
fn list_lines(d: &Drawn) -> Vec<String> {
    let rect = match d.layout.body {
        Body::Narrow { main, .. } => Rect::new(main.x + 1, main.y, main.width - 1, main.height),
        Body::Wide { list, .. } => Rect::new(list.x + 1, list.y, list.width - 2, list.height),
        Body::TooSmall(r) => panic!("fixture: {r:?} is below the floor"),
    };
    region(&d.buffer, rect)
}

/// The inspector pane's lines, at a width that lays out three columns: its
/// frame's edges taken off, each line's leading blanks kept so indentation
/// can be read.
fn inspector_lines(state: &GameState, view: &ViewState) -> Vec<String> {
    let d = draw(state, view, WIDEST, TALL);
    let Body::Wide { inspector, .. } = d.layout.body else {
        panic!("fixture: {WIDEST} columns lays out the inspector pane");
    };
    region(&d.buffer, inspector)
        .into_iter()
        .map(|l| {
            l.trim_start_matches('│')
                .trim_end_matches([' ', '│'])
                .to_string()
        })
        .collect()
}

fn inspector_text(state: &GameState, key: RowKey) -> String {
    inspector_lines(state, &focused(Section::Crafts, key)).join("\n")
}

/// The index of the first line holding `needle`.
fn line_of(lines: &[String], needle: &str) -> Option<usize> {
    lines.iter().position(|l| l.contains(needle))
}

fn char_col(line: &str, needle: &str) -> usize {
    let byte = line.find(needle).expect("needle on the line");
    line[..byte].chars().count()
}

// ---------------------------------------------------------------------------
// TUI-NAV-156 — when the bench is listed, and where
// ---------------------------------------------------------------------------

/// Before the Tinkerer job is unlocked the Crafts list holds its recipes'
/// make rows and nothing about tinkerers — no bench and no tinkerer count
/// rows. Once it is, the list opens with the `tinkerers` group holding the
/// bench alone, then the `crafts` group, each recipe's tinkerer row beside
/// its make row there.
/// @spec TUI-NAV-156, TUI-NAV-098, TUI-NAV-099, TUI-NAV-100
#[test]
fn the_bench_is_listed_with_the_tinkerer_job_and_never_before() {
    let mut before = bench_world(2, vec![]);
    before.ideas.remove(&IdeaId::Toymaking);
    let list = rows(&before, Section::Crafts);
    assert!(
        list.iter()
            .any(|r| matches!(r.key, RowKey::Content(_, "craft"))),
        "fixture: the recipes are open"
    );
    assert!(
        !list.iter().any(|r| r.key == RowKey::Bench),
        "no bench before the Tinkerer job"
    );
    assert!(
        !list
            .iter()
            .any(|r| matches!(r.key, RowKey::Content(_, "tinker"))),
        "no tinkerer count row before the Tinkerer job (SIM-ECO-046)"
    );

    let after = bench_world(2, vec![]);
    let list = rows(&after, Section::Crafts);
    let keys: Vec<&RowKey> = list.iter().map(|r| &r.key).collect();
    assert_eq!(
        keys[0],
        &RowKey::Fixed("tinkerers"),
        "the tinkerers header first"
    );
    assert!(matches!(list[0].kind, RowKind::Group));
    assert_eq!(keys[1], &RowKey::Bench, "the bench beneath it, alone");
    assert!(matches!(list[1].kind, RowKind::Display), "display-only");
    assert_eq!(keys[2], &RowKey::Fixed("crafts"), "then the crafts header");
    assert!(matches!(list[2].kind, RowKind::Group));
    assert_eq!(
        list.iter().filter(|r| r.key == RowKey::Bench).count(),
        1,
        "one bench"
    );
    for (i, r) in list.iter().enumerate().skip(3) {
        if let RowKey::Content(ContentId::Craft(c), "tinker") = r.key {
            assert_eq!(
                list[i - 1].key,
                RowKey::Content(ContentId::Craft(c), "craft"),
                "{c:?}'s tinkerer row stands beside its make row"
            );
        }
    }
}

/// Nothing a key does on the bench issues a command: each key answers as it
/// does on a stores row.
/// @spec TUI-NAV-156
#[test]
fn the_bench_row_takes_no_command_and_answers_as_a_stores_row() {
    let mut state = bench_world(2, vec![tinkerer("Ash", None, false)]);
    state.resources[ResourceId::Catnip.ix()] = 1.0;
    let store = RowKey::Resource(ResourceId::Catnip);
    assert!(rows(&state, Section::Meadow).iter().any(|r| r.key == store));
    for code in [
        KeyCode::Enter,
        KeyCode::Char('+'),
        KeyCode::Char('-'),
        KeyCode::Char('x'),
        KeyCode::Char('X'),
    ] {
        let press = KeyEvent::new(code, KeyModifiers::NONE);
        let answer = |section: Section, key: RowKey| {
            let mut v = focused(section, key);
            let action = clowder_tui::nav::keys::resolve(press, &v);
            let outcome = update(action, &mut v, &state);
            (outcome, v.footer.map(|(m, _)| m), v.prompt.is_some())
        };
        let on_bench = answer(Section::Crafts, RowKey::Bench);
        assert_eq!(
            on_bench.0,
            Outcome::Handled,
            "{code:?} issues no command on the bench"
        );
        let on_store = answer(Section::Meadow, store.clone());
        assert_eq!(
            on_bench, on_store,
            "{code:?} answers on the bench as on a stores row"
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-157 / TUI-RENDER-028 — what the bench row says, and where
// ---------------------------------------------------------------------------

/// The bench row says the bindings, the seats, and — only where any wait —
/// how many Tinkerers wait, beginning at the count field's left edge, with
/// no marker. An away kitten's binding counts as taken.
/// @spec TUI-NAV-157, TUI-RENDER-028, TUI-NAV-156
#[test]
fn the_bench_row_draws_seats_taken_then_waiting_in_its_fields_place() {
    let anchor = |n: usize| -> Vec<Kitten> {
        (0..n)
            .map(|i| tinkerer(&format!("Pin{i}"), Some(CraftId::Wands), false))
            .collect()
    };
    let cases: Vec<(GameState, &str, bool)> = vec![
        (
            {
                let mut roster = anchor(1);
                roster.push(tinkerer("Bea", Some(CraftId::Tiles), true));
                roster.push(tinkerer("Cy", None, false));
                roster.push(kitten("Eve", Some(JobId::Gardener), None, false));
                bench_world(3, roster)
            },
            "2/3 seats taken · 1 waiting",
            true,
        ),
        (bench_world(3, anchor(3)), "3/3 seats taken", false),
        (
            {
                let mut roster = anchor(2);
                for i in 0..12 {
                    roster.push(tinkerer(&format!("W{i}"), None, false));
                }
                bench_world(10, roster)
            },
            "2/10 seats taken · 12 waiting",
            true,
        ),
        (bench_world(0, vec![]), "0/0 seats taken", false),
    ];
    for (state, expected, waits) in cases {
        for w in [FLOOR_W, WIDE] {
            let d = draw(
                &state,
                &focused(
                    Section::Crafts,
                    RowKey::Content(ContentId::Craft(CraftId::Tiles), "craft"),
                ),
                w,
                TALL,
            );
            let lines = list_lines(&d);
            let bench = &lines[line_of(&lines, BENCH_NAME).expect("the bench row is drawn")];
            assert!(
                bench.contains(expected),
                "at {w} columns the bench row reads {bench:?}, wanting {expected:?}"
            );
            if !waits {
                assert!(!bench.contains("waiting"), "no one waits: {bench:?}");
            }
            assert!(
                !bench.contains('●') && !bench.contains('○') && !bench.contains('◐'),
                "the bench wears no marker: {bench:?}"
            );
            assert_eq!(
                char_col(bench, BENCH_NAME),
                4,
                "the name stands where every name does, the marker's slot blank: {bench:?}"
            );

            // Where there are bindings at home on Wands, its tinkerer row
            // draws a count: the bench's text starts at that field's left edge.
            if let Some(t) = lines
                .iter()
                .find(|l| l.contains("tinkerers") && l.contains('×'))
            {
                let x = char_col(t, "×");
                let count_len = t[t.find('×').unwrap()..]
                    .chars()
                    .take_while(|c| !c.is_whitespace())
                    .count();
                let field_left = x + count_len - 7;
                let first = expected.split('/').next().unwrap();
                assert_eq!(
                    char_col(bench, &format!(" {first}/")) + 1,
                    field_left,
                    "at {w} columns the bench's text begins at the count field's left edge:\n{t}\n{bench}"
                );
            }
        }
    }
}

/// Where the fields' place cannot hold the whole text, the waiting clause
/// gives way whole, and the seats stand.
/// @spec TUI-RENDER-028
#[test]
fn where_the_place_cannot_hold_it_the_waiting_clause_gives_way_whole() {
    let roster: Vec<Kitten> = (0..1000)
        .map(|i| tinkerer(&format!("K{i}"), None, false))
        .collect();
    let state = bench_world(150, roster);
    let full = "0/150 seats taken · 1000 waiting";
    for w in [FLOOR_W, WIDE] {
        let d = draw(&state, &focused(Section::Crafts, RowKey::Bench), w, TALL);
        let lines = list_lines(&d);
        let bench = &lines[line_of(&lines, BENCH_NAME).expect("the bench row is drawn")];
        let start = char_col(bench, "0/150");
        let room = bench.chars().count() - start;
        assert!(
            full.chars().count() > room,
            "fixture: {full:?} must overflow the {room} columns of the place"
        );
        assert!(
            bench.contains("0/150 seats taken"),
            "the seats stand: {bench:?}"
        );
        assert!(
            !bench.contains("waiting") && !bench.contains('·') && !bench.contains("1000"),
            "the waiting clause gives way whole, not in part: {bench:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-158 / TUI-NAV-159 — what the inspector states on the bench
// ---------------------------------------------------------------------------

/// The seats line, then a line for the structure that gives seats — named
/// only once the core has revealed it — with the seats per copy and the
/// copies held. Free seats never go below zero.
/// @spec TUI-NAV-158
#[test]
fn the_bench_inspector_states_the_seats_and_where_they_come_from() {
    let state = bench_world(
        3,
        vec![
            tinkerer("Ash", Some(CraftId::Wands), false),
            tinkerer("Bea", Some(CraftId::Tiles), true),
            tinkerer("Cy", None, false),
        ],
    );
    let lines = inspector_lines(&state, &focused(Section::Crafts, RowKey::Bench));
    let text = lines.join("\n");
    assert!(
        text.contains(BENCH_NAME),
        "the inspector names the bench:\n{text}"
    );
    let seats = line_of(&lines, "Seats: 3 — 2 taken, 1 free.")
        .unwrap_or_else(|| panic!("the seats line:\n{text}"));
    let source = format!("Each {} seats 1 tinkerer. Owned: 3.", workshop_name());
    let from = line_of(&lines, &source).unwrap_or_else(|| panic!("{source:?}:\n{text}"));
    assert!(seats < from, "the seats line first:\n{text}");
    assert_eq!(
        text.matches("Each ").count(),
        1,
        "one structure gives seats:\n{text}"
    );

    let mut hidden = state.clone();
    hidden
        .revealed
        .remove(&ContentId::Structure(StructureId::ToyWorkshop));
    let text = inspector_lines(&hidden, &focused(Section::Crafts, RowKey::Bench)).join("\n");
    assert!(
        !text.contains(workshop_name()) && !text.contains("Each "),
        "a structure the core has not revealed is not named:\n{text}"
    );

    let over = bench_world(
        1,
        vec![
            tinkerer("Ash", Some(CraftId::Wands), false),
            tinkerer("Bea", Some(CraftId::Tiles), false),
            tinkerer("Cy", Some(CraftId::Tiles), false),
        ],
    );
    let text = inspector_lines(&over, &focused(Section::Crafts, RowKey::Bench)).join("\n");
    assert!(
        text.contains("Seats: 1 — 3 taken, 0 free."),
        "free seats never go below zero:\n{text}"
    );
}

/// After the seats lines: who sits where, in roster order, set in by two
/// columns, an away kitten marked visiting; then who waits. A Tinkerer away
/// without a binding, and a kitten at another job, are on neither list; with
/// nobody of a kind, its line is not stated.
/// @spec TUI-NAV-159
#[test]
fn the_bench_inspector_states_who_sits_where_and_who_waits() {
    let state = bench_world(
        4,
        vec![
            tinkerer("Ash", Some(CraftId::Wands), false),
            kitten("Eve", Some(JobId::Gardener), None, false),
            tinkerer("Bea", Some(CraftId::Tiles), true),
            tinkerer("Cy", None, false),
            tinkerer("Fen", None, true),
            tinkerer("Dot", None, false),
        ],
    );
    let lines = inspector_lines(&state, &focused(Section::Crafts, RowKey::Bench));
    let text = lines.join("\n");
    let wands = craft_def(CraftId::Wands).name;
    let tiles = craft_def(CraftId::Tiles).name;
    let at =
        |needle: &str| line_of(&lines, needle).unwrap_or_else(|| panic!("{needle:?}:\n{text}"));

    let seats = at("Seats: 4 — 2 taken, 2 free.");
    let source = at("Each ");
    let header = at("At the bench:");
    let ash = at(&format!("Ash — {wands}"));
    let bea = at(&format!("Bea — {tiles} (visiting)"));
    let head_col = char_col(&lines[header], "At the bench:");
    for (line, who) in [(ash, "Ash"), (bea, "Bea")] {
        assert_eq!(
            char_col(&lines[line], who),
            head_col + 2,
            "{who}'s line is set in by two blank columns beneath the heading:\n{text}"
        );
    }
    let waiting = at("Waiting for a seat: Cy, Dot.");
    assert!(
        seats < source && source < header && header < ash && ash < bea && bea < waiting,
        "seats, where they come from, who sits where in roster order, who waits:\n{text}"
    );
    assert!(
        !lines[ash].contains("visiting"),
        "a kitten at home is not visiting:\n{text}"
    );
    for absent in ["Eve", "Fen"] {
        assert!(
            !text.contains(absent),
            "{absent} neither sits nor waits:\n{text}"
        );
    }

    let empty = bench_world(2, vec![kitten("Eve", Some(JobId::Gardener), None, false)]);
    let text = inspector_lines(&empty, &focused(Section::Crafts, RowKey::Bench)).join("\n");
    assert!(
        !text.contains("At the bench:") && !text.contains("Waiting for a seat"),
        "nobody sits and nobody waits, so neither line is stated:\n{text}"
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-160 / TUI-NAV-053 — a tinkerer row's bind
// ---------------------------------------------------------------------------

/// On a tinkerer count row the inspector states why the bind would be
/// refused, in the bind's own words, and nothing where it would bind.
/// @spec TUI-NAV-160
#[test]
fn a_tinkerer_rows_inspector_states_why_its_bind_is_refused() {
    const NO_ROOM: &str = "Every seat at the bench is taken.";
    const NO_ONE: &str = "No tinkerer is waiting for a seat.";
    let cases = [
        (
            bench_world(0, vec![tinkerer("Cy", None, false)]),
            Some(NO_ROOM),
        ),
        (
            bench_world(2, vec![tinkerer("Ash", Some(CraftId::Tiles), false)]),
            Some(NO_ONE),
        ),
        (bench_world(1, vec![tinkerer("Cy", None, false)]), None),
    ];
    for (state, want) in cases {
        let text = inspector_text(&state, tinker_row(CraftId::Wands));
        for words in [NO_ROOM, NO_ONE] {
            assert_eq!(
                text.contains(words),
                want == Some(words),
                "the bind reason is {want:?}; the inspector read:\n{text}"
            );
        }
    }
}

/// A tinkerer row wears ● where its bind would be made and ○ where the core
/// reports a reason it would be refused.
/// @spec TUI-NAV-053
#[test]
fn a_tinkerer_rows_marker_follows_its_bind() {
    for (state, want) in [
        (bench_world(1, vec![tinkerer("Cy", None, false)]), '●'),
        (bench_world(0, vec![tinkerer("Cy", None, false)]), '○'),
        (bench_world(3, vec![]), '○'),
    ] {
        let d = draw(&state, &focused(Section::Crafts, RowKey::Bench), WIDE, TALL);
        let tinker_lines: Vec<String> = list_lines(&d)
            .into_iter()
            .filter(|l| l.contains("tinkerers") && !l.contains("──"))
            .collect();
        assert!(!tinker_lines.is_empty(), "fixture: tinkerer rows are drawn");
        for l in tinker_lines {
            assert!(
                l.chars().nth(2) == Some(want),
                "a tinkerer row wears {want}: {l:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-161 — the footer of a refused bind
// ---------------------------------------------------------------------------

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("clowder-bench-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("save.json")
}

fn press_enter_on(state: GameState, section: Section, key: RowKey, name: &str) -> Option<String> {
    let mut app = App {
        state,
        view: focused(section, key),
        clock: TickAccumulator::default(),
        save_path: scratch(name),
        last_save: Instant::now(),
        prev_layout: None,
        pointer_record: None,
        quitting: false,
        save_failed: false,
    };
    let mut term = Terminal::new(TestBackend::new(120, 40)).expect("TestBackend is infallible");
    let still = Reading {
        monotonic: Duration::ZERO,
        wall: Duration::ZERO,
    };
    let enter = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.step(&mut term, still, &[enter])
        .expect("an in-memory step does not fail");
    app.view
        .footer
        .as_ref()
        .filter(|(_, until)| *until > Instant::now())
        .map(|(m, _)| m.clone())
}

/// A refused bind says what it wants in the bind's own words; a job's
/// refusal keeps its own.
/// @spec TUI-NAV-161, TUI-NAV-014
#[test]
fn a_refused_binds_footer_says_what_the_bench_wants() {
    let no_one = bench_world(2, vec![kitten("Ivy", None, None, false)]);
    assert_eq!(
        press_enter_on(
            no_one,
            Section::Crafts,
            tinker_row(CraftId::Wands),
            "no-one"
        )
        .as_deref(),
        Some("no tinkerer is waiting for a seat"),
        "an idle kitten is no tinkerer: the bind wants one holding the job"
    );
    let no_room = bench_world(0, vec![tinkerer("Cy", None, false)]);
    assert_eq!(
        press_enter_on(
            no_room,
            Section::Crafts,
            tinker_row(CraftId::Wands),
            "no-room"
        )
        .as_deref(),
        Some("no room at the bench")
    );
    let busy = bench_world(1, vec![kitten("Eve", Some(JobId::Gardener), None, false)]);
    let job = RowKey::Content(ContentId::Job(JobId::Gardener), "");
    assert_eq!(
        press_enter_on(busy, Section::Clowder, job, "job").as_deref(),
        Some(clowder_tui::nav::plain_error(&clowder_core::CommandError::NoIdleKittens).as_str()),
        "a job's refusal keeps the wording of its error"
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-032 — where the cursor lands
// ---------------------------------------------------------------------------

/// Entering Crafts with no row remembered lands on the bench, its first row.
/// @spec TUI-NAV-032, TUI-NAV-156
#[test]
fn entering_crafts_lands_on_the_bench() {
    let state = bench_world(2, vec![tinkerer("Cy", None, false)]);
    let mut v = ViewState {
        section: Some(Section::Meadow),
        ..Default::default()
    };
    let four =
        clowder_tui::nav::keys::resolve(KeyEvent::new(KeyCode::Char('4'), KeyModifiers::NONE), &v);
    update(four, &mut v, &state);
    assert_eq!(v.section, Some(Section::Crafts));
    assert_eq!(v.focus[Section::Crafts.index()], Some(RowKey::Bench));
}

// ---------------------------------------------------------------------------
// TUI-NAV-018 — the seats on the structure's own row
// ---------------------------------------------------------------------------

/// The structure that gives seats says so on its own row, from its
/// definition; no other structure speaks of seats.
/// @spec TUI-NAV-018, SIM-ECO-045
#[test]
fn the_seat_giving_structure_states_its_seats() {
    let mut state = bench_world(1, vec![]);
    for b in StructureId::ALL {
        state.revealed.insert(ContentId::Structure(b));
    }
    for b in StructureId::ALL {
        let key = RowKey::Content(ContentId::Structure(b), "");
        if !rows(&state, Section::Meadow).iter().any(|r| r.key == key) {
            continue;
        }
        let text = inspector_lines(&state, &focused(Section::Meadow, key)).join(" ");
        let says = text.contains("Seats 1 tinkerer.");
        assert_eq!(
            says,
            b == StructureId::ToyWorkshop,
            "{b:?}: only the seat-giving structure states seats:\n{text}"
        );
    }
}
