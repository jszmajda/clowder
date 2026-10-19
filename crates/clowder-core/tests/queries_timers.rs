//! What runs on a clock: the timed states the meadow holds, each with the
//! ticks it has left. Written tests-first from SIM-ENG-069 and SIM-ENG-070.

mod common;

use clowder_core::*;
use common::*;

const YEAR_TICKS: u64 = content::DAYS_PER_YEAR * content::TICKS_PER_DAY;

/// A meadow a little way into a year, off every day boundary, so no state
/// below ends on a tick that also rolls for a fresh one.
fn meadow() -> GameState {
    let mut s = GameState::new(311);
    run_ticks(&mut s, 3 * YEAR_TICKS + 437);
    assert_ne!(s.ticks % content::TICKS_PER_DAY, 0);
    s.zoomies_until = 0;
    s.jamboree_until = 0;
    s.deja_vu_until = 0;
    s.pending_sky_event = None;
    s.whale_visit_years = 0;
    s
}

fn left(s: &GameState, t: Timer) -> Option<u64> {
    s.timers()
        .into_iter()
        .find(|(k, _)| *k == t)
        .map(|(_, n)| n)
}

// @spec SIM-ENG-069
#[test]
fn a_quiet_meadow_runs_nothing_on_a_clock() {
    assert_eq!(GameState::new(1).timers(), vec![]);
    assert_eq!(meadow().timers(), vec![]);
}

// @spec SIM-ENG-069
#[test]
fn every_timed_state_is_listed_in_the_fixed_order() {
    let mut s = meadow();
    let now = s.ticks;
    s.deja_vu_until = now + 3;
    s.zoomies_until = now + 50;
    s.jamboree_until = now + 400;
    s.whale_visit_years = 2;
    s.pending_sky_event = Some(now + 70);
    let kinds: Vec<Timer> = s.timers().into_iter().map(|(k, _)| k).collect();
    assert_eq!(
        kinds,
        vec![
            Timer::SkyEvent,
            Timer::WhaleVisit,
            Timer::Jamboree,
            Timer::Zoomies,
            Timer::DejaVuDay,
        ]
    );
}

// @spec SIM-ENG-069
#[test]
fn a_state_whose_rule_no_longer_holds_is_not_listed() {
    let mut s = meadow();
    let now = s.ticks;
    // Each field set to an end already reached: nothing is running.
    s.pending_sky_event = Some(now);
    s.jamboree_until = now;
    s.zoomies_until = now;
    s.deja_vu_until = now;
    assert_eq!(s.timers(), vec![]);
    assert!(!s.zoomies_active());
    assert!(!s.jamboree_active());
}

/// Tick until `t` is one tick from its end, check it is still listed with one
/// tick left, then tick once more and check it has ended.
fn ends_on_time(mut s: GameState, t: Timer, ended: impl Fn(&GameState) -> bool) {
    let n = left(&s, t).expect("listed at the start");
    assert!(n > 1);
    run_ticks(&mut s, n - 1);
    assert_eq!(left(&s, t), Some(1), "{t:?}: one tick before its end");
    assert!(!ended(&s), "{t:?}: still running one tick before its end");
    run_ticks(&mut s, 1);
    assert!(ended(&s), "{t:?}: ended after the ticks it had left");
    assert_eq!(left(&s, t), None);
}

// @spec SIM-ENG-070
#[test]
fn a_sky_event_has_the_ticks_left_until_its_expiry() {
    let mut s = meadow();
    let expiry = (s.ticks / content::TICKS_PER_DAY + 40) * content::TICKS_PER_DAY;
    s.pending_sky_event = Some(expiry);
    assert_eq!(left(&s, Timer::SkyEvent), Some(expiry - s.ticks));
    ends_on_time(s, Timer::SkyEvent, |s| {
        s.clone().apply(Command::ObserveSky) == Err(CommandError::Locked)
    });
}

// @spec SIM-ENG-070
#[test]
fn the_zoomies_jamboree_and_deja_vu_have_the_ticks_left_until_their_ends() {
    let mut s = meadow();
    s.zoomies_until = s.ticks + 93;
    assert_eq!(left(&s, Timer::Zoomies), Some(93));
    ends_on_time(s, Timer::Zoomies, |s| !s.zoomies_active());

    let mut s = meadow();
    s.jamboree_until = s.ticks + 611;
    assert_eq!(left(&s, Timer::Jamboree), Some(611));
    ends_on_time(s, Timer::Jamboree, |s| !s.jamboree_active());

    let mut s = meadow();
    s.deja_vu_until = s.ticks + 7;
    assert_eq!(left(&s, Timer::DejaVuDay), Some(7));
    ends_on_time(s, Timer::DejaVuDay, |s| s.deja_vu_until <= s.ticks);
}

// @spec SIM-ENG-070
#[test]
fn a_whale_visit_runs_to_the_year_close_where_its_last_year_runs_out() {
    let mut s = meadow();
    s.whale_visit_years = 3;
    let next_close = (s.ticks / YEAR_TICKS + 1) * YEAR_TICKS;
    assert_eq!(
        left(&s, Timer::WhaleVisit),
        Some(next_close - s.ticks + 2 * YEAR_TICKS)
    );
    ends_on_time(s, Timer::WhaleVisit, |s| s.whale_visit_years == 0);

    // Standing exactly on a year close, that close has already counted.
    let mut s = meadow();
    s.whale_visit_years = 1;
    let to_close = YEAR_TICKS - s.ticks % YEAR_TICKS;
    run_ticks(&mut s, to_close);
    assert_eq!(s.ticks % YEAR_TICKS, 0);
    assert_eq!(s.whale_visit_years, 0, "the close took the last year");
    assert_eq!(left(&s, Timer::WhaleVisit), None);

    let mut s = meadow();
    s.whale_visit_years = 2;
    let to_close = YEAR_TICKS - s.ticks % YEAR_TICKS;
    run_ticks(&mut s, to_close);
    assert_eq!(s.whale_visit_years, 1);
    assert_eq!(left(&s, Timer::WhaleVisit), Some(YEAR_TICKS));
}
