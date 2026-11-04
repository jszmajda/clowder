//! Taking a copy of a structure down, from the keyboard: the keys that arm it,
//! what the armed row says, and what commits it.
//!
//! @spec TUI-NAV-204, TUI-NAV-205, TUI-NAV-206

use std::path::PathBuf;
use std::time::{Duration, Instant};

use clowder_core::{Command, ContentId, GameState, IdeaId, ResourceId, StructureId};
use crossterm::event::{Event as TermEvent, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

use clowder_tui::nav::rows::{rows, RowKind};
use clowder_tui::nav::{update, Action, Outcome, RowKey, Section, ViewState};
use clowder_tui::render;
use clowder_tui::render::widgets::chrome;
use clowder_tui::runtime::clock::Reading;
use clowder_tui::runtime::{now, App, TickAccumulator};

/// Past the gate that tells a distinct press from a held one.
const DISTINCT: Duration = Duration::from_millis(500);
/// Past how long an arming lasts.
const EXPIRED: Duration = Duration::from_secs(9);

/// The first structure's build row, with `copies` standing, in a meadow that has
/// plenty of everything.
fn meadow(copies: u32) -> (GameState, RowKey, StructureId) {
    let mut s = GameState::new(7);
    s.ideas = IdeaId::ALL.into_iter().collect();
    s.revealed = StructureId::ALL
        .map(ContentId::Structure)
        .into_iter()
        .collect();
    for r in ResourceId::ALL {
        s.resources[r.ix()] = 1.0e6;
    }
    let (key, st) = rows(&s, Section::Meadow)
        .into_iter()
        .find_map(|r| match r.kind {
            RowKind::Act {
                command: Command::Build(st),
                ..
            } => Some((r.key.clone(), st)),
            _ => None,
        })
        .expect("fixture: a build row is listed");
    s.structures[st.ix()] = copies;
    (s, key, st)
}

fn on(key: &RowKey) -> ViewState {
    let mut v = ViewState {
        section: Some(Section::Meadow),
        ..Default::default()
    };
    v.focus[Section::Meadow.index()] = Some(key.clone());
    v
}

fn footer(v: &ViewState) -> Option<&str> {
    v.footer.as_ref().map(|(text, _)| text.as_str())
}

/// Both keys arm the row with taking one down, and neither acts.
///
/// @spec TUI-NAV-204
#[test]
fn both_keys_arm_taking_one_down_and_neither_acts() {
    let _clock = now::held();
    let (s, key, st) = meadow(2);
    for action in [Action::Decrement] {
        let mut v = on(&key);
        assert_eq!(update(action, &mut v, &s), Outcome::Handled, "it only arms");
        let armed = v.armed.as_ref().expect("the row is armed");
        assert_eq!(armed.key, key);
        assert_eq!(armed.command, Command::Raze(st));
        assert_eq!(footer(&v), None, "arming needs no note");
    }
    for code in [KeyCode::Char('-'), KeyCode::Backspace] {
        assert_eq!(
            clowder_tui::nav::keys::resolve(KeyEvent::new(code, KeyModifiers::NONE), &on(&key)),
            Action::Decrement,
            "{code:?} is the key"
        );
    }
}

/// A distinct Enter commits; one still held from the arming does not.
///
/// @spec TUI-NAV-204
#[test]
fn only_a_distinct_enter_commits() {
    let _clock = now::held();
    let (s, key, st) = meadow(2);
    let mut v = on(&key);
    update(Action::Decrement, &mut v, &s);

    assert_eq!(
        update(Action::Activate, &mut v, &s),
        Outcome::Handled,
        "held: no commit"
    );
    assert!(v.armed.is_some(), "still armed");

    now::advance(DISTINCT);
    assert_eq!(
        update(Action::Activate, &mut v, &s),
        Outcome::Apply(Command::Raze(st)),
        "a distinct Enter commits what was armed, not a build"
    );
    assert!(v.armed.is_none(), "the commit ends the arming");
}

/// The Enter that committed, held on, does not build a copy back; a distinct one
/// builds as it always does.
///
/// @spec TUI-NAV-204
#[test]
fn the_enter_that_commits_held_on_does_not_build_one_back() {
    let _clock = now::held();
    let (s, key, st) = meadow(2);
    let mut v = on(&key);
    update(Action::Decrement, &mut v, &s);
    now::advance(DISTINCT);
    update(Action::Activate, &mut v, &s);

    now::advance(Duration::from_millis(30));
    assert_eq!(
        update(Action::Activate, &mut v, &s),
        Outcome::Handled,
        "held on: nothing"
    );
    now::advance(DISTINCT);
    let mut later = 0;
    for _ in 0..3 {
        if update(Action::Activate, &mut v, &s) == Outcome::Apply(Command::Build(st)) {
            later += 1;
        }
        now::advance(DISTINCT);
    }
    assert!(later > 0, "a distinct press builds as it always does");
}

/// A further press of either key leaves the arming as it stands: it neither
/// commits nor buys the arming more time.
///
/// @spec TUI-NAV-204
#[test]
fn a_further_press_leaves_the_arming_as_it_stands() {
    let _clock = now::held();
    let (s, key, _) = meadow(2);
    let mut v = on(&key);
    update(Action::Decrement, &mut v, &s);
    let expires = v.armed.as_ref().unwrap().expires_at;

    now::advance(DISTINCT);
    assert_eq!(update(Action::Decrement, &mut v, &s), Outcome::Handled);
    assert_eq!(
        v.armed.as_ref().expect("still armed").expires_at,
        expires,
        "no more time for pressing again"
    );
}

/// With none standing, the key says so and arms nothing.
///
/// @spec TUI-NAV-204
#[test]
fn with_none_standing_the_key_says_so() {
    let _clock = now::held();
    let (s, key, _) = meadow(0);
    let mut v = on(&key);
    assert_eq!(update(Action::Decrement, &mut v, &s), Outcome::Handled);
    assert!(v.armed.is_none(), "nothing to arm");
    assert!(
        footer(&v).is_some_and(|t| t.contains("none")),
        "the key says so: {:?}",
        footer(&v)
    );
}

/// An arming ends as any does; once it has, Enter builds as it always does.
///
/// @spec TUI-NAV-204
#[test]
fn an_arming_ends_as_any_does_and_enter_builds_again() {
    let _clock = now::held();
    let (s, key, st) = meadow(2);

    let mut v = on(&key);
    update(Action::Decrement, &mut v, &s);
    now::advance(EXPIRED);
    assert_eq!(
        update(Action::Activate, &mut v, &s),
        Outcome::Apply(Command::Build(st)),
        "expired: Enter builds"
    );

    let mut v = on(&key);
    update(Action::Decrement, &mut v, &s);
    update(Action::CursorDown, &mut v, &s);
    assert!(v.armed.is_none(), "navigation ends it");

    let mut v = on(&key);
    update(Action::Decrement, &mut v, &s);
    update(Action::Dismiss, &mut v, &s);
    assert!(v.armed.is_none(), "Esc ends it");
}

/// The keys do what they did on the rows that have no copies to count: a count
/// row still unassigns, and a row that makes nothing arms nothing.
///
/// @spec TUI-NAV-013, TUI-NAV-204
#[test]
fn the_keys_arm_nothing_on_any_other_row() {
    let _clock = now::held();
    let (s, _, _) = meadow(2);
    for section in Section::ALL {
        for row in rows(&s, section) {
            if matches!(
                row.kind,
                RowKind::Act {
                    command: Command::Build(_),
                    ..
                }
            ) {
                continue;
            }
            let mut v = ViewState {
                section: Some(section),
                ..Default::default()
            };
            v.focus[section.index()] = Some(row.key.clone());
            let out = update(Action::Decrement, &mut v, &s);
            assert!(v.armed.is_none(), "{:?} armed", row.key);
            if let RowKind::Count { decrement, .. } = row.kind {
                assert_eq!(out, Outcome::Apply(decrement), "{:?}", row.key);
            }
        }
    }
}

fn frame_text(s: &GameState, v: &ViewState) -> String {
    let mut term = Terminal::new(TestBackend::new(200, 60)).unwrap();
    term.draw(|f| {
        render::frame(f, s, v, None);
    })
    .unwrap();
    let buf = term.backend().buffer().clone();
    (0..buf.area.height)
        .map(|y| {
            (0..buf.area.width)
                .map(|x| buf[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The armed row states what the core reports: the structure's name, the copies
/// left, each amount returned, and each store trimmed, afresh on every frame.
///
/// @spec TUI-NAV-205
#[test]
fn the_armed_row_says_what_the_core_reports() {
    let _clock = now::held();
    let (mut s, key, st) = meadow(3);
    // Room in every store, so what comes home is all credited.
    for r in ResourceId::ALL {
        s.resources[r.ix()] = 0.0;
    }

    let mut v = on(&key);
    update(Action::Decrement, &mut v, &s);
    let report = s.raze_report(st).expect("a copy stands");
    let name = clowder_core::content::structure_def(st).name;
    let text = frame_text(&s, &v);
    assert!(text.contains(name), "the structure: {text}");
    assert!(text.contains("2 left"), "the copies left: {text}");
    assert!(!report.returned.is_empty(), "fixture: something comes home");
    for (r, amount) in &report.returned {
        assert!(
            text.contains(&render::fmt::exact(*amount)),
            "the amount of {r:?} returned, {amount}: {text}"
        );
    }

    // A store standing over the cap it would have: said, with the amount.
    let (trimming, store) = StructureId::ALL
        .into_iter()
        .find_map(|c| {
            let mut less = s.clone();
            less.structures[c.ix()] = 1;
            let mut more = s.clone();
            more.structures[c.ix()] = 2;
            ResourceId::ALL
                .into_iter()
                .find(|r| more.cap(*r).is_finite() && less.cap(*r) < more.cap(*r))
                .map(|r| (c, r))
        })
        .expect("fixture: a structure gives a cap");
    s.structures[trimming.ix()] = 2;
    s.resources[store.ix()] = s.cap(store);
    s.revealed.insert(ContentId::Structure(trimming));
    let row = rows(&s, Section::Meadow)
        .into_iter()
        .find(
            |r| matches!(r.kind, RowKind::Act { command: Command::Build(b), .. } if b == trimming),
        )
        .expect("fixture: its row is listed");
    let mut v = on(&row.key);
    update(Action::Decrement, &mut v, &s);
    let report = s.raze_report(trimming).expect("a copy stands");
    assert!(
        !report.trimmed.is_empty(),
        "fixture: a store would be trimmed"
    );
    let text = frame_text(&s, &v);
    for (r, amount) in &report.trimmed {
        assert!(
            text.contains(&render::fmt::exact(*amount)),
            "the amount of {r:?} trimmed, {amount}: {text}"
        );
    }
    assert!(text.contains("trims"), "says what happens to it: {text}");

    // Afresh on every frame.
    s.structures[trimming.ix()] = 5;
    assert!(
        frame_text(&s, &v).contains("4 left"),
        "the copies left, now"
    );
}

/// The hint bar names the key on a structure's row with a copy standing, and on
/// no other.
///
/// @spec TUI-NAV-206
#[test]
fn the_hint_bar_names_the_key_only_where_it_acts() {
    let _clock = now::held();
    let hint = |s: &GameState, v: &ViewState| {
        let mut term = Terminal::new(TestBackend::new(200, 1)).unwrap();
        term.draw(|f| {
            let area = f.area();
            chrome::hints(f, area, s, v, false);
        })
        .unwrap();
        let buf = term.backend().buffer().clone();
        (0..200)
            .map(|x| buf[(x, 0)].symbol().to_string())
            .collect::<String>()
    };
    let (s, key, _) = meadow(1);
    assert!(hint(&s, &on(&key)).contains("- take down"), "a copy stands");

    let (none, key, _) = meadow(0);
    assert!(!hint(&none, &on(&key)).contains("take down"), "none stands");

    let (s, _, _) = meadow(1);
    let other = rows(&s, Section::Meadow)
        .into_iter()
        .find(|r| matches!(r.kind, RowKind::Act { command, .. } if !matches!(command, Command::Build(_))))
        .expect("fixture: a row that builds nothing");
    assert!(
        !hint(&s, &on(&other.key)).contains("take down"),
        "not a structure"
    );
}

/// The help overlay lists it among the keys.
///
/// @spec TUI-NAV-206
#[test]
fn the_help_overlay_lists_it() {
    let _clock = now::held();
    let (s, key, _) = meadow(1);
    let mut v = on(&key);
    v.overlay = Some(clowder_tui::nav::Overlay::Help);
    assert!(frame_text(&s, &v).contains("takes down a building"));
}

fn press(code: KeyCode) -> TermEvent {
    TermEvent::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("clowder-raze-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("save.json")
}

/// Through the game's own frames: the key arms, Enter commits, one copy comes
/// down, what comes home comes home, and the journal says so.
///
/// @spec TUI-NAV-204
#[test]
fn a_copy_comes_down_through_the_games_own_frames() {
    let _clock = now::held();
    let (s, key, st) = meadow(3);
    let report = s.raze_report(st).expect("a copy stands");
    let mut app = App {
        state: s.clone(),
        view: on(&key),
        clock: TickAccumulator::default(),
        save_path: scratch("frames"),
        last_save: Instant::now(),
        prev_layout: None,
        pointer_record: None,
        quitting: false,
        save_failed: false,
    };
    let mut term = Terminal::new(TestBackend::new(120, 40)).unwrap();
    let still = Reading {
        monotonic: Duration::ZERO,
        wall: Duration::ZERO,
    };

    app.step(&mut term, still, &[press(KeyCode::Backspace)])
        .unwrap();
    assert_eq!(app.state.structures[st.ix()], 3, "armed, not done");
    now::advance(DISTINCT);
    app.step(&mut term, still, &[press(KeyCode::Enter)])
        .unwrap();
    assert_eq!(app.state.structures[st.ix()], 2, "one came down");
    for (r, amount) in &report.returned {
        assert!(
            (app.state.stock(*r) - s.stock(*r) - amount).abs() < 1e-9,
            "{r:?} came home"
        );
    }
    assert!(
        app.state
            .journal
            .iter_chronological()
            .iter()
            .any(|e| format!("{e:?}").contains("StructureRazed")),
        "the journal keeps it"
    );
}

/// The core's refusal in plain words, if the copy has gone by the commit.
///
/// @spec TUI-NAV-014, TUI-NAV-204
#[test]
fn a_refusal_reads_in_plain_words() {
    let words = clowder_tui::nav::plain_error(&clowder_core::CommandError::NothingToRaze);
    assert!(words.contains("none"), "{words}");
}
