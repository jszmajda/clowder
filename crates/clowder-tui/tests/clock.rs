//! The time model: two clocks, one simulation.
//!
//! Every test here injects a `Reading` rather than reading a clock, which is
//! the whole point of the component's shape — suspend detection, a backwards
//! wall clock, and the shared absence cap are exactly the behaviours that need
//! to be exercised at arbitrary instants, headlessly.
//!
//! `clowder-tui` is a binary with no library target, so the module under test
//! is included by path. That keeps the tests pointed at the real
//! implementation file rather than a copy; see the openQuestions note about
//! giving the crate a `[lib]` target instead.

use clowder_tui::runtime::clock;

use std::time::Duration;

use clock::{Reading, TickAccumulator};
use clowder_core::content::TICKS_PER_SECOND;
use clowder_core::{load_string, save_string, GameState, CATCHUP_CAP_SECONDS};

const TPS: u64 = TICKS_PER_SECOND;
const TPS_F: f64 = TPS as f64;

/// Both clocks in step — an ordinary attended frame.
fn steady(d: Duration) -> Reading {
    Reading { monotonic: d, wall: d }
}

fn reading(monotonic: Duration, wall: Duration) -> Reading {
    Reading { monotonic, wall }
}

fn assert_span(actual: Duration, expected_secs: f64, what: &str) {
    let got = actual.as_secs_f64();
    assert!(
        (got - expected_secs).abs() < 1e-3,
        "{what}: unrun span was {got}s, expected {expected_secs}s"
    );
}

/// The fixtures whose expected tick sequences were worked out by hand assume
/// the core's current rate. If the core changes it, recompute them rather than
/// loosening the assertions.
fn require_known_rate() {
    assert_eq!(TPS, 4, "hand-computed fixtures assume the core's ticks-per-second");
}

// ---------------------------------------------------------------------------
// TUI-RUN-018 — elapsed × ticks-per-second × speed, remainder banked
// ---------------------------------------------------------------------------

/// @spec TUI-RUN-018
#[test]
fn elapsed_converts_at_ticks_per_second_times_speed() {
    let mut acc = TickAccumulator::default();

    acc.observe(steady(Duration::from_secs(1)), 1.0);
    assert_eq!(acc.backlog(), TPS, "one second at normal speed is one second of ticks");

    acc.observe(steady(Duration::from_secs(3)), 2.0);
    assert_eq!(
        acc.backlog(),
        TPS + 3 * TPS * 2,
        "three seconds at double speed owes twice the ticks that interval would owe at normal speed"
    );

    // The banked span is real time, not tick-time: 4 s of clock elapsed.
    assert_span(acc.unrun_span(), 4.0, "four real seconds were observed");
}

/// The sub-tick remainder is what makes a frame budget honest: a frame far
/// shorter than a tick must still contribute its time, and contribute it
/// exactly once.
/// @spec TUI-RUN-018
#[test]
fn sub_tick_time_is_banked_across_frames_and_neither_lost_nor_double_counted() {
    require_known_rate();
    let mut acc = TickAccumulator::default();

    // 110 ms per frame at normal speed is 0.44 ticks — well clear of every
    // whole-tick boundary, so the expected sequence is unambiguous.
    // Cumulative ticks owed after k frames: floor(0.44 * k).
    let expected = [0u64, 0, 1, 1, 2, 2, 3, 3, 3, 4];
    for (k, want) in expected.iter().enumerate() {
        acc.observe(steady(Duration::from_millis(110)), 1.0);
        assert_eq!(
            acc.backlog(),
            *want,
            "after {} frames of 110 ms the whole-tick backlog should be {want}",
            k + 1
        );
    }

    // 1.1 s observed, 4 whole ticks converted (1.0 s worth); the remaining
    // 0.1 s is still banked as time, not yet as a tick, and so is not part of
    // the unrun backlog's span.
    assert_span(acc.unrun_span(), 1.0, "only converted whole ticks carry a span");
}

// ---------------------------------------------------------------------------
// TUI-RUN-019 — integer tick accounting
// ---------------------------------------------------------------------------

/// @spec TUI-RUN-019
#[test]
fn a_large_backlog_is_counted_exactly() {
    let mut acc = TickAccumulator::default();

    // A hundred days of absence, well inside the cap.
    let hundred_days = 100 * 86_400;
    acc.observe(steady(Duration::from_secs(hundred_days)), 1.0);
    assert_eq!(
        acc.backlog(),
        hundred_days * TPS,
        "a huge backlog is an exact integer count, not a rounded float"
    );

    // One more tick's worth of time adds exactly one tick, at that magnitude.
    acc.observe(steady(Duration::from_millis(1000 / TPS)), 1.0);
    assert_eq!(acc.backlog(), hundred_days * TPS + 1, "no whole tick is lost at scale");

    let taken = acc.take(u64::MAX);
    assert_eq!(taken, hundred_days * TPS + 1, "the drained count matches the owed count exactly");
    assert_eq!(acc.backlog(), 0);
}

// ---------------------------------------------------------------------------
// TUI-RUN-020 / TUI-RUN-030 — what a speed change may and may not touch
// ---------------------------------------------------------------------------

/// Backlog ticks were converted at the speed in force when their time elapsed.
/// Re-converting them at a new speed would rewrite history the player already
/// lived through.
/// @spec TUI-RUN-020, TUI-RUN-030
#[test]
fn a_speed_change_never_rescales_the_whole_tick_backlog() {
    let mut acc = TickAccumulator::default();

    acc.observe(steady(Duration::from_secs(4)), 1.0);
    assert_eq!(acc.backlog(), 4 * TPS);

    acc.observe(steady(Duration::from_secs(4)), 2.0);
    assert_eq!(
        acc.backlog(),
        4 * TPS + 4 * TPS * 2,
        "the new speed governs only the interval that elapsed after the change"
    );
    assert_ne!(
        acc.backlog(),
        8 * TPS * 2,
        "the first four seconds must not be re-converted at the new speed"
    );

    assert_span(acc.unrun_span(), 8.0, "the banked span is real time and is speed-agnostic");
}

/// The two banked quantities are distinct: time not yet worth a whole tick is
/// not backlog, and carries no unrun span, until it converts.
/// @spec TUI-RUN-030
#[test]
fn the_sub_tick_remainder_is_not_the_whole_tick_backlog() {
    require_known_rate();
    let mut acc = TickAccumulator::default();

    acc.observe(steady(Duration::from_millis(200)), 1.0);
    assert_eq!(acc.backlog(), 0, "0.8 of a tick is not a tick");
    assert_span(acc.unrun_span(), 0.0, "unconverted time is not an unrun backlog");

    acc.observe(steady(Duration::from_millis(100)), 1.0);
    assert_eq!(acc.backlog(), 1, "the banked remainder completes the tick");
    assert_span(
        acc.unrun_span(),
        0.25,
        "the span covers the converted tick only; the leftover 0.05 s is still awaiting conversion",
    );
}

/// Nothing banked is ever re-rated.
///
/// This test previously pinned the opposite reading, when TUI-RUN-030 and
/// TUI-RUN-020 contradicted each other: 030 said a speed change applied to the
/// sub-tick remainder, 020 said banked time is never rescaled. 020 won — it
/// follows from idle-respect, and it is the rule that stays simple enough to
/// keep. Every interval now converts to ticks at the speed in force when *it*
/// elapsed, so the bank holds fractions of a tick rather than raw seconds and
/// a later speed change cannot reach back and re-price it.
///
/// 0.2 s at 1.0× is 0.8 of a tick; 0.2 s at 4.0× is 3.2 more. 4.0 total → 4
/// whole ticks. Banking seconds instead would have given 6, by re-pricing the
/// first interval at the second interval's speed.
/// @spec TUI-RUN-020, TUI-RUN-030
#[test]
fn banked_time_is_never_re_rated_by_a_later_speed_change() {
    require_known_rate();
    let mut acc = TickAccumulator::default();

    acc.observe(steady(Duration::from_millis(200)), 1.0);
    assert_eq!(acc.backlog(), 0, "0.8 of a tick — nothing whole yet");

    acc.observe(steady(Duration::from_millis(200)), 4.0);
    assert_eq!(
        acc.backlog(),
        4,
        "0.8 earned at 1x plus 3.2 earned at 4x; the first interval keeps its own rate"
    );
    // Four ticks, all attributed to the 4x conversion that emitted them.
    assert_span(acc.unrun_span(), 0.25, "four ticks at 4x is 0.25 s of real time");
}

// ---------------------------------------------------------------------------
// TUI-RUN-021 — the monotonic clock drives ordinary accumulation
// ---------------------------------------------------------------------------

/// A wall clock that reports nothing — stalled, or stepped backwards by NTP —
/// cannot stall the simulation, because accumulation is driven by the
/// monotonic reading.
/// @spec TUI-RUN-021
#[test]
fn the_monotonic_clock_drives_accumulation_when_the_wall_clock_does_not_move() {
    let mut acc = TickAccumulator::default();

    acc.observe(reading(Duration::from_secs(4), Duration::ZERO), 1.0);
    assert_eq!(
        acc.backlog(),
        4 * TPS,
        "four monotonic seconds owe four seconds of ticks whatever the wall clock says"
    );
    assert_span(acc.unrun_span(), 4.0, "the span follows the monotonic reading");
}

/// No observed time, no ticks: the accumulator never invents an interval.
/// @spec TUI-RUN-021
#[test]
fn no_elapsed_time_injects_no_ticks() {
    let mut acc = TickAccumulator::default();

    acc.observe(steady(Duration::ZERO), 1.0);
    acc.observe(steady(Duration::ZERO), 8.0);
    assert_eq!(acc.backlog(), 0, "a zero-length frame owes nothing at any speed");
    assert_span(acc.unrun_span(), 0.0, "and represents no real time");
}

// ---------------------------------------------------------------------------
// TUI-RUN-022 — the monotonic clock stops while the machine is suspended
// ---------------------------------------------------------------------------

/// Eight hours with the lid closed is eight hours of absence. The monotonic
/// clock saw a couple of seconds of it; the wall clock saw all of it, and the
/// excess is the part the player must not be charged for.
///
/// The excess used here is hours, far larger than any plausible value of the
/// "small tolerance" the spec leaves unquantified.
/// @spec TUI-RUN-022
#[test]
fn a_suspended_machine_costs_the_player_no_game_time() {
    let eight_hours = 8 * 3_600;

    let mut suspended = TickAccumulator::default();
    suspended.observe(
        reading(Duration::from_secs(1), Duration::from_secs(eight_hours + 1)),
        1.0,
    );

    let mut attended = TickAccumulator::default();
    attended.observe(steady(Duration::from_secs(eight_hours + 1)), 1.0);

    assert_eq!(
        suspended.backlog(),
        (eight_hours + 1) * TPS,
        "the wall-clock excess is absence the monotonic clock did not see"
    );
    assert_eq!(
        suspended.backlog(),
        attended.backlog(),
        "a suspended machine owes exactly what an attended one would have earned"
    );
    assert_span(suspended.unrun_span(), (eight_hours + 1) as f64, "the whole absence is banked");
}

/// "Through the same accumulator": suspend excess is not a special path — it
/// obeys the current speed exactly as attended time does.
/// @spec TUI-RUN-022
#[test]
fn suspend_excess_runs_through_the_same_accumulator() {
    let mut acc = TickAccumulator::default();

    // 60 monotonic seconds, 660 wall seconds: a ten-minute suspend inside an
    // otherwise ordinary interval, at double speed.
    acc.observe(reading(Duration::from_secs(60), Duration::from_secs(660)), 2.0);
    assert_eq!(
        acc.backlog(),
        660 * TPS * 2,
        "the recovered excess is converted at the current speed like any other time"
    );
    assert_span(acc.unrun_span(), 660.0, "and banks the real interval it represents");
}

// ---------------------------------------------------------------------------
// TUI-RUN-023 — a backwards wall clock
// ---------------------------------------------------------------------------

/// @spec TUI-RUN-023
#[test]
fn a_backwards_wall_clock_never_rewinds_the_simulation() {
    let mut acc = TickAccumulator::default();

    acc.observe(steady(Duration::from_secs(10)), 1.0);
    let before = acc.backlog();
    assert_eq!(before, 10 * TPS);

    // The wall clock reports less than the monotonic clock: a correction, not
    // a suspend. The monotonic reading wins.
    acc.observe(reading(Duration::from_secs(10), Duration::from_secs(2)), 1.0);
    assert_eq!(
        acc.backlog(),
        before + 10 * TPS,
        "the monotonic reading is kept in full when the wall clock falls short"
    );
    assert!(acc.backlog() >= before, "the simulation never moves backwards");
    assert_span(acc.unrun_span(), 20.0, "the banked span follows the monotonic reading too");
}

/// The checkpoint re-bases silently: a backwards correction leaves no deficit
/// to be worked off by the frames that follow it.
/// @spec TUI-RUN-023
#[test]
fn a_backwards_wall_clock_leaves_no_lingering_deficit() {
    let mut acc = TickAccumulator::default();

    acc.observe(reading(Duration::from_secs(10), Duration::ZERO), 1.0);
    assert_eq!(acc.backlog(), 10 * TPS);

    acc.observe(steady(Duration::from_secs(10)), 1.0);
    assert_eq!(
        acc.backlog(),
        20 * TPS,
        "the next ordinary frame accrues in full; the earlier shortfall is not carried"
    );
}

// ---------------------------------------------------------------------------
// TUI-RUN-024 — one absence rule, shared with the load path
// ---------------------------------------------------------------------------

/// A two-year suspend and a two-year closed save must behave identically, so
/// the in-process gap uses the core's exported cap rather than a copy of the
/// number.
/// @spec TUI-RUN-024
#[test]
fn an_in_process_gap_is_capped_at_the_cores_catch_up_limit() {
    let two_years = 2 * 365 * 86_400;
    assert!(two_years > CATCHUP_CAP_SECONDS, "fixture must exceed the cap to exercise it");

    let mut acc = TickAccumulator::default();
    acc.observe(reading(Duration::from_secs(1), Duration::from_secs(two_years)), 1.0);

    assert_eq!(
        acc.backlog(),
        CATCHUP_CAP_SECONDS * TPS,
        "an in-process gap replays exactly the capped interval the load path would replay"
    );
    assert_span(
        acc.unrun_span(),
        CATCHUP_CAP_SECONDS as f64,
        "and banks only the capped span, so a back-dated save cannot reach past the cap",
    );
}

/// The cap is a ceiling, not a rounding: a gap just under it is untouched.
/// @spec TUI-RUN-024
#[test]
fn a_gap_below_the_cap_is_not_truncated() {
    let gap = CATCHUP_CAP_SECONDS - 10;

    let mut acc = TickAccumulator::default();
    acc.observe(reading(Duration::from_secs(1), Duration::from_secs(gap)), 1.0);

    assert_eq!(acc.backlog(), gap * TPS, "an absence inside the cap is replayed in full");
    assert_span(acc.unrun_span(), gap as f64, "and banks its whole span");
}

// ---------------------------------------------------------------------------
// TUI-RUN-025 — catch-up is paced, never dropped
// ---------------------------------------------------------------------------

/// @spec TUI-RUN-025
#[test]
fn a_backlog_drains_in_bounded_slices_and_nothing_owed_is_discarded() {
    let mut acc = TickAccumulator::default();
    acc.observe(steady(Duration::from_secs(250)), 1.0);

    let total = acc.backlog();
    assert_eq!(total, 250 * TPS, "fixture: a thousand ticks owed");

    let first = acc.take(100);
    assert_eq!(first, 100, "a frame runs at most its bound");
    assert_eq!(acc.backlog(), total - 100, "the remainder stays banked");

    // Half-drained, the banked span shrinks with the backlog it describes.
    let half = total / 2;
    while acc.backlog() > half {
        acc.take((acc.backlog() - half).min(100));
    }
    assert_eq!(acc.backlog(), half);
    assert_span(
        acc.unrun_span(),
        125.0,
        "half a backlog accrued at one speed represents half its real time",
    );

    let mut ran = 100 + (total - 100 - half);
    let mut frames = 0;
    while acc.backlog() > 0 {
        let n = acc.take(100);
        assert!(n <= 100, "take never exceeds its bound");
        assert!(n > 0, "a non-empty backlog must make progress");
        ran += n;
        frames += 1;
        assert!(frames < 1_000, "the drain must terminate");
    }

    assert_eq!(ran, total, "every owed tick is eventually run; none is discarded");
    assert_eq!(acc.take(100), 0, "an empty backlog yields nothing");
    assert_span(acc.unrun_span(), 0.0, "a fully drained backlog owes no real time");
}

// ---------------------------------------------------------------------------
// TUI-RUN-032 — the unrun span is banked as it accrues, never derived
// ---------------------------------------------------------------------------

/// Dividing the backlog by the *current* speed is wrong the moment the backlog
/// accrued at a different one. Banking the span as it accrues makes the answer
/// exact regardless of the speed history.
/// @spec TUI-RUN-032
#[test]
fn the_unrun_span_is_banked_as_it_accrues_not_derived_from_the_backlog() {
    let mut acc = TickAccumulator::default();

    // Ten seconds at the fastest speed, then one second at the slowest.
    acc.observe(steady(Duration::from_secs(10)), 8.0);
    assert_eq!(acc.backlog(), 10 * TPS * 8);
    acc.observe(steady(Duration::from_secs(1)), 0.25);

    let backlog = acc.backlog();
    assert_eq!(backlog, 10 * TPS * 8 + 1, "321 ticks owed across two speeds");

    assert_span(acc.unrun_span(), 11.0, "eleven real seconds elapsed, and eleven were banked");

    let derived_at_current_speed = backlog as f64 / (TPS_F * 0.25);
    assert!(
        (acc.unrun_span().as_secs_f64() - derived_at_current_speed).abs() > 1.0,
        "the span must not be derived from the backlog at the current speed ({derived_at_current_speed}s)"
    );
    let derived_at_previous_speed = backlog as f64 / (TPS_F * 8.0);
    assert!(
        (acc.unrun_span().as_secs_f64() - derived_at_previous_speed).abs() > 0.5,
        "nor at the previous one ({derived_at_previous_speed}s)"
    );
}

/// The derived span does not merely differ — across the full speed range it
/// differs by the ratio between the speeds, which is enough to push a modest
/// absence past the absence cap and back-date a save into an absence that
/// never happened.
/// @spec TUI-RUN-032
#[test]
fn a_derived_span_would_cross_the_absence_cap_on_an_absence_that_never_happened() {
    let twenty_days = 20 * 86_400;

    let mut acc = TickAccumulator::default();
    acc.observe(steady(Duration::from_secs(twenty_days)), 8.0);
    acc.observe(steady(Duration::from_secs(1)), 0.25);

    let derived = acc.backlog() as f64 / (TPS_F * 0.25);
    assert!(
        derived > CATCHUP_CAP_SECONDS as f64,
        "fixture: the naive derivation ({derived}s) must exceed the cap"
    );

    let span = acc.unrun_span();
    assert_span(span, (twenty_days + 1) as f64, "the true span is the interval actually observed");
    assert!(
        span.as_secs() < CATCHUP_CAP_SECONDS,
        "a twenty-day absence must stay a twenty-day absence"
    );
}

// ---------------------------------------------------------------------------
// TUI-RUN-031 — shutdown does not wait for the sim to catch up
// ---------------------------------------------------------------------------

/// Reporting the unrun interval must not require draining it: shutdown asks
/// the accumulator what is owed and leaves immediately.
/// @spec TUI-RUN-031
#[test]
fn an_outstanding_backlog_is_reported_without_being_drained() {
    let mut acc = TickAccumulator::default();
    acc.observe(steady(Duration::from_secs(60)), 1.0);

    let before = acc.backlog();
    let span = acc.unrun_span();
    assert!(span > Duration::ZERO, "an outstanding backlog owes real time");
    assert_eq!(acc.unrun_span(), span, "asking twice gives the same answer");
    assert_eq!(acc.backlog(), before, "asking does not run anything");
}

/// The seam between the banked backlog and the core's load path. Back-dating
/// the save stamp by the unrun span makes the owed interval run exactly once,
/// at the next load — neither lost nor counted twice.
/// @spec TUI-RUN-031, TUI-RUN-032
#[test]
fn back_dating_by_the_unrun_span_carries_the_backlog_into_the_next_load() {
    let mut state = GameState::new(11);
    state.speed = 1.0;
    assert_eq!(state.ticks, 0, "fixture: a fresh game has run no ticks");

    // A minute of absence accrues, and shutdown arrives before any of it runs.
    let mut acc = TickAccumulator::default();
    acc.observe(steady(Duration::from_secs(60)), 1.0);
    let owed = acc.backlog();
    assert_eq!(owed, 60 * TPS);

    let now_unix = 1_800_000_000u64;
    let saved_at = now_unix - acc.unrun_span().as_secs();
    assert_eq!(saved_at, now_unix - 60, "the stamp names the instant the sim actually reached");

    let loaded = load_string(&save_string(&state, saved_at), now_unix)
        .expect("a freshly written save loads");
    assert_eq!(
        loaded.state.ticks, owed,
        "the unrun interval runs exactly once at the next load — not lost, not doubled"
    );

    // Why back-dating exists: a wall-clock stamp on a banked save tells the
    // next load it is caught up, and the owed interval simply vanishes.
    let naive =
        load_string(&save_string(&state, now_unix), now_unix).expect("a freshly written save loads");
    assert_eq!(naive.state.ticks, 0, "a wall-clock stamp would swallow the owed interval");
}
