//! Esc's order of transients (TUI-NAV-030) and the narrow-mode inspector
//! swap's keys (TUI-NAV-035, TUI-NAV-088), as a session lives them — every key pressed
//! through `App::step`, at the width the terminal has.
//!
//! @spec TUI-NAV-030, TUI-NAV-035, TUI-NAV-088
//!
//! **Why through the loop.** Resolution (`nav::keys::resolve`) and the
//! reducer (`nav::update`) each see only part of what these two specs decide.
//! Whether the narrow-mode swap is a transient Esc closes depends on the
//! layout on screen at the press — the terminal's width *and* which side of
//! the inspector breakpoint's dead-band the previous frame settled on
//! (TUI-NAV-047) — and only the loop holds both. So every key here is a real
//! key event handed to a real step, and "the layout on screen" is whatever
//! the previous step drew.
//!
//! **How "everything beneath it as it stood" is read.** The whole view is
//! compared, by its Debug form, against the view before the press with only
//! the closed transient taken away. Esc touches no clock, so the footer
//! note's own expiry and the arming's expiry compare exactly: a footer note
//! reset, cleared, or re-timed by Esc shows up as a difference, as does a
//! moved cursor, a changed section, a visit's watermark, or a second report
//! retired. The game state is compared whole as well, since Esc never issues
//! a command.
//!
//! **How two sessions are compared.** For TUI-NAV-035's "keeps its list-view
//! meaning", the same key is pressed in two sessions built identically but
//! for the swap, and the views are compared with every wall-clock instant
//! pinned to one value, since each session reads `Instant::now()` at its own
//! moment.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use clowder_core::{
    Command, GameState, IdeaId, LoadedGame, OfflineReport, ResourceId, StructureId,
};
use clowder_tui::nav::rows::{rows, RowKind};
use clowder_tui::nav::{RowKey, Section, ViewState};
use clowder_tui::render::layout::Body;
use clowder_tui::render::widgets::inspector;
use clowder_tui::runtime::clock::Reading;
use clowder_tui::runtime::{App, TickAccumulator};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::Terminal;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// A fresh directory per call, not per name: tests run in parallel within one
/// process, and two callers sharing a name would race one another's
/// remove-and-recreate.
fn scratch(name: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir()
        .join(format!("clowder-transients-{}-{name}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("save.json")
}

/// A meadow with plenty of everything, every structure revealed, and a nap
/// that is listed and affordable — so there are rows to move among, a heavy
/// row to arm, and count rows to open a prompt on.
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

/// A report a load's catch-up would fold, so a banner is showing.
fn away_report() -> OfflineReport {
    let mut r = OfflineReport { elapsed_seconds: 3 * 86_400, ..OfflineReport::default() };
    r.resource_deltas[ResourceId::Feathers.ix()] = 4_800.0;
    r.arrived = vec![("Quenby".to_string(), 1)];
    r
}

/// A session opened on `section` with the cursor on `focus`, and — where
/// `banner` — one load's report queued, so its banner shows.
fn session(focus: Option<(Section, RowKey)>, banner: bool, name: &str) -> App {
    let mut state = meadow();
    let mut view = ViewState { section: Some(Section::Meadow), ..ViewState::default() };
    if banner {
        let report = away_report();
        state.journal.record_away(state.ticks, &report);
        let loaded = LoadedGame { state: state.clone(), report };
        clowder_tui::nav::queue_load_report(&mut view, &loaded);
        assert!(
            view.reports.iter().any(|r| !r.dismissed),
            "fixture: the load's report queued a banner"
        );
    }
    if let Some((section, key)) = focus {
        view.section = Some(section);
        view.focus[section.index()] = Some(key);
    }
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

fn nap_key(state: &GameState) -> RowKey {
    rows(state, Section::Dreams)
        .into_iter()
        .find(|r| matches!(r.kind, RowKind::Act { command: Command::DeepNap(_), .. }))
        .map(|r| r.key)
        .expect("fixture: the nap row is listed")
}

fn meadow_rows(state: &GameState) -> Vec<RowKey> {
    rows(state, Section::Meadow).into_iter().filter(|r| r.selectable()).map(|r| r.key).collect()
}

fn still() -> Reading {
    Reading { monotonic: Duration::ZERO, wall: Duration::ZERO }
}

fn term(w: u16, h: u16) -> Terminal<TestBackend> {
    Terminal::new(TestBackend::new(w, h)).expect("TestBackend is infallible")
}

fn press(code: KeyCode) -> Event {
    Event::Key(KeyEvent::from(code))
}

fn press_char(c: char) -> Event {
    let modifiers = if c.is_ascii_uppercase() { KeyModifiers::SHIFT } else { KeyModifiers::NONE };
    Event::Key(KeyEvent::new(KeyCode::Char(c), modifiers))
}

fn esc() -> Event {
    press(KeyCode::Esc)
}

/// A held Esc's auto-repeat, as the terminal reports it.
fn esc_repeat() -> Event {
    Event::Key(KeyEvent {
        code: KeyCode::Esc,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Repeat,
        state: KeyEventState::NONE,
    })
}

/// Step once with these keys and no time passing.
fn keys(a: &mut App, t: &mut Terminal<TestBackend>, input: &[Event]) {
    a.step(t, still(), input).expect("a step against a TestBackend does not fail");
}

fn view_of(a: &App) -> String {
    format!("{:?}", a.view)
}

/// The view with every wall-clock instant pinned, so two sessions that read
/// `Instant::now()` at different moments compare on everything else.
fn shape(v: &ViewState, base: Instant) -> String {
    let mut v = v.clone();
    if let Some((_, until)) = v.footer.as_mut() {
        *until = base;
    }
    if let Some(armed) = v.armed.as_mut() {
        armed.expires_at = base;
    }
    if v.last_repeat.is_some() {
        v.last_repeat = Some(base);
    }
    format!("{v:?}")
}

fn showing_banner(a: &App) -> bool {
    a.view.reports.iter().any(|r| !r.dismissed)
}

/// Retire the one banner showing, as the closing of that transient would.
fn retire_showing(v: &mut ViewState) {
    if let Some(r) = v.reports.iter_mut().find(|r| !r.dismissed) {
        r.dismissed = true;
    }
}

/// The space the swap draws into — the narrow body's one pane, with its
/// pane divider (TUI-RENDER-016) set aside, since the divider is inert
/// chrome on the pane's edge column facing the sidebar, never the swap's own
/// content.
fn main_rect(a: &App) -> Rect {
    match a.prev_layout.expect("a step drew a frame").body {
        Body::Narrow { main, .. } => Rect::new(main.x + 1, main.y, main.width - 1, main.height),
        other => panic!("fixture: expected the narrow layout, drew {other:?}"),
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

/// The inspector widget drawn alone into `rect` of a terminal this size.
fn inspector_alone(w: u16, h: u16, rect: Rect, state: &GameState, view: &ViewState) -> Vec<String> {
    let mut t = term(w, h);
    t.draw(|f| inspector::draw(f, rect, state, view)).expect("TestBackend is infallible");
    region(t.backend().buffer(), rect)
}

/// A long-lived footer note, set the way a rejection leaves one.
fn note(a: &mut App) {
    a.view.footer = Some(("a note already showing".to_string(), Instant::now() + Duration::from_secs(600)));
}

// ---------------------------------------------------------------------------
// TUI-NAV-030 — one transient per press, topmost first
// ---------------------------------------------------------------------------

/// The layers a stack can hold, topmost first.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Layer {
    Prompt,
    Palette,
    Help,
    Journal,
    Armed,
    Swap,
    Banner,
}

/// What closing `layer` takes away from the view, and nothing else.
fn close(v: &mut ViewState, layer: Layer) {
    match layer {
        Layer::Prompt => v.prompt = None,
        Layer::Palette | Layer::Help => v.overlay = None,
        Layer::Journal => {
            v.overlay = None;
            v.journal_pinned.clear();
        }
        Layer::Armed => v.armed = None,
        Layer::Swap => v.inspector_wanted = false,
        Layer::Banner => retire_showing(v),
    }
}

/// A stack built by a player's own keys at the floor, where the inspector is
/// a swap: on the nap row with a banner showing, Space swaps the inspector
/// in, Enter arms the nap, and `top` opens the topmost transient with
/// something typed into it where it is a field. Returns the session, its
/// terminal, and the layers open, topmost first.
fn stack(top: Layer, name: &str) -> (App, Terminal<TestBackend>, Vec<Layer>) {
    let nap = nap_key(&meadow());
    let mut a = session(Some((Section::Dreams, nap)), true, name);
    let mut t = term(80, 24);
    keys(&mut a, &mut t, &[]);
    keys(&mut a, &mut t, &[press_char(' ')]);
    assert!(a.view.inspector_wanted, "fixture: Space swapped the inspector in");
    keys(&mut a, &mut t, &[press(KeyCode::Enter)]);
    assert!(a.view.armed.is_some(), "fixture: Enter armed the nap row");
    let opener: Vec<Event> = match top {
        Layer::Prompt => vec![press_char('x'), press_char('3')],
        Layer::Palette => vec![press_char('/'), press_char('a'), press_char('b')],
        Layer::Help => vec![press_char('?')],
        Layer::Journal => vec![press_char('e')],
        other => panic!("fixture: {other:?} is not a top layer"),
    };
    keys(&mut a, &mut t, &opener);
    match top {
        Layer::Prompt => assert_eq!(
            a.view.prompt.as_ref().map(|p| p.digits.as_str()),
            Some("3"),
            "fixture: x opened the count prompt and 3 was typed into it"
        ),
        _ => assert!(a.view.overlay.is_some(), "fixture: {top:?} opened"),
    }
    assert!(a.view.armed.is_some(), "fixture: opening {top:?} left the nap armed beneath");
    assert!(a.view.inspector_wanted, "fixture: opening {top:?} left the swap in beneath");

    let mut layers = vec![top, Layer::Armed, Layer::Swap];
    if showing_banner(&a) {
        layers.push(Layer::Banner);
    } else {
        assert_eq!(top, Layer::Journal, "fixture: only opening the Journal reads the banner");
    }
    (a, t, layers)
}

const TOPS: [Layer; 4] = [Layer::Prompt, Layer::Palette, Layer::Help, Layer::Journal];

/// Each Esc closes exactly the topmost transient — a field or full-screen
/// overlay, then the armed row, then the narrow-mode swap, then the banner —
/// and leaves everything beneath it as it stood: the other transients, the
/// cursor and section, every visit's watermark, the rest of the report
/// queue, a footer note already showing (its own clock untouched), and the
/// game. A field closing discards only what was typed into it. With nothing
/// left open, one more Esc does nothing at all.
///
/// @spec TUI-NAV-030
#[test]
fn each_esc_closes_exactly_the_topmost_transient_and_leaves_everything_beneath() {
    for top in TOPS {
        let (mut a, mut t, layers) = stack(top, "stack");
        note(&mut a);

        for layer in &layers {
            let mut expected = a.view.clone();
            close(&mut expected, *layer);
            let game_before = a.state.clone();
            keys(&mut a, &mut t, &[esc()]);
            assert_eq!(
                view_of(&a),
                format!("{expected:?}"),
                "with {layers:?} open under {top:?}, the Esc that should close only {layer:?} \
                 changed something else, or closed something other than {layer:?}"
            );
            assert!(a.state == game_before, "an Esc closing {layer:?} changed the game");
            assert!(!a.quitting, "an Esc closing {layer:?} quit the game");
        }

        // Nothing left: a bare screen.
        let before = view_of(&a);
        let game_before = a.state.clone();
        keys(&mut a, &mut t, &[esc()]);
        assert_eq!(
            view_of(&a),
            before,
            "with every transient closed ({top:?} stack), a further Esc changed the view; on a \
             bare screen Esc does nothing"
        );
        assert!(a.state == game_before, "an Esc on a bare screen changed the game");
        assert!(!a.quitting, "an Esc on a bare screen quit the game");
    }
}

/// A held Esc is a press per repeat: each of its repeats closes exactly one
/// transient, the next one down, never two and never none.
///
/// @spec TUI-NAV-030
#[test]
fn each_repeat_of_a_held_esc_closes_exactly_one_transient() {
    for top in TOPS {
        let (mut a, mut t, layers) = stack(top, "held");
        note(&mut a);
        for n in 1..=layers.len() {
            let (mut a2, mut t2, _) = stack(top, "held-n");
            note(&mut a2);
            let mut expected = a2.view.clone();
            for layer in &layers[..n] {
                close(&mut expected, *layer);
            }
            let mut held = vec![esc()];
            held.extend(std::iter::repeat_with(esc_repeat).take(n - 1));
            keys(&mut a2, &mut t2, &held);
            assert_eq!(
                view_of(&a2),
                format!("{expected:?}"),
                "an Esc held for {n} presses (one press, {} repeats) over {layers:?} did not \
                 close exactly the top {n} transients",
                n - 1
            );
        }
        // The same stack, one repeat past the last transient: nothing more.
        let mut expected = a.view.clone();
        for layer in &layers {
            close(&mut expected, *layer);
        }
        let mut held = vec![esc()];
        held.extend(std::iter::repeat_with(esc_repeat).take(layers.len()));
        keys(&mut a, &mut t, &held);
        assert_eq!(
            view_of(&a),
            format!("{expected:?}"),
            "a held Esc's repeat past the last transient over {layers:?} changed something"
        );
    }
}

/// The widths below the pane breakpoint at which a terminal that was showing
/// the pane keeps showing it — the dead-band, read off the layout the loop
/// actually draws rather than a constant, so it holds whatever width the band
/// is measured to.
fn dead_band_widths(h: u16) -> Vec<u16> {
    (80u16..100)
        .filter(|&w| {
            let mut a = session(None, false, "band");
            keys(&mut a, &mut term(100, h), &[]);
            keys(&mut a, &mut term(w, h), &[]);
            matches!(a.prev_layout.map(|l| l.body), Some(Body::Wide { .. }))
        })
        .collect()
}

/// Whether the swap is a transient Esc closes is judged by the layout on
/// screen at the press, with the breakpoint's dead-band applied. Below the
/// pane breakpoint but inside the band, a terminal that narrowed from a pane
/// is still showing the pane — there is no swap on screen — so Esc passes over
/// the inspector the player asked for and retires the banner, and with no
/// banner does nothing at all. The same width reached from below shows the
/// swap, and there Esc returns the list view first.
///
/// @spec TUI-NAV-030
#[test]
fn esc_judges_the_swap_by_the_layout_on_screen_with_the_dead_band_applied() {
    const H: u16 = 30;
    let band = dead_band_widths(H);
    assert!(
        !band.is_empty(),
        "fixture: no width under the pane breakpoint kept the pane after narrowing from it; \
         the dead-band (TUI-NAV-047) should hold at least one"
    );
    let focus = Some((Section::Meadow, meadow_rows(&meadow())[0].clone()));

    for w in 80u16..100 {
        let in_band = band.contains(&w);

        // Narrowed from a pane.
        for banner in [true, false] {
            let mut a = session(focus.clone(), banner, "narrowed");
            a.view.inspector_wanted = true;
            keys(&mut a, &mut term(100, H), &[]);
            let mut t = term(w, H);
            keys(&mut a, &mut t, &[]);
            note(&mut a);
            let mut expected = a.view.clone();
            if in_band {
                if banner {
                    close(&mut expected, Layer::Banner);
                }
            } else {
                close(&mut expected, Layer::Swap);
            }
            keys(&mut a, &mut t, &[esc()]);
            assert_eq!(
                view_of(&a),
                format!("{expected:?}"),
                "narrowed from a pane to {w} columns ({}), with the inspector asked for and {}, \
                 Esc {}",
                if in_band { "inside the dead-band, the pane still on screen" } else { "past the dead-band, the swap on screen" },
                if banner { "a banner showing" } else { "no banner" },
                if in_band {
                    if banner {
                        "must retire the banner and leave the inspector asked for: no swap is on screen"
                    } else {
                        "must do nothing: no swap is on screen and nothing else is open"
                    }
                } else {
                    "must return the list view and leave the rest as it stood"
                }
            );
        }

        // Reached from below: the swap is on screen at every narrow width.
        let mut a = session(focus.clone(), true, "widened");
        a.view.inspector_wanted = true;
        let mut t = term(w, H);
        keys(&mut a, &mut t, &[]);
        note(&mut a);
        let mut expected = a.view.clone();
        close(&mut expected, Layer::Swap);
        keys(&mut a, &mut t, &[esc()]);
        assert_eq!(
            view_of(&a),
            format!("{expected:?}"),
            "at {w} columns with the swap on screen, Esc must return the list view and leave the \
             banner as it stood"
        );
    }
}

/// On a bare screen — nothing Esc closes open, whatever the layout, a footer
/// note showing or not, the inspector a pane or not asked for — Esc changes
/// nothing: no command, no cursor or section move, no visit, no footer note,
/// no quit.
///
/// @spec TUI-NAV-030
#[test]
fn esc_on_a_bare_screen_changes_nothing_at_all() {
    let first = meadow_rows(&meadow())[0].clone();
    let cases: Vec<(&str, u16, u16, bool)> = vec![
        ("the floor, no inspector", 80, 24, false),
        ("the top of the narrow range, no inspector", 99, 24, false),
        ("a pane, asked for", 100, 30, true),
        ("a pane, not asked for", 160, 40, false),
        ("a pane at the widest, asked for", 160, 40, true),
    ];
    for (what, w, h, wanted) in cases {
        for footer in [false, true] {
            let mut a = session(Some((Section::Meadow, first.clone())), false, "bare");
            a.view.inspector_wanted = wanted;
            let mut t = term(w, h);
            keys(&mut a, &mut t, &[]);
            if footer {
                note(&mut a);
            }
            let before = view_of(&a);
            let game = a.state.clone();
            keys(&mut a, &mut t, &[esc(), esc_repeat(), esc_repeat()]);
            assert_eq!(view_of(&a), before, "on a bare screen ({what}), Esc changed the view");
            assert!(a.state == game, "on a bare screen ({what}), Esc changed the game");
            assert!(!a.quitting, "on a bare screen ({what}), Esc quit");
        }
    }

    // Inside the dead-band, narrowed from a pane: the pane is still on screen,
    // so the inspector asked for is no swap and the screen is bare.
    for w in dead_band_widths(30) {
        let mut a = session(Some((Section::Meadow, first.clone())), false, "bare-band");
        a.view.inspector_wanted = true;
        keys(&mut a, &mut term(100, 30), &[]);
        let mut t = term(w, 30);
        keys(&mut a, &mut t, &[]);
        let before = view_of(&a);
        keys(&mut a, &mut t, &[esc()]);
        assert_eq!(
            view_of(&a),
            before,
            "at {w} columns, narrowed from a pane and still showing it inside the dead-band, Esc \
             changed the view; nothing Esc closes is on screen"
        );
    }
}

/// Every key in the admissible set, as the terminal delivers it fused with a
/// late Esc that arrived just before it — an Alt/Meta chord.
fn fused_with_esc() -> Vec<KeyEvent> {
    let mut codes: Vec<KeyCode> = (' '..='~').map(KeyCode::Char).collect();
    codes.extend([
        KeyCode::Tab,
        KeyCode::BackTab,
        KeyCode::Enter,
        KeyCode::Esc,
        KeyCode::Backspace,
        KeyCode::Up,
        KeyCode::Down,
        KeyCode::Left,
        KeyCode::Right,
    ]);
    let mut out = Vec::new();
    for code in codes {
        for mods in [
            KeyModifiers::ALT,
            KeyModifiers::META,
            KeyModifiers::ALT | KeyModifiers::SHIFT,
            KeyModifiers::META | KeyModifiers::SHIFT,
        ] {
            out.push(KeyEvent::new(code, mods));
        }
    }
    out.push(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL | KeyModifiers::ALT));
    out
}

/// A key fused with a preceding late Esc is ignored whole, in every context a
/// player can be in when one arrives: it cannot act, commit an arming, open or
/// close anything, move anything, quit, or retire a report.
///
/// @spec TUI-NAV-030
#[test]
fn a_key_fused_with_a_late_esc_is_ignored_whole_through_the_loop() {
    let first = meadow_rows(&meadow())[0].clone();
    let mut contexts: Vec<(&str, App, Terminal<TestBackend>)> = Vec::new();

    let mut bare = session(Some((Section::Meadow, first.clone())), false, "fused-bare");
    let mut t = term(80, 24);
    keys(&mut bare, &mut t, &[]);
    contexts.push(("a bare screen", bare, t));

    let mut shown = session(Some((Section::Meadow, first)), true, "fused-banner");
    let mut t = term(160, 40);
    keys(&mut shown, &mut t, &[]);
    contexts.push(("a banner showing", shown, t));

    for top in TOPS {
        let (a, t, _) = stack(top, "fused-stack");
        contexts.push((
            match top {
                Layer::Prompt => "a prompt over an armed row, the swap, and a banner",
                Layer::Palette => "the palette over an armed row, the swap, and a banner",
                Layer::Help => "help over an armed row, the swap, and a banner",
                _ => "the Journal over an armed row and the swap",
            },
            a,
            t,
        ));
    }

    for (what, mut a, mut t) in contexts {
        for key in fused_with_esc() {
            let before = view_of(&a);
            let game = a.state.clone();
            keys(&mut a, &mut t, &[Event::Key(key)]);
            assert_eq!(
                view_of(&a),
                before,
                "with {what}, {key:?} — a key fused with a late Esc — changed the view; it must be \
                 ignored whole"
            );
            assert!(a.state == game, "with {what}, {key:?} changed the game");
            assert!(!a.quitting, "with {what}, {key:?} quit the game");
        }
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-035 — the narrow-mode swap keeps the list view's keys
// ---------------------------------------------------------------------------

/// Every global key but Space, i, and Esc: every printable character (the
/// global table, the digits, and every screen's mnemonic letters) and every
/// non-printable key the key map binds.
fn every_global_key_but_the_swaps_own() -> Vec<(String, Event)> {
    let mut out: Vec<(String, Event)> = (' '..='~')
        .filter(|c| *c != ' ' && *c != 'i')
        .map(|c| (format!("{c:?}"), press_char(c)))
        .collect();
    for code in [
        KeyCode::Up,
        KeyCode::Down,
        KeyCode::Left,
        KeyCode::Right,
        KeyCode::Tab,
        KeyCode::BackTab,
        KeyCode::Enter,
        KeyCode::Backspace,
    ] {
        out.push((format!("{code:?}"), press(code)));
    }
    out
}

/// Where the cursor starts: the top, middle and bottom of the Meadow, and the
/// nap row.
fn spots() -> Vec<(Section, RowKey)> {
    let state = meadow();
    let list = meadow_rows(&state);
    vec![
        (Section::Meadow, list[0].clone()),
        (Section::Meadow, list[list.len() / 2].clone()),
        (Section::Meadow, list[list.len() - 1].clone()),
        (Section::Dreams, nap_key(&state)),
    ]
}

/// While the swap is in and no field or overlay is open, every global key but
/// Space, i, and Esc means exactly what it means in the list view — the same
/// view change and the same command, the cursor, section and acting keys
/// alike — and leaves the swap in.
///
/// @spec TUI-NAV-035
#[test]
fn in_the_swap_every_global_key_but_space_i_and_esc_keeps_its_list_view_meaning() {
    let base = Instant::now();
    for (section, key) in spots() {
        for banner in [false, true] {
            for (what, event) in every_global_key_but_the_swaps_own() {
                let mut listing = session(Some((section, key.clone())), banner, "list");
                let mut swapped = session(Some((section, key.clone())), banner, "swap");
                swapped.view.inspector_wanted = true;
                let (mut tl, mut ts) = (term(80, 24), term(80, 24));
                keys(&mut listing, &mut tl, &[]);
                keys(&mut swapped, &mut ts, &[]);

                keys(&mut listing, &mut tl, std::slice::from_ref(&event));
                keys(&mut swapped, &mut ts, std::slice::from_ref(&event));

                assert!(
                    swapped.view.inspector_wanted,
                    "{what} with the cursor on {key:?} took the swap away; only Space, i, and Esc \
                     return the list view"
                );
                let mut as_listed = swapped.view.clone();
                as_listed.inspector_wanted = false;
                assert_eq!(
                    shape(&as_listed, base),
                    shape(&listing.view, base),
                    "{what} with the cursor on {key:?} ({}) did something different in the swap \
                     than in the list view",
                    if banner { "a banner showing" } else { "no banner" }
                );
                assert!(
                    swapped.state == listing.state,
                    "{what} with the cursor on {key:?} acted differently on the game in the swap \
                     than in the list view"
                );
                assert_eq!(swapped.quitting, listing.quitting, "{what} quit in one view only");
            }
        }
    }
}

/// The navigation keys, each of which moves the cursor or focus as in the
/// list view.
fn navigation_keys() -> Vec<(String, Event)> {
    let mut out: Vec<(String, Event)> = [
        KeyCode::Up,
        KeyCode::Down,
        KeyCode::Left,
        KeyCode::Right,
        KeyCode::Tab,
        KeyCode::BackTab,
    ]
    .into_iter()
    .map(|k| (format!("{k:?}"), press(k)))
    .collect();
    out.extend(['k', 'j', 'h', 'l', '[', ']'].into_iter().map(|c| (format!("{c:?}"), press_char(c))));
    out.extend(('0'..='9').map(|c| (format!("digit {c}"), press_char(c))));
    out
}

/// In the swap the cursor keys move the underlying cursor and the section
/// keys and digits move focus, and the inspector holding the list's space
/// shows the row the cursor lands on — after every such key, from every
/// starting spot.
///
/// @spec TUI-NAV-035
#[test]
fn in_the_swap_navigation_moves_the_cursor_and_the_inspector_shows_where_it_lands() {
    let (w, h) = (80u16, 24u16);
    for (section, key) in spots() {
        for (what, event) in navigation_keys() {
            let mut a = session(Some((section, key.clone())), false, "nav");
            a.view.inspector_wanted = true;
            let mut t = term(w, h);
            keys(&mut a, &mut t, &[]);
            keys(&mut a, &mut t, &[event]);
            assert!(a.view.inspector_wanted, "{what} took the swap away");
            let main = main_rect(&a);
            assert_eq!(
                region(t.backend().buffer(), main),
                inspector_alone(w, h, main, &a.state, &a.view),
                "after {what} from {key:?} in the swap, the list's space is not the inspector \
                 describing the row the cursor landed on"
            );
        }
    }
}

/// Space, i, and Esc (with nothing above the swap) each return the list view,
/// at the row the cursor is on — the row it moved to while the inspector was
/// up, not the one it started on — and move nothing.
///
/// @spec TUI-NAV-035
#[test]
fn space_i_and_esc_return_the_list_view_at_the_row_the_cursor_is_on() {
    let (w, h) = (80u16, 24u16);
    let list = meadow_rows(&meadow());
    for (what, event) in [("Space", press_char(' ')), ("i", press_char('i')), ("Esc", esc())] {
        let mut a = session(Some((Section::Meadow, list[0].clone())), false, "return");
        a.view.inspector_wanted = true;
        let mut t = term(w, h);
        keys(&mut a, &mut t, &[]);
        keys(&mut a, &mut t, &[press_char('j'), press_char('j')]);
        let moved = a.view.focus[Section::Meadow.index()].clone();
        assert_ne!(moved, Some(list[0].clone()), "fixture: j moved the cursor in the swap");
        let swapped_frame = region(t.backend().buffer(), main_rect(&a));

        keys(&mut a, &mut t, &[event]);
        assert!(!a.view.inspector_wanted, "{what} in the swap did not return the list view");
        assert_eq!(
            a.view.focus[Section::Meadow.index()],
            moved,
            "{what} returned the list view but moved the cursor"
        );
        assert_eq!(a.view.section, Some(Section::Meadow), "{what} moved the section");
        let main = main_rect(&a);
        let drawn = region(t.backend().buffer(), main);
        assert_ne!(drawn, swapped_frame, "{what} left the inspector in the list's space");
        assert_ne!(
            drawn,
            inspector_alone(w, h, main, &a.state, &a.view),
            "{what} left the inspector in the list's space"
        );

        // The list back is the list at that row: the same frame a session
        // that never swapped draws with the cursor there.
        let mut never = session(Some((Section::Meadow, moved.clone().unwrap())), false, "never");
        let mut tn = term(w, h);
        keys(&mut never, &mut tn, &[]);
        assert_eq!(
            drawn,
            region(tn.backend().buffer(), main_rect(&never)),
            "{what} returned a list that is not the list at the row the cursor is on"
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-088 — Space and i act on the call only where the swap can show it
// ---------------------------------------------------------------------------

/// The inspector keys, as the terminal delivers them.
fn inspector_keys() -> [(&'static str, Event); 2] {
    [("Space", press_char(' ')), ("i", press_char('i'))]
}

/// Space and i set the inspector call and clear it again wherever the narrow
/// layout is on screen at the press, judged as Esc's swap step is judged: by
/// the layout drawn, the breakpoint's dead-band applied. That is every width
/// from the floor to just under the pane breakpoint reached from below, at
/// more than one height, and every such width narrowed to from the pane that
/// lies past the dead-band.
///
/// @spec TUI-NAV-088
#[test]
fn space_and_i_set_and_clear_the_call_wherever_the_narrow_layout_is_on_screen() {
    let first = meadow_rows(&meadow())[0].clone();
    for h in [24u16, 30] {
        let band = dead_band_widths(h);
        for w in 80u16..100 {
            let mut approaches: Vec<(&str, u16)> = vec![("reached from below", 80)];
            if !band.contains(&w) {
                approaches.push(("narrowed from the pane past the dead-band", 100));
            }
            for (how, from) in approaches {
                for (what, key) in inspector_keys() {
                    let mut a = session(Some((Section::Meadow, first.clone())), false, "narrow-key");
                    keys(&mut a, &mut term(from, h), &[]);
                    let mut t = term(w, h);
                    keys(&mut a, &mut t, &[]);
                    assert!(
                        matches!(a.prev_layout.map(|l| l.body), Some(Body::Narrow { .. })),
                        "fixture: {w}×{h} {how} draws the narrow layout"
                    );
                    assert!(!a.view.inspector_wanted, "fixture: no inspector called for yet");

                    keys(&mut a, &mut t, std::slice::from_ref(&key));
                    assert!(
                        a.view.inspector_wanted,
                        "{what} at {w}×{h} ({how}), the narrow layout on screen, did not call \
                         for the inspector"
                    );
                    keys(&mut a, &mut t, std::slice::from_ref(&key));
                    assert!(
                        !a.view.inspector_wanted,
                        "{what} at {w}×{h} ({how}) with the swap in did not clear the call"
                    );
                }
            }
        }
    }
}

/// The widths at which the persistent pane is drawn, each with the width the
/// terminal was at the frame before: every width from the breakpoint up
/// through the cap breakpoint and on to the widest walked here, reached
/// cold, and every width inside the dead-band narrowed to from the pane.
fn pane_widths(h: u16) -> Vec<(u16, Option<u16>)> {
    let mut out: Vec<(u16, Option<u16>)> =
        (100u16..=160).chain([200, 240]).map(|w| (w, None)).collect();
    out.extend(dead_band_widths(h).into_iter().map(|w| (w, Some(100))));
    out
}

/// While the persistent pane is drawn at the press — at any width, the
/// dead-band's included — Space and i leave the inspector call exactly as it
/// stood, called for or not, and answer with one footer note (that the
/// inspector already stands beside the list), drawn where the hint bar
/// was; nothing else in the view changes, and nothing reaches the game. A
/// footer note already showing gives way to that one note.
///
/// @spec TUI-NAV-088
#[test]
fn while_the_pane_is_drawn_space_and_i_leave_the_call_and_leave_one_note() {
    const H: u16 = 30;
    let first = meadow_rows(&meadow())[0].clone();
    for (w, from) in pane_widths(H) {
        for wanted in [false, true] {
            for earlier_note in [false, true] {
                for (what, key) in inspector_keys() {
                    let mut a = session(Some((Section::Meadow, first.clone())), false, "pane-key");
                    a.view.inspector_wanted = wanted;
                    if let Some(from) = from {
                        keys(&mut a, &mut term(from, H), &[]);
                    }
                    let mut t = term(w, H);
                    keys(&mut a, &mut t, &[]);
                    assert!(
                        matches!(a.prev_layout.map(|l| l.body), Some(Body::Wide { .. })),
                        "fixture: {w}×{H} draws the persistent pane"
                    );
                    if earlier_note {
                        note(&mut a);
                    }
                    let before = a.view.clone();
                    let game = a.state.clone();

                    keys(&mut a, &mut t, std::slice::from_ref(&key));

                    assert_eq!(
                        a.view.inspector_wanted, wanted,
                        "{what} at {w} columns, the persistent pane on screen, changed the \
                         inspector call (was {wanted}); no swap is drawn for it to bring in or \
                         take out"
                    );
                    let (text, _) = a.view.footer.clone().unwrap_or_else(|| {
                        panic!(
                            "{what} at {w} columns, the persistent pane on screen, left no footer \
                             note; a bound key that cannot act says so"
                        )
                    });
                    assert!(!text.trim().is_empty(), "{what} at {w} columns left an empty note");
                    if earlier_note {
                        assert_ne!(
                            text, "a note already showing",
                            "{what} at {w} columns left the earlier note standing instead of \
                             answering with its own"
                        );
                    }
                    let mut expected = before.clone();
                    expected.footer = a.view.footer.clone();
                    assert_eq!(
                        view_of(&a),
                        format!("{expected:?}"),
                        "{what} at {w} columns, the persistent pane on screen, changed more than \
                         the footer note"
                    );
                    assert!(a.state == game, "{what} at {w} columns changed the game");

                    let hints = a.prev_layout.expect("a step drew a frame").hints;
                    let bar = region(t.backend().buffer(), hints).concat();
                    if text.chars().count() < w as usize {
                        assert!(
                            bar.contains(text.as_str()),
                            "{what} at {w} columns: the note {text:?} is not drawn where the \
                             hint bar was; the bar read {bar:?}"
                        );
                    }
                }
            }
        }
    }
}

/// While the too-small card is drawn at the press — below the floor by
/// width, by height, or both, whether the terminal came to it from the
/// narrow layout, from the pane, or opened on it — Space and i leave the
/// inspector call exactly as it stood, called for or not, and leave no note:
/// nothing in the view changes at all, a footer note already showing keeps
/// its own clock, and nothing reaches the game.
///
/// @spec TUI-NAV-088
#[test]
fn under_the_too_small_card_space_and_i_leave_the_call_and_leave_no_note() {
    let first = meadow_rows(&meadow())[0].clone();
    let cards: [(u16, u16); 9] =
        [(79, 24), (79, 40), (60, 20), (40, 30), (1, 1), (80, 23), (160, 23), (240, 10), (80, 1)];
    let approaches: [(&str, Option<(u16, u16)>); 3] =
        [("opened on it", None), ("from the narrow layout", Some((80, 24))), ("from the pane", Some((160, 40)))];
    for (w, h) in cards {
        for (how, from) in approaches {
            for wanted in [false, true] {
                for banner in [false, true] {
                    for earlier_note in [false, true] {
                        for (what, key) in inspector_keys() {
                            let mut a =
                                session(Some((Section::Meadow, first.clone())), banner, "card-key");
                            a.view.inspector_wanted = wanted;
                            if let Some((fw, fh)) = from {
                                keys(&mut a, &mut term(fw, fh), &[]);
                            }
                            let mut t = term(w, h);
                            keys(&mut a, &mut t, &[]);
                            assert!(
                                matches!(a.prev_layout.map(|l| l.body), Some(Body::TooSmall(_))),
                                "fixture: {w}×{h} draws the too-small card"
                            );
                            if earlier_note {
                                note(&mut a);
                            }
                            let before = view_of(&a);
                            let game = a.state.clone();

                            keys(&mut a, &mut t, std::slice::from_ref(&key));

                            assert_eq!(
                                a.view.inspector_wanted, wanted,
                                "{what} under the too-small card at {w}×{h} ({how}) changed the \
                                 inspector call (was {wanted})"
                            );
                            assert_eq!(
                                view_of(&a),
                                before,
                                "{what} under the too-small card at {w}×{h} ({how}, {}, {}) \
                                 changed the view; it leaves the call as it stood and leaves no \
                                 note",
                                if banner { "a banner queued" } else { "no banner" },
                                if earlier_note { "a note already showing" } else { "no note" }
                            );
                            assert!(a.state == game, "{what} under the card changed the game");
                        }
                    }
                }
            }
        }
    }
}
