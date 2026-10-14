//! Waking up — a load's catch-up replayed after the terminal is taken, a
//! bounded count per frame, behind the waking screen, until the game opens.
//!
//! @spec TUI-RUN-033, TUI-RUN-034, TUI-RUN-035, TUI-RUN-036, TUI-RUN-037,
//! TUI-RUN-038, TUI-RUN-039, TUI-NAV-102, TUI-NAV-103
//!
//! **The seam.** Start-up hands the core's pending catch-up (SAVE-018,
//! `clowder_core::begin_load`) to `runtime::Wake::begin`, which advances the
//! first bound before any frame. Where that finishes the catch-up the game
//! opens at once (`Wake::Awake(App)`, the opening taken and the load's report
//! queued); otherwise the first waking frame is drawn and a `Waking` comes
//! back (`Wake::Waking`). Each `Waking::step` is one waking frame — keys, the
//! clock reading, a bound replayed, the waking screen drawn — and returns the
//! next `Wake`. `Waking::shut_down` is shutdown taken while waking: it sets
//! the catch-up down and saves. Like `App::step`, a waking step takes its
//! events and its clock reading as arguments and draws into whatever backend
//! it is handed, so all of it runs headless here.
//!
//! The surface these tests address, which the code stage adds:
//!
//! ```text
//! pub const WAKE_TICKS_PER_FRAME: u64;           // waking's own bound
//! pub enum Wake { Waking(Waking), Awake(App) }
//! impl Wake {
//!     pub fn begin<B: Backend>(catch_up: CatchUp, save_path: PathBuf,
//!         terminal: &mut Terminal<B>) -> io::Result<Wake>;
//!     pub fn begin_bounded<B: Backend>(catch_up: CatchUp, save_path: PathBuf,
//!         terminal: &mut Terminal<B>, bound: u64) -> io::Result<Wake>;
//! }
//! pub struct Waking { pub clock: TickAccumulator, pub save_path: PathBuf,
//!                     pub last_save: Instant, pub quitting: bool, .. }
//! impl Waking {
//!     pub fn total_ticks(&self) -> u64;
//!     pub fn remaining_ticks(&self) -> u64;
//!     pub fn poll_timeout(&self) -> Duration;
//!     pub fn step<B: Backend>(self, terminal: &mut Terminal<B>,
//!         reading: Reading, input: &[Event]) -> io::Result<Wake>;
//!     pub fn shut_down(self) -> io::Result<()>;
//! }
//! impl App { pub fn poll_timeout(&self) -> Duration; }
//! ```
//!
//! `begin` is `begin_bounded` at `WAKE_TICKS_PER_FRAME`. Most tests here pass
//! a small bound so a wake of a few frames costs a few hundred ticks; the
//! bound's own value is pinned separately.
//!
//! **Reading a save back.** This crate has no JSON dependency, so a save the
//! shell wrote is decoded through the core alone: `begin_load(contents, 0)` is
//! a backwards clock, which owes no interval, so its total is exactly the
//! owed count the save carries, and setting it down at once hands back the
//! state, owed count, and report so far exactly as written (SAVE-020,
//! SAVE-021).

use std::path::PathBuf;
use std::time::{Duration, Instant};

use clowder_core::{
    begin_load, load_string, save_string, Advance, CatchUp, Command, GameState, JournalItem,
    LoadedGame, OfflineReport, ResourceId, SetDown, CATCHUP_CAP_SECONDS,
};
use clowder_tui::nav::{self, ViewState};
use clowder_tui::render;
use clowder_tui::runtime::clock::Reading;
use clowder_tui::runtime::{App, TickAccumulator, Wake, Waking, WAKE_TICKS_PER_FRAME};
use crossterm::event::{Event as TermEvent, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const SAVED_AT: u64 = 1_000_000;

fn scratch(name: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir =
        std::env::temp_dir().join(format!("clowder-waking-{}-{name}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("save.json")
}

/// A meadow with a little going on, at the given speed.
fn meadow(seed: u64, speed: f64) -> GameState {
    let mut s = GameState::new(seed);
    s.resources[ResourceId::Catnip.ix()] = 40.0;
    s.apply(Command::SetSpeed(speed)).expect("a speed on the ladder");
    s
}

/// A save written at `SAVED_AT`.
fn save_of(state: &GameState) -> String {
    save_string(state, SAVED_AT)
}

fn catch_up(json: &str, now: u64) -> CatchUp {
    begin_load(json, now).expect("the save loads")
}

fn term(w: u16, h: u16) -> Terminal<TestBackend> {
    Terminal::new(TestBackend::new(w, h)).expect("TestBackend is infallible")
}

fn still() -> Reading {
    Reading { monotonic: Duration::ZERO, wall: Duration::ZERO }
}

fn elapsed(secs: f64) -> Reading {
    let d = Duration::from_secs_f64(secs);
    Reading { monotonic: d, wall: d }
}

fn press(code: KeyCode, modifiers: KeyModifiers) -> TermEvent {
    TermEvent::Key(KeyEvent::new(code, modifiers))
}

fn begin(json: &str, now: u64, bound: u64, t: &mut Terminal<TestBackend>, name: &str) -> Wake {
    Wake::begin_bounded(catch_up(json, now), scratch(name), t, bound)
        .expect("a wake against a TestBackend does not fail")
}

fn waking(w: Wake) -> Waking {
    match w {
        Wake::Waking(w) => w,
        Wake::Awake(_) => panic!("the game opened, but the catch-up should still be replaying"),
    }
}

fn awake(w: Wake) -> App {
    match w {
        Wake::Awake(app) => app,
        Wake::Waking(w) => {
            panic!("still waking with {} of {} ticks to replay", w.remaining_ticks(), w.total_ticks())
        }
    }
}

fn step(w: Waking, t: &mut Terminal<TestBackend>, reading: Reading, input: &[TermEvent]) -> Wake {
    w.step(t, reading, input).expect("a waking step against a TestBackend does not fail")
}

fn state_bytes(s: &GameState) -> String {
    save_string(s, 0)
}

/// The view a load that never needed the screen opens with: the opening
/// taken against the caught-up state, then the load's report queued.
fn opened_view(loaded: &LoadedGame) -> ViewState {
    let mut view = ViewState::default();
    nav::opening(&mut view, &loaded.state);
    nav::queue_load_report(&mut view, loaded);
    view
}

fn view_of(v: &ViewState) -> String {
    format!("{v:?}")
}

fn set_down_after(json: &str, now: u64, replayed: u64) -> SetDown {
    let c = catch_up(json, now);
    if replayed == 0 {
        return c.set_down();
    }
    match c.advance(replayed) {
        Advance::Pending(c) => c.set_down(),
        Advance::Finished(_) => panic!("fixture: the reference finished before {replayed} ticks"),
    }
}

/// What a save the shell wrote carries: (state, owed count, report so far).
fn decode_save(path: &std::path::Path) -> SetDown {
    let contents = std::fs::read_to_string(path).expect("the shutdown wrote a save");
    begin_load(&contents, 0).expect("the shell's save loads").set_down()
}

fn away_entries(state: &GameState) -> usize {
    state.journal.iter_chronological().iter().filter(|e| matches!(e.item, JournalItem::Away(_))).count()
}

// ---------------------------------------------------------------------------
// Reading the screen
// ---------------------------------------------------------------------------

fn screen_rows(t: &Terminal<TestBackend>) -> Vec<String> {
    let buf = t.backend().buffer();
    let w = buf.area.width as usize;
    buf.content().chunks(w).map(|row| row.iter().map(|c| c.symbol()).collect()).collect()
}

fn blank(t: &Terminal<TestBackend>) -> bool {
    t.backend().buffer().content().iter().all(|c| c.symbol() == " ")
}

/// The waking screen as drawn: where its three lines sit, how many of the
/// bar's forty cells are filled, and the percent figure beside it.
#[derive(Debug)]
struct WakingScreen {
    calm: usize,
    bar: usize,
    hint: usize,
    filled: usize,
    percent: u64,
}

fn read_waking_screen(t: &Terminal<TestBackend>) -> WakingScreen {
    let rows = screen_rows(t);
    let drawn: Vec<(usize, &String)> =
        rows.iter().enumerate().filter(|(_, r)| !r.trim().is_empty()).collect();
    let dump = rows.join("\n");
    assert_eq!(
        drawn.len(),
        3,
        "the waking screen holds three lines and nothing of the game's frame:\n{dump}"
    );

    let is_cell = |c: char| c == '█' || c == '░';
    let bar_rows: Vec<&(usize, &String)> = drawn.iter().filter(|(_, r)| r.chars().any(is_cell)).collect();
    assert_eq!(bar_rows.len(), 1, "one bar:\n{dump}");
    let (bar, bar_text) = *bar_rows[0];
    let cells: Vec<char> = bar_text.chars().skip_while(|c| !is_cell(*c)).take_while(|c| is_cell(*c)).collect();
    assert_eq!(cells.len(), 40, "the bar is forty cells wide:\n{dump}");
    assert_eq!(
        bar_text.chars().filter(|c| is_cell(*c)).count(),
        40,
        "and is one unbroken run:\n{dump}"
    );
    let filled = cells.iter().filter(|c| **c == '█').count();
    assert!(
        cells.iter().take(filled).all(|c| *c == '█'),
        "the bar fills from its start: {}",
        cells.iter().collect::<String>()
    );
    let after: String = bar_text.chars().skip_while(|c| !is_cell(*c)).skip(40).collect();
    let pct_at = after.find('%').unwrap_or_else(|| panic!("a percent figure beside the bar:\n{dump}"));
    let digits: String = after[..pct_at].chars().rev().take_while(|c| c.is_ascii_digit()).collect();
    let digits: String = digits.chars().rev().collect();
    let percent: u64 = digits.parse().unwrap_or_else(|_| panic!("a whole-percent figure:\n{dump}"));

    let others: Vec<&(usize, &String)> = drawn.iter().filter(|(i, _)| *i != bar).collect();
    let (calm, calm_text) = *others[0];
    let (hint, hint_text) = *others[1];
    let calm_lower = calm_text.to_lowercase();
    assert!(
        calm_lower.contains("clowder") && (calm_lower.contains("waking") || calm_lower.contains("awake")),
        "the first line says the clowder is waking: {calm_text:?}"
    );
    let hint_lower = hint_text.to_lowercase();
    assert!(
        hint_lower.split_whitespace().any(|w| w == "q")
            && hint_lower.contains("save")
            && hint_lower.contains("quit")
            && hint_lower.contains("next time"),
        "the last line says q saves and quits with the rest left for next time: {hint_text:?}"
    );
    WakingScreen { calm, bar, hint, filled, percent }
}

fn expect_progress(t: &Terminal<TestBackend>, total: u64, remaining: u64, ctx: &str) {
    let screen = read_waking_screen(t);
    let replayed = total - remaining;
    assert_eq!(screen.filled as u64, 40 * replayed / total, "{ctx}: cells filled for {replayed}/{total}");
    assert_eq!(screen.percent, 100 * replayed / total, "{ctx}: percent for {replayed}/{total}");
    assert!(screen.calm < screen.bar && screen.bar < screen.hint, "{ctx}: line, bar, hint, in that order");
}

/// What the frame draws below the floor, whatever it is drawing for.
fn too_small_card(w: u16, h: u16) -> ratatui::buffer::Buffer {
    let mut t = term(w, h);
    t.draw(|f| {
        render::frame(f, &GameState::new(1), &ViewState::default(), None);
    })
    .unwrap();
    t.backend().buffer().clone()
}

// ---------------------------------------------------------------------------
// TUI-RUN-033 — replayed after the terminal is taken, a bounded count a frame
// ---------------------------------------------------------------------------

/// The first frame shows the meadow having replayed no more than one bound,
/// and every frame after it replays no more than one more.
/// @spec TUI-RUN-033
#[test]
fn the_first_waking_frame_has_replayed_at_most_one_bound_and_each_frame_one_more() {
    let json = save_of(&meadow(33_001, 1.0));
    let mut t = term(100, 30);
    let bound = 100;
    let mut w = waking(begin(&json, SAVED_AT + 300, bound, &mut t, "first-frame"));
    let total = w.total_ticks();
    assert_eq!(total, 1_200, "fixture: five minutes away at speed 1");
    assert!(total - w.remaining_ticks() <= bound, "the first frame replayed more than one bound");
    expect_progress(&t, total, w.remaining_ticks(), "first frame");

    for frame in 1.. {
        let before = w.remaining_ticks();
        match step(w, &mut t, still(), &[]) {
            Wake::Waking(next) => {
                assert!(before - next.remaining_ticks() <= bound, "frame {frame} replayed more than a bound");
                w = next;
            }
            Wake::Awake(_) => {
                assert!(before <= bound, "the last frame replayed more than a bound");
                break;
            }
        }
        assert!(frame < 1_000, "the wake never finished");
    }
}

/// Waking's bound is a count of its own, larger than live play's per-frame
/// drain: the waking screen has only its progress and the quit key to keep
/// responsive. The production entry point replays no more than that count
/// before its first frame.
/// @spec TUI-RUN-033
#[test]
fn waking_replays_by_its_own_count_and_not_live_plays() {
    // Live play's per-frame drain, measured: a day's interval in one frame
    // runs exactly the live bound.
    let mut live = App {
        state: GameState::new(33_002),
        view: ViewState::default(),
        clock: TickAccumulator::default(),
        save_path: scratch("live-bound"),
        last_save: Instant::now(),
        prev_layout: None,
        quitting: false,
        save_failed: false,
    };
    let before = live.state.ticks;
    live.step(&mut term(100, 30), elapsed(86_400.0), &[]).unwrap();
    let live_bound = live.state.ticks - before;
    assert!(live_bound > 0 && live_bound < 86_400 * 4, "fixture: live play drained a bounded count");
    assert!(
        WAKE_TICKS_PER_FRAME > live_bound,
        "waking's bound ({WAKE_TICKS_PER_FRAME}) is its own count, larger than live play's ({live_bound})"
    );

    let away = WAKE_TICKS_PER_FRAME / 4 + 60;
    let json = save_of(&meadow(33_003, 1.0));
    let mut t = term(100, 30);
    let w = Wake::begin(catch_up(&json, SAVED_AT + away), scratch("own-bound"), &mut t).unwrap();
    let w = waking(w);
    assert!(
        w.total_ticks() - w.remaining_ticks() <= WAKE_TICKS_PER_FRAME,
        "the first frame replayed more than one of waking's bounds"
    );
    expect_progress(&t, w.total_ticks(), w.remaining_ticks(), "first frame at the production bound");
}

// ---------------------------------------------------------------------------
// TUI-RUN-034 — the screen only when the first bound does not finish it
// ---------------------------------------------------------------------------

/// No absence, a backwards clock, and an absence the first bound replays
/// whole all open the game directly: no waking screen is ever drawn.
/// @spec TUI-RUN-034
#[test]
fn a_catch_up_the_first_bound_finishes_opens_the_game_with_no_waking_screen() {
    let json = save_of(&meadow(34_001, 1.0));
    for (now, why) in [
        (SAVED_AT, "no absence"),
        (SAVED_AT - 5_000, "a backwards clock"),
        (SAVED_AT + 100, "a short absence"),
    ] {
        let mut t = term(100, 30);
        let app = awake(Wake::begin(catch_up(&json, now), scratch("direct"), &mut t).unwrap());
        assert!(blank(&t), "{why}: the game opened directly, so nothing was drawn before play");
        assert_eq!(state_bytes(&app.state), state_bytes(&load_string(&json, now).unwrap().state), "{why}");
    }

    // The boundary, exactly: a bound equal to the whole catch-up finishes it
    // on the first advance; one tick short does not.
    let mut t = term(100, 30);
    let total = catch_up(&json, SAVED_AT + 300).total_ticks();
    let _ = awake(begin(&json, SAVED_AT + 300, total, &mut t, "exact"));
    assert!(blank(&t), "a first bound that finishes the catch-up draws no waking screen");
    let mut t = term(100, 30);
    let w = waking(begin(&json, SAVED_AT + 300, total - 1, &mut t, "short-by-one"));
    assert_eq!(w.remaining_ticks(), 1);
    expect_progress(&t, total, 1, "one tick short");
}

/// Otherwise every frame until the catch-up finishes is the waking screen.
/// @spec TUI-RUN-034
#[test]
fn a_longer_catch_up_draws_the_waking_screen_on_every_frame_until_it_finishes() {
    let json = save_of(&meadow(34_002, 1.0));
    let mut t = term(100, 30);
    let mut w = waking(begin(&json, SAVED_AT + 200, 75, &mut t, "every-frame"));
    let total = w.total_ticks();
    expect_progress(&t, total, w.remaining_ticks(), "frame 0");
    let mut frames = 0;
    while let Wake::Waking(next) = step(w, &mut t, elapsed(0.06), &[]) {
        frames += 1;
        expect_progress(&t, total, next.remaining_ticks(), &format!("frame {frames}"));
        w = next;
    }
    assert!(frames >= 9, "fixture: 800 ticks at 75 a frame is a wake of many frames");
}

// ---------------------------------------------------------------------------
// TUI-RUN-035 — no waiting while waking
// ---------------------------------------------------------------------------

/// The poll for input does not wait out the frame budget while waking; once
/// awake, it does. What a waking frame asks for instead is the briefest wait
/// — longer than none, because the input source reads nothing in a wait of
/// no time, and no longer than a millisecond, which is nothing beside the
/// time a bound takes to replay.
/// @spec TUI-RUN-035, TUI-RUN-059
#[test]
fn waking_polls_without_waiting_and_play_waits_out_the_frame_budget() {
    fn expect_the_briefest_wait(wait: Duration, which: &str) {
        assert!(
            wait > Duration::ZERO,
            "{which} asks its input source to wait no time at all, and the source reads nothing \
             in such a wait — a waking game would never hear its quit key"
        );
        assert!(
            wait <= Duration::from_millis(1),
            "{which} waits {wait:?} for input: a waking frame checks for a key and moves on"
        );
    }

    let json = save_of(&meadow(35_001, 1.0));
    let mut t = term(100, 30);
    let mut w = waking(begin(&json, SAVED_AT + 100, 50, &mut t, "poll"));
    let first = w.poll_timeout();
    expect_the_briefest_wait(first, "the first waking frame");
    let app = loop {
        match step(w, &mut t, still(), &[]) {
            Wake::Waking(next) => {
                expect_the_briefest_wait(next.poll_timeout(), "a waking frame");
                w = next;
            }
            Wake::Awake(app) => break app,
        }
    };
    assert!(
        app.poll_timeout() > Duration::from_millis(1) && app.poll_timeout() > first,
        "the awake game waits out its frame budget, which is longer than waking's briefest wait"
    );
}

// ---------------------------------------------------------------------------
// TUI-RUN-036 — no live tick while waking; the time spent waking is banked
// ---------------------------------------------------------------------------

/// While waking no live tick runs, and the real time passing is banked at the
/// save's speed through the ordinary accumulator; the first frame of play
/// runs it, after every replayed tick.
/// @spec TUI-RUN-036
#[test]
fn time_spent_waking_is_banked_at_the_saves_speed_and_runs_after_the_replay() {
    let speed = 2.0;
    let json = save_of(&meadow(36_001, speed));
    let now = SAVED_AT + 100; // 800 ticks at speed 2
    let mut t = term(100, 30);
    let mut w = waking(begin(&json, now, 50, &mut t, "banked"));
    let mut reference = TickAccumulator::default();
    let frame = elapsed(1.5);
    let app = loop {
        reference.observe(frame, speed);
        // A speed key pressed while waking is discarded (TUI-NAV-103), so it
        // cannot re-price the time spent waking either.
        match step(w, &mut t, frame, &[press(KeyCode::Char('<'), KeyModifiers::NONE)]) {
            Wake::Waking(next) => {
                assert_eq!(next.clock, reference, "each waking frame's time is banked at the save's speed");
                w = next;
            }
            Wake::Awake(app) => break app,
        }
    };
    assert_eq!(app.clock, reference, "the opened game holds every interval waking observed");
    let one_call = load_string(&json, now).unwrap();
    assert_eq!(
        state_bytes(&app.state),
        state_bytes(&one_call.state),
        "no live tick ran while waking: the meadow is exactly the replay's"
    );

    let banked = app.clock.backlog();
    assert!(banked > 0 && banked < 2_000, "fixture: a small banked backlog, drained in one frame");
    let mut app = app;
    app.step(&mut t, still(), &[]).unwrap();
    let mut expected = one_call.state.clone();
    for _ in 0..banked {
        expected.tick();
    }
    assert_eq!(
        state_bytes(&app.state),
        state_bytes(&expected),
        "the first frame of play runs the banked ticks as live ticks, after every replayed tick"
    );
}

/// The ordinary accumulator's rules hold while waking: suspend excess is
/// absence, and an interval is capped by the one absence rule.
/// @spec TUI-RUN-036
#[test]
fn waking_observes_suspend_excess_and_the_absence_cap() {
    let speed = 0.5;
    let json = save_of(&meadow(36_002, speed));
    for (reading, why) in [
        (Reading { monotonic: Duration::from_millis(500), wall: Duration::from_secs(3_600) }, "suspend excess"),
        (
            Reading { monotonic: Duration::ZERO, wall: Duration::from_secs(3 * CATCHUP_CAP_SECONDS) },
            "the absence cap",
        ),
    ] {
        let mut t = term(100, 30);
        let w = waking(begin(&json, SAVED_AT + 300, 10, &mut t, "suspend"));
        let mut reference = TickAccumulator::default();
        reference.observe(reading, speed);
        let w = waking(step(w, &mut t, reading, &[]));
        assert_eq!(w.clock, reference, "{why}: observed as the accumulator observes it, at the save's speed");
        assert!(w.clock.backlog() > 0, "{why}: and banked, not dropped");
    }
}

// ---------------------------------------------------------------------------
// TUI-RUN-037 — no autosave while waking
// ---------------------------------------------------------------------------

/// The save on disk is left owing everything the wake has not finished, on
/// every waking frame, however overdue the cadence. Once the game is awake
/// the cadence, which kept counting, saves on the first frame of play.
/// @spec TUI-RUN-037
#[test]
fn no_autosave_is_taken_while_waking() {
    let json = save_of(&meadow(37_001, 1.0));
    let path = scratch("no-autosave");
    std::fs::write(&path, &json).unwrap();
    let mut t = term(100, 30);
    let w = Wake::begin_bounded(catch_up(&json, SAVED_AT + 200), path.clone(), &mut t, 60).unwrap();
    let mut w = waking(w);
    let overdue = Instant::now() - Duration::from_secs(24 * 3_600);
    w.last_save = overdue;
    let mut app = loop {
        match step(w, &mut t, elapsed(1.0), &[]) {
            Wake::Waking(next) => {
                assert_eq!(
                    std::fs::read_to_string(&path).unwrap(),
                    json,
                    "a waking frame wrote the save; a kill now would lose the replay's place"
                );
                w = next;
            }
            Wake::Awake(app) => break app,
        }
    };
    assert_eq!(std::fs::read_to_string(&path).unwrap(), json, "nor did the frame that finished it");
    app.step(&mut t, still(), &[]).unwrap();
    assert_ne!(
        std::fs::read_to_string(&path).unwrap(),
        json,
        "the cadence kept counting while waking, so the first frame of play saves"
    );
}

// ---------------------------------------------------------------------------
// TUI-RUN-038 — the frame that finishes opens the game
// ---------------------------------------------------------------------------

/// The finishing frame runs no live tick and opens the game against exactly
/// the state a one-call load of the same save at the same moment produces:
/// the opening taken and the report queued as for a load that never woke.
/// @spec TUI-RUN-038, TUI-NAV-024
#[test]
fn the_frame_that_finishes_opens_the_game_against_the_one_call_state() {
    let json = save_of(&meadow(38_001, 1.0));
    for (away, bound) in [(300u64, 70u64), (90, 1_000), (300, 1_199)] {
        let now = SAVED_AT + away;
        let mut t = term(100, 30);
        let mut w = begin(&json, now, bound, &mut t, "finish");
        let app = loop {
            match w {
                Wake::Awake(app) => break app,
                Wake::Waking(ww) => w = step(ww, &mut t, elapsed(0.5), &[]),
            }
        };
        let one_call = load_string(&json, now).unwrap();
        assert_eq!(
            state_bytes(&app.state),
            state_bytes(&one_call.state),
            "away {away}s, bound {bound}: the opening sees the one-call state, bit for bit"
        );
        assert_eq!(
            view_of(&app.view),
            view_of(&opened_view(&one_call)),
            "away {away}s, bound {bound}: the opening taken and the report queued against it"
        );
        assert_eq!(app.view.reports.len(), 1, "away {away}s: the absence's report is queued");
        assert!(!app.quitting);
    }
}

// ---------------------------------------------------------------------------
// TUI-RUN-039 — shutdown while waking sets the catch-up down and saves
// ---------------------------------------------------------------------------

/// A quit key or a termination signal mid-wake saves promptly: the state so
/// far, owing the catch-up's remaining ticks plus the live ticks banked while
/// waking, with the core's report so far beside them.
/// @spec TUI-RUN-039, TUI-RUN-026
#[test]
fn shutdown_while_waking_saves_the_set_down_catch_up_and_the_banked_time() {
    let json = save_of(&meadow(39_001, 1.0));
    let now = SAVED_AT + 600; // 2400 ticks
    let quits: [(&str, Option<TermEvent>); 3] = [
        ("q", Some(press(KeyCode::Char('q'), KeyModifiers::NONE))),
        ("Ctrl-C", Some(press(KeyCode::Char('c'), KeyModifiers::CONTROL))),
        ("a termination signal", None),
    ];
    for (how, key) in quits {
        let path = scratch(&format!("quit-{}", how.len()));
        let mut t = term(100, 30);
        let w = Wake::begin_bounded(catch_up(&json, now), path.clone(), &mut t, 100).unwrap();
        let mut w = waking(w);
        for _ in 0..3 {
            w = waking(step(w, &mut t, elapsed(2.0), &[]));
        }
        if let Some(key) = key {
            w = waking(step(w, &mut t, elapsed(2.0), &[key]));
            assert!(w.quitting, "{how}: answered by quitting");
        }
        let replayed = w.total_ticks() - w.remaining_ticks();
        let remaining = w.remaining_ticks();
        let banked = w.clock.backlog();
        assert!(remaining > 0, "fixture: shutdown came mid-wake");
        assert!(banked > 0, "fixture: some live time was banked while waking");
        w.shut_down().expect("the shutdown save is written");

        let saved = decode_save(&path);
        let reference = set_down_after(&json, now, replayed);
        assert_eq!(saved.owed_ticks, remaining + banked, "{how}: the rest of the catch-up plus the banked time");
        assert_eq!(
            state_bytes(&saved.state),
            state_bytes(&reference.state),
            "{how}: the state so far, not finished first"
        );
        assert_eq!(saved.report_so_far, reference.report_so_far, "{how}: the core's report so far");
    }
}

/// The save a mid-wake shutdown leaves wakes through the rest at the next
/// load and tells the absence once.
/// @spec TUI-RUN-039
#[test]
fn a_wake_quit_midway_tells_the_whole_absence_once_at_the_next_load() {
    let json = save_of(&meadow(39_002, 1.0));
    let now = SAVED_AT + 300;
    let path = scratch("tell-once");
    let mut t = term(100, 30);
    let mut w = waking(Wake::begin_bounded(catch_up(&json, now), path.clone(), &mut t, 200).unwrap());
    w = waking(step(w, &mut t, still(), &[]));
    w = waking(step(w, &mut t, still(), &[press(KeyCode::Char('q'), KeyModifiers::NONE)]));
    assert!(w.quitting);
    w.shut_down().unwrap();

    let contents = std::fs::read_to_string(&path).unwrap();
    let saved = decode_save(&path);
    assert_eq!(away_entries(&saved.state), away_entries(&meadow(39_002, 1.0)), "nothing recorded at the quit");

    let next = begin_load(&contents, 0).unwrap();
    let finished = match next.advance(u64::MAX) {
        Advance::Finished(g) => g,
        Advance::Pending(_) => panic!("an unbounded advance finishes"),
    };
    let whole = load_string(&json, now).unwrap();
    let r: &OfflineReport = &finished.report;
    assert_eq!(r.elapsed_seconds, whole.report.elapsed_seconds, "the whole absence's span");
    assert_eq!(r.truncated, whole.report.truncated);
    assert_eq!(r.arrived, whole.report.arrived);
    assert_eq!(r.wandered, whole.report.wandered);
    assert_eq!(r.returned, whole.report.returned);
    assert_eq!(r.seasons_passed, whole.report.seasons_passed);
    for res in ResourceId::ALL {
        let (a, b) = (r.resource_deltas[res.ix()], whole.report.resource_deltas[res.ix()]);
        assert!((a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0), "{res:?}: {a} vs {b}");
    }
    assert_eq!(finished.state.ticks, whole.state.ticks, "not one owed tick lost or doubled");
    assert_eq!(
        away_entries(&finished.state),
        away_entries(&whole.state),
        "one absence, one entry"
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-102 — the waking screen
// ---------------------------------------------------------------------------

/// Three lines in place of the game's frame — at the floor and above it.
/// @spec TUI-NAV-102
#[test]
fn the_waking_screen_is_three_lines_in_place_of_the_games_frame() {
    let json = save_of(&meadow(102_001, 1.0));
    for (w, h) in [(80u16, 24u16), (100, 30), (160, 50)] {
        let mut t = term(w, h);
        let wk = waking(begin(&json, SAVED_AT + 300, 100, &mut t, "three-lines"));
        expect_progress(&t, wk.total_ticks(), wk.remaining_ticks(), &format!("{w}x{h}"));
    }
}

/// Bar and figure are the share of owed ticks replayed, both rounded down —
/// on every frame of a wake whose shares never fall on whole numbers, and at
/// both ends, where rounding down keeps a nearly-done wake from reading full.
/// @spec TUI-NAV-102
#[test]
fn the_bar_and_figure_are_the_share_replayed_rounded_down_on_every_frame() {
    let json = save_of(&meadow(102_002, 0.25));
    let now = SAVED_AT + 97; // 97 ticks at speed 0.25
    let mut t = term(100, 30);
    let mut w = waking(begin(&json, now, 10, &mut t, "every-share"));
    assert_eq!(w.total_ticks(), 97, "fixture");
    expect_progress(&t, 97, w.remaining_ticks(), "frame 0");
    while let Wake::Waking(next) = step(w, &mut t, still(), &[]) {
        let screen = read_waking_screen(&t);
        assert!(screen.filled < 40 && screen.percent < 100, "never full while waking");
        expect_progress(&t, 97, next.remaining_ticks(), "a later frame");
        w = next;
    }

    let json = save_of(&meadow(102_003, 1.0));
    let now = SAVED_AT + 250; // 1000 ticks
    let mut t = term(100, 30);
    let w = waking(begin(&json, now, 999, &mut t, "nearly"));
    let screen = read_waking_screen(&t);
    assert_eq!((screen.filled, screen.percent), (39, 99), "999 of 1000 reads 39 cells and 99%");
    drop(w);
    let mut t = term(100, 30);
    let _ = waking(begin(&json, now, 1, &mut t, "barely"));
    let screen = read_waking_screen(&t);
    assert_eq!((screen.filled, screen.percent), (0, 0), "1 of 1000 reads empty and 0%");
}

/// The screen is drawn from the catch-up's counts and the terminal size
/// alone: two different meadows owing the same ticks draw the same screen.
/// @spec TUI-NAV-102
#[test]
fn the_waking_screen_is_drawn_from_the_counts_and_the_size_alone() {
    let mut a_state = meadow(102_004, 1.0);
    a_state.resources[ResourceId::Catnip.ix()] = 5.0;
    let b_state = meadow(102_005, 2.0);
    let a_json = save_of(&a_state);
    let b_json = save_of(&b_state);
    let mut ta = term(110, 32);
    let mut tb = term(110, 32);
    let mut a = waking(begin(&a_json, SAVED_AT + 200, 90, &mut ta, "alone-a")); // 800 ticks
    let mut b = waking(begin(&b_json, SAVED_AT + 100, 90, &mut tb, "alone-b")); // 800 ticks
    assert_eq!(a.total_ticks(), b.total_ticks(), "fixture: the same owed ticks");
    for frame in 0..5 {
        assert_eq!(a.remaining_ticks(), b.remaining_ticks());
        assert_eq!(ta.backend().buffer(), tb.backend().buffer(), "frame {frame}: the same screen");
        a = waking(step(a, &mut ta, elapsed(0.06), &[]));
        b = waking(step(b, &mut tb, elapsed(0.06), &[]));
    }
}

/// Below 80 columns or 24 rows the too-small card stands in, and the replay
/// carries on beneath it; back at size, the waking screen returns.
/// @spec TUI-NAV-102
#[test]
fn below_the_floor_the_too_small_card_draws_and_the_replay_continues() {
    let json = save_of(&meadow(102_006, 1.0));
    for (w, h) in [(79u16, 30u16), (100, 23), (40, 10)] {
        let mut t = term(w, h);
        let mut wk = waking(begin(&json, SAVED_AT + 300, 100, &mut t, "too-small"));
        assert_eq!(*t.backend().buffer(), too_small_card(w, h), "{w}x{h}: the too-small card");
        for _ in 0..3 {
            let before = wk.remaining_ticks();
            wk = waking(step(wk, &mut t, elapsed(0.06), &[]));
            assert!(wk.remaining_ticks() < before, "{w}x{h}: the replay carries on under the card");
            assert_eq!(*t.backend().buffer(), too_small_card(w, h), "{w}x{h}: still the card");
        }
        t.backend_mut().resize(100, 30);
        let wk = waking(step(wk, &mut t, elapsed(0.06), &[TermEvent::Resize(100, 30)]));
        expect_progress(&t, wk.total_ticks(), wk.remaining_ticks(), "back at size");
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-103 — keys while waking
// ---------------------------------------------------------------------------

/// `q` and Ctrl-C each save and quit from the waking screen.
/// @spec TUI-NAV-103
#[test]
fn q_and_ctrl_c_save_and_quit_while_waking() {
    let json = save_of(&meadow(103_001, 1.0));
    for (i, key) in [
        press(KeyCode::Char('q'), KeyModifiers::NONE),
        press(KeyCode::Char('c'), KeyModifiers::CONTROL),
    ]
    .into_iter()
    .enumerate()
    {
        let path = scratch(&format!("nav-quit-{i}"));
        let mut t = term(100, 30);
        let w = waking(Wake::begin_bounded(catch_up(&json, SAVED_AT + 300), path.clone(), &mut t, 50).unwrap());
        let w = waking(step(w, &mut t, still(), std::slice::from_ref(&key)));
        assert!(w.quitting, "{key:?} quits while waking");
        assert!(!path.exists(), "fixture: nothing saved before shutdown");
        let remaining = w.remaining_ticks();
        w.shut_down().unwrap();
        assert_eq!(decode_save(&path).owed_ticks, remaining, "{key:?}: saved, owing the rest of the wake");
    }
}

/// Every key but the quit keys, quantified: printable characters (Shift
/// included), the named keys, function keys, Ctrl chords, and Alt chords —
/// among them the `Esc`-fused `q` tmux's escape-time produces.
fn every_other_key() -> Vec<KeyEvent> {
    let mut keys = Vec::new();
    for c in ' '..='~' {
        if c == 'q' {
            continue;
        }
        let m = if c.is_ascii_uppercase() { KeyModifiers::SHIFT } else { KeyModifiers::NONE };
        keys.push(KeyEvent::new(KeyCode::Char(c), m));
    }
    for code in [
        KeyCode::Enter,
        KeyCode::Esc,
        KeyCode::Tab,
        KeyCode::Backspace,
        KeyCode::Up,
        KeyCode::Down,
        KeyCode::Left,
        KeyCode::Right,
        KeyCode::Home,
        KeyCode::End,
        KeyCode::PageUp,
        KeyCode::PageDown,
        KeyCode::Delete,
        KeyCode::Insert,
    ] {
        keys.push(KeyEvent::new(code, KeyModifiers::NONE));
    }
    keys.push(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT));
    for n in 1..=12 {
        keys.push(KeyEvent::new(KeyCode::F(n), KeyModifiers::NONE));
    }
    for c in 'a'..='z' {
        if c != 'c' {
            keys.push(KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL));
        }
    }
    for c in ' '..='~' {
        keys.push(KeyEvent::new(KeyCode::Char(c), KeyModifiers::ALT));
    }
    keys
}

/// Every other key is discarded while waking — none quits, none acts, and
/// none is held for the game once it opens: the game opens exactly as a load
/// that never woke would, and its first frame of play acts on nothing.
/// @spec TUI-NAV-103
#[test]
fn every_other_key_is_discarded_while_waking_and_none_is_held_for_the_game() {
    let json = save_of(&meadow(103_002, 1.0));
    let now = SAVED_AT + 120; // 480 ticks, one a frame
    let keys = every_other_key();
    let mut t = term(100, 30);
    let mut w = waking(begin(&json, now, 1, &mut t, "discard"));
    assert!(w.total_ticks() as usize > keys.len() + 1, "fixture: a frame for every key");
    for key in &keys {
        w = waking(step(w, &mut t, still(), &[TermEvent::Key(*key)]));
        assert!(!w.quitting, "{key:?} is not a quit key");
    }
    let mut app = loop {
        match step(w, &mut t, still(), &[]) {
            Wake::Waking(next) => w = next,
            Wake::Awake(app) => break app,
        }
    };
    assert!(!app.quitting);

    let one_call = load_string(&json, now).unwrap();
    assert_eq!(state_bytes(&app.state), state_bytes(&one_call.state), "no key acted on the meadow");
    assert_eq!(view_of(&app.view), view_of(&opened_view(&one_call)), "no key moved the view");

    let mut reference = App {
        state: one_call.state.clone(),
        view: opened_view(&one_call),
        clock: TickAccumulator::default(),
        save_path: scratch("discard-reference"),
        last_save: Instant::now(),
        prev_layout: None,
        quitting: false,
        save_failed: false,
    };
    let mut rt = term(100, 30);
    app.step(&mut t, still(), &[]).unwrap();
    reference.step(&mut rt, still(), &[]).unwrap();
    assert_eq!(state_bytes(&app.state), state_bytes(&reference.state), "no key was held for play");
    assert_eq!(view_of(&app.view), view_of(&reference.view), "the first frame of play acted on nothing");
    assert_eq!(t.backend().buffer(), rt.backend().buffer(), "and draws as a load that never woke");
}
