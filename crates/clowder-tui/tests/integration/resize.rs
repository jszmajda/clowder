//! A resize, as a session lives it: the terminal's size changes and the
//! player's view does not (TUI-NAV-029), the inspector call is held at every
//! width (TUI-NAV-029, TUI-NAV-088), and below the floor the too-small card
//! draws its one line whole (TUI-NAV-087).
//!
//! @spec TUI-NAV-029, TUI-NAV-087
//!
//! **Why through the loop.** A resize reaches the game as an
//! `Event::Resize` handed to `App::step`, and the layout the next frame
//! draws is the one the step computes from the terminal's size and the
//! previous frame's layout. So every resize here is both: the in-memory
//! backend is resized (a real terminal's size changes before the event
//! reports it) and the resize event is delivered to a real step. No time
//! passes in any step unless a test says so, so nothing the clock drives can
//! stand in for something the resize did.
//!
//! **How "unchanged" is read.** The whole view state is compared by its
//! Debug form, before and after. That form carries every piece the spec
//! names — the section in focus, every section's remembered row, an open
//! field and what is typed into it (the count prompt's held row included),
//! an overlay and its scroll position, an arming and its expiry instant, the
//! report queue with each report's unread state, a footer note and its
//! expiry instant, and the inspector call — and every visit's watermark,
//! which is how "counts as no visit" is observed. Nothing is pinned or
//! normalised: a resize that re-timed a clock would show.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use clowder_core::{
    Command, GameState, IdeaId, LoadedGame, OfflineReport, ResourceId, StructureId,
};
use clowder_tui::nav::rows::{rows, RowKind};
use clowder_tui::nav::{Overlay, RowKey, Section, ViewState};
use clowder_tui::render::layout::Body;
use clowder_tui::render::widgets::inspector;
use clowder_tui::runtime::clock::Reading;
use clowder_tui::runtime::{App, TickAccumulator};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::Terminal;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn scratch(name: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir =
        std::env::temp_dir().join(format!("clowder-resize-{}-{name}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("save.json")
}

/// A meadow with plenty of everything, every structure revealed, and a nap
/// listed and affordable — rows to remember in more than one section, a heavy
/// row to arm, and a count row to open a prompt on.
fn meadow() -> GameState {
    let mut s = GameState::new(4242);
    for r in ResourceId::ALL {
        s.resources[r.ix()] = 1.0e9;
    }
    s.revealed = StructureId::ALL
        .iter()
        .map(|b| clowder_core::ContentId::Structure(*b))
        .collect();
    s.ideas.insert(IdeaId::DeepNapTheory);
    s.resources[ResourceId::Starshards.ix()] = 1.0e12;
    s.ticks = 5_000;
    s
}

/// A distinct report per `n`, long enough that the core keeps it.
fn away_report(n: u64) -> OfflineReport {
    let mut r = OfflineReport {
        elapsed_seconds: (3 + n) * 86_400,
        ..OfflineReport::default()
    };
    r.resource_deltas[ResourceId::Feathers.ix()] = 4_800.0 + n as f64;
    r
}

fn nap_key(state: &GameState) -> RowKey {
    rows(state, Section::Dreams)
        .into_iter()
        .find(|r| {
            matches!(
                r.kind,
                RowKind::Act {
                    command: Command::DeepNap(_),
                    ..
                }
            )
        })
        .map(|r| r.key)
        .expect("fixture: the nap row is listed")
}

fn meadow_rows(state: &GameState) -> Vec<RowKey> {
    rows(state, Section::Meadow)
        .into_iter()
        .filter(|r| r.selectable())
        .map(|r| r.key)
        .collect()
}

/// A session past its opening, with a queue of three reports — the newest two
/// unread, so one banner shows and another waits behind it, and the oldest
/// already read — a remembered row in the Meadow that is not its first, and
/// the focus on the nap row in the Dreams section.
fn session(name: &str) -> App {
    let mut state = meadow();
    let mut view = ViewState::default();
    clowder_tui::nav::opening(&mut view, &state);
    for n in 0..3 {
        state.ticks += 1;
        let report = away_report(n);
        state.journal.record_away(state.ticks, &report);
        let loaded = LoadedGame {
            state: state.clone(),
            report,
        };
        clowder_tui::nav::queue_load_report(&mut view, &loaded);
    }
    assert_eq!(
        view.reports.len(),
        3,
        "fixture: all three reports queued a banner"
    );
    view.reports[2].dismissed = true;

    let list = meadow_rows(&state);
    assert!(
        list.len() > 3,
        "fixture: the Meadow lists enough rows to remember one past the first"
    );
    view.focus[Section::Meadow.index()] = Some(list[2].clone());
    view.section = Some(Section::Dreams);
    view.focus[Section::Dreams.index()] = Some(nap_key(&state));

    App {
        state,
        view,
        clock: TickAccumulator::default(),
        save_path: scratch(name),
        last_save: Instant::now(),
        prev_layout: None,
        pointer_record: None,
        quitting: false,
        save_failed: false,
    }
}

fn still() -> Reading {
    Reading {
        monotonic: Duration::ZERO,
        wall: Duration::ZERO,
    }
}

fn elapsed(secs: f64) -> Reading {
    let d = Duration::from_secs_f64(secs);
    Reading {
        monotonic: d,
        wall: d,
    }
}

fn term(w: u16, h: u16) -> Terminal<TestBackend> {
    Terminal::new(TestBackend::new(w, h)).expect("TestBackend is infallible")
}

fn press(code: KeyCode) -> Event {
    Event::Key(KeyEvent::from(code))
}

fn press_char(c: char) -> Event {
    let modifiers = if c.is_ascii_uppercase() {
        KeyModifiers::SHIFT
    } else {
        KeyModifiers::NONE
    };
    Event::Key(KeyEvent::new(KeyCode::Char(c), modifiers))
}

/// Step once with these keys and no time passing.
fn keys(a: &mut App, t: &mut Terminal<TestBackend>, input: &[Event]) {
    a.step(t, still(), input)
        .expect("a step against a TestBackend does not fail");
}

/// Resize the terminal to `w`×`h` as a real one is resized: its size changes,
/// and the change arrives as an event to the next step.
fn resize(a: &mut App, t: &mut Terminal<TestBackend>, w: u16, h: u16) {
    t.backend_mut().resize(w, h);
    a.step(t, still(), &[Event::Resize(w, h)])
        .expect("a step against a TestBackend does not fail");
}

fn view_of(a: &App) -> String {
    format!("{:?}", a.view)
}

fn drawn(a: &App) -> Body {
    a.prev_layout.expect("a step drew a frame").body
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

/// The inspector widget drawn alone into `rect` of a terminal this size.
fn inspector_alone(w: u16, h: u16, rect: Rect, state: &GameState, view: &ViewState) -> Vec<String> {
    let mut t = term(w, h);
    t.draw(|f| inspector::draw(f, rect, state, view))
        .expect("TestBackend is infallible");
    region(t.backend().buffer(), rect)
}

/// A long-lived footer note, set the way a rejection leaves one.
fn note(a: &mut App) {
    a.view.footer = Some((
        "a note already showing".to_string(),
        Instant::now() + Duration::from_secs(600),
    ));
}

/// The topmost transient a session is built with, above the arming.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Top {
    Nothing,
    Prompt,
    Palette,
    Help,
    Journal,
}

const TOPS: [Top; 5] = [
    Top::Nothing,
    Top::Prompt,
    Top::Palette,
    Top::Help,
    Top::Journal,
];

/// A session carrying every piece of view state a resize must keep, built by
/// a player's own keys at the floor: the inspector called for (where
/// `wanted`), the nap row armed, `top` open above it with something typed
/// into it where it is a field, the Journal scrolled off its top, and a
/// footer note on its own clock.
fn loaded_session(top: Top, wanted: bool, name: &str) -> (App, Terminal<TestBackend>) {
    let mut a = session(name);
    let mut t = term(80, 24);
    keys(&mut a, &mut t, &[]);
    if wanted {
        keys(&mut a, &mut t, &[press_char(' ')]);
        assert!(
            a.view.inspector_wanted,
            "fixture: Space at the floor called for the inspector"
        );
    }
    keys(&mut a, &mut t, &[press(KeyCode::Enter)]);
    assert!(a.view.armed.is_some(), "fixture: Enter armed the nap row");
    match top {
        Top::Nothing => {}
        Top::Prompt => {
            keys(
                &mut a,
                &mut t,
                &[press_char('x'), press_char('3'), press_char('7')],
            );
            let prompt = a
                .view
                .prompt
                .as_ref()
                .expect("fixture: x opened the count prompt");
            assert_eq!(
                prompt.digits, "37",
                "fixture: 37 was typed into the count prompt"
            );
            assert!(
                prompt.row.is_some(),
                "fixture: the count prompt is held to a row"
            );
        }
        Top::Palette => {
            keys(
                &mut a,
                &mut t,
                &[press_char('/'), press_char('a'), press_char('b')],
            );
            assert!(
                matches!(&a.view.overlay, Some(Overlay::Palette { query, .. }) if query == "ab"),
                "fixture: the palette opened with ab typed into it"
            );
        }
        Top::Help => {
            keys(&mut a, &mut t, &[press_char('?')]);
            assert_eq!(a.view.overlay, Some(Overlay::Help), "fixture: help opened");
        }
        Top::Journal => {
            keys(&mut a, &mut t, &[press_char('e')]);
            assert!(
                matches!(a.view.overlay, Some(Overlay::Journal { .. })),
                "fixture: the Journal opened"
            );
            a.view.overlay = Some(Overlay::Journal { anchor: 2 });
        }
    }
    assert!(
        a.view.armed.is_some(),
        "fixture: {top:?} left the nap armed beneath"
    );
    assert_eq!(
        a.view.inspector_wanted, wanted,
        "fixture: the inspector call is as built"
    );
    note(&mut a);
    (a, t)
}

/// Every kind of resize: across the pane breakpoint both ways, into and out of
/// its dead-band, into the too-small card from the narrow layout and from the
/// pane, out of the card to each, below the floor by width alone, by height
/// alone, and by both, down to a single cell, and back to the floor.
const WALK: [(u16, u16); 16] = [
    (130, 40),
    (100, 30),
    (97, 30),
    (90, 30),
    (60, 20),
    (240, 60),
    (79, 24),
    (80, 23),
    (200, 10),
    (1, 1),
    (80, 24),
    (40, 40),
    (99, 24),
    (120, 30),
    (79, 60),
    (80, 24),
];

// ===========================================================================
// TUI-NAV-029 — a resize changes how the view is drawn, never the view
// ===========================================================================

/// Every piece of view state stands through every resize, whatever is open:
/// the section in focus and each section's remembered row; an open field with
/// what is typed into it, the count prompt's held row included; an open
/// full-screen overlay at its scroll position; an arming with its expiry
/// still running; the banner showing and each queued report's unread state;
/// a footer note on its own clock; every visit's watermark; and the inspector
/// call, asked for or not. A resize is neither a key nor navigation, so it
/// ends no arming, retires no banner, reads no report, and counts as no
/// visit.
///
/// @spec TUI-NAV-029
#[test]
fn a_resize_leaves_every_piece_of_view_state_as_it_stood() {
    for top in TOPS {
        for wanted in [false, true] {
            let (mut a, mut t) = loaded_session(top, wanted, "walk");
            let before = view_of(&a);
            let (mut w0, mut h0) = (80u16, 24u16);
            for (w, h) in WALK {
                resize(&mut a, &mut t, w, h);
                assert_eq!(
                    view_of(&a),
                    before,
                    "with {top:?} open above an arming (inspector called for: {wanted}), a \
                     resize from {w0}×{h0} to {w}×{h} (drawn: {:?}) changed the view state; a \
                     resize changes only how it is drawn",
                    drawn(&a)
                );
                assert!(!a.quitting, "a resize to {w}×{h} quit the game");
                (w0, h0) = (w, h);
            }
        }
    }
}

/// The inspector call is held at every width, and neither a resize nor a key
/// pressed while the pane or the too-small card is drawn sets or clears it.
/// So a swap widened into the pane and narrowed back is in again, and a list
/// view widened and narrowed back is the list view — whatever Space and i
/// presses fell while the pane or the card was on screen between, and
/// however the terminal got there: straight across the breakpoint, into the
/// dead-band where the pane holds, under the card, or through all of them.
/// On return to the floor the frame draws what the call says: the inspector
/// in the list's space where it was called for, and the list where it was
/// not.
///
/// @spec TUI-NAV-029, TUI-NAV-088
#[test]
fn the_inspector_call_survives_a_round_trip_through_the_pane_and_the_card() {
    let space = || press_char(' ');
    let i = || press_char('i');
    // Each excursion: the sizes it passes through, each with the keys
    // pressed there. Every excursion presses an odd number of times, so a
    // key that flipped the call anywhere but the narrow layout shows.
    type Excursion = (&'static str, Vec<((u16, u16), Vec<Event>)>);
    let excursions: Vec<Excursion> = vec![
        ("across the breakpoint", vec![((130, 40), vec![space()])]),
        (
            "across the breakpoint, pressing each key",
            vec![
                ((130, 40), vec![space(), i(), space()]),
                ((100, 30), vec![i()]),
            ],
        ),
        (
            "into the pane, then back into its dead-band",
            vec![
                ((100, 30), vec![i()]),
                ((97, 30), vec![space(), i(), space()]),
            ],
        ),
        (
            "under the card",
            vec![((60, 20), vec![space(), i(), space()])],
        ),
        (
            "under the card by height alone",
            vec![((200, 10), vec![i()])],
        ),
        (
            "through the pane and the card",
            vec![
                ((160, 40), vec![space()]),
                ((70, 24), vec![i(), space()]),
                ((120, 30), vec![space(), space(), i()]),
                ((79, 24), vec![i()]),
            ],
        ),
    ];

    for wanted in [false, true] {
        for (what, excursion) in &excursions {
            let mut a = session("round-trip");
            let mut t = term(80, 24);
            keys(&mut a, &mut t, &[]);
            if wanted {
                keys(&mut a, &mut t, &[press_char(' ')]);
            }
            assert_eq!(
                a.view.inspector_wanted, wanted,
                "fixture: the call is as built"
            );

            for ((w, h), pressed) in excursion {
                resize(&mut a, &mut t, *w, *h);
                assert!(
                    !matches!(drawn(&a), Body::Narrow { .. }),
                    "fixture: {what} at {w}×{h} should not draw the narrow layout"
                );
                assert_eq!(
                    a.view.inspector_wanted, wanted,
                    "{what}: the resize to {w}×{h} changed the inspector call"
                );
                for key in pressed {
                    keys(&mut a, &mut t, std::slice::from_ref(key));
                    assert_eq!(
                        a.view.inspector_wanted,
                        wanted,
                        "{what}: {key:?} at {w}×{h} ({:?} drawn) changed the inspector call; it \
                         is set or cleared only where the narrow layout is on screen",
                        drawn(&a)
                    );
                }
            }

            resize(&mut a, &mut t, 80, 24);
            assert_eq!(
                a.view.inspector_wanted, wanted,
                "{what}: back at the floor, the inspector call is not the one the player left"
            );
            let main = match drawn(&a) {
                Body::Narrow { main, .. } => main,
                other => panic!("fixture: 80×24 draws the narrow layout, drew {other:?}"),
            };
            // The pane divider (TUI-RENDER-016) is taken from the main
            // pane's edge facing the sidebar, so the swap draws into what it
            // leaves, not the raw layout rect.
            let space = Rect::new(main.x + 1, main.y, main.width - 1, main.height);
            let shown = region(t.backend().buffer(), space);
            let alone = inspector_alone(80, 24, space, &a.state, &a.view);
            if wanted {
                assert_eq!(
                    shown, alone,
                    "{what}: a swap widened out and narrowed back must be in again"
                );
            } else {
                assert_ne!(
                    shown, alone,
                    "{what}: a list view widened out and narrowed back must be the list view"
                );
            }
        }
    }
}

/// Below 80 columns or 24 rows the too-small card is drawn in place of the
/// frame, the simulation keeps advancing beneath it, and returning to 80×24
/// draws the frame the view state describes with whatever the sim did
/// meanwhile — the same frame a session that had never been under the card
/// draws from that state and view.
///
/// @spec TUI-NAV-029
#[test]
fn under_the_card_the_sim_advances_and_the_frame_returns_as_the_view_describes() {
    for (w, h) in [(79u16, 24u16), (80, 23), (60, 20), (200, 10), (1, 1)] {
        let (mut a, mut t) = loaded_session(Top::Nothing, true, "card-sim");
        resize(&mut a, &mut t, w, h);
        assert!(
            matches!(drawn(&a), Body::TooSmall(_)),
            "{w}×{h} is below the floor, so the frame is the too-small card; drew {:?}",
            drawn(&a)
        );
        let ticks = a.state.ticks;
        a.step(&mut t, elapsed(3.0), &[])
            .expect("a step below the floor must not fail");
        assert!(
            a.state.ticks > ticks,
            "under the too-small card at {w}×{h}, three seconds passed and the sim did not advance"
        );

        resize(&mut a, &mut t, 80, 24);
        let mut fresh = term(80, 24);
        fresh
            .draw(|f| {
                clowder_tui::render::frame(f, &a.state, &a.view, None);
            })
            .expect("TestBackend is infallible");
        assert_eq!(
            region(t.backend().buffer(), Rect::new(0, 0, 80, 24)),
            region(fresh.backend().buffer(), Rect::new(0, 0, 80, 24)),
            "back at 80×24 from {w}×{h}, the frame is not the one the view state describes"
        );
    }
}

// ===========================================================================
// TUI-NAV-087 — the too-small card's one line, whole
// ===========================================================================

/// The card's one line (navigation design, Resolved Edge Cases: What a resize
/// keeps, under the too-small card): calm, saying nothing is wrong, and naming
/// the size the game needs.
const CARD_LINE: &str = "this pane is too cozy \u{2014} 80\u{d7}24 needed";

/// Draw one frame of a session at `w`×`h` and return every row of it.
fn frame_rows(state: &GameState, view: &ViewState, w: u16, h: u16) -> Vec<String> {
    let mut t = term(w, h);
    t.draw(|f| {
        clowder_tui::render::frame(f, state, view, None);
    })
    .expect("TestBackend is infallible");
    region(t.backend().buffer(), Rect::new(0, 0, w, h))
}

/// Wherever the terminal is under 80 columns or 24 rows and the pane is at
/// least as wide as the card's line and at least one row tall, the card draws
/// that line whole — on one row, every character of it — and nothing else:
/// it is drawn in place of the whole frame. Every such size is checked, not a
/// sample: every width from the line's own width to the widest a terminal is
/// taken to here, at every height the card can show at.
///
/// @spec TUI-NAV-087
#[test]
fn the_too_small_card_draws_its_one_line_whole_wherever_it_has_room() {
    let line_w = CARD_LINE.chars().count() as u16;
    let state = meadow();
    let mut view = ViewState::default();
    clowder_tui::nav::opening(&mut view, &state);

    let mut sizes: Vec<(u16, u16)> = Vec::new();
    for w in line_w..80 {
        for h in 1..=60 {
            sizes.push((w, h));
        }
    }
    for w in 80..=240 {
        for h in 1..24 {
            sizes.push((w, h));
        }
    }

    for (w, h) in sizes {
        let rows = frame_rows(&state, &view, w, h);
        let drawn: Vec<(usize, &str)> = rows
            .iter()
            .enumerate()
            .filter(|(_, r)| !r.trim().is_empty())
            .map(|(y, r)| (y, r.trim()))
            .collect();
        assert_eq!(
            drawn.len(),
            1,
            "at {w}×{h} the too-small card must draw exactly one line and nothing else; it drew \
             {drawn:?}"
        );
        assert_eq!(
            drawn[0].1, CARD_LINE,
            "at {w}×{h} the too-small card's line must be drawn whole: {CARD_LINE:?}"
        );
    }
}
