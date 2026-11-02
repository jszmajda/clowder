//! The shell's one read of each clock.
//!
//! Everything the shell paces or times out on real time asks here, so a test
//! can hold the clocks and move them by hand rather than sleep: a test of a
//! two-second behaviour advances two seconds and takes none, and a test of a
//! night's suspend moves the wall clock alone. The hold belongs to one thread,
//! because a crate's tests share a process and run side by side, and a hold
//! reaching past its own test would stop the clock under all the others.
//! @spec TUI-RUN-066, TUI-RUN-067, TUI-RUN-075

use std::cell::Cell;
use std::time::{Duration, Instant, SystemTime};

thread_local! {
    static HELD: Cell<Option<Instant>> = const { Cell::new(None) };
    static HELD_WALL: Cell<Option<SystemTime>> = const { Cell::new(None) };
}

/// The monotonic clock as this thread reads it: where a hold has left it, or
/// real time where nothing holds it. @spec TUI-RUN-066
pub fn instant() -> Instant {
    HELD.with(Cell::get).unwrap_or_else(Instant::now)
}

/// The wall clock as this thread reads it: where a hold has left it, or real
/// time where nothing holds it. @spec TUI-RUN-075
pub fn wall() -> SystemTime {
    HELD_WALL.with(Cell::get).unwrap_or_else(SystemTime::now)
}

/// How long ago `earlier` was, on this thread's clock.
pub fn since(earlier: Instant) -> Duration {
    instant().saturating_duration_since(earlier)
}

/// A hold on this thread's clock, ending where the value is dropped, so a hold
/// never outlives what took it, even on a thread later tests share.
#[must_use = "the clock runs again as soon as this is dropped"]
pub struct Held {
    previous: Option<Instant>,
    previous_wall: Option<SystemTime>,
}

/// Stop this thread's clocks where they read now, until [`advance`] or
/// [`suspend`] moves them. @spec TUI-RUN-067, TUI-RUN-075
pub fn held() -> Held {
    let (now, wall_now) = (instant(), wall());
    let previous = HELD.with(|h| h.replace(Some(now)));
    let previous_wall = HELD_WALL.with(|h| h.replace(Some(wall_now)));
    Held {
        previous,
        previous_wall,
    }
}

/// Move this thread's held clocks forward by `by`, which takes no real time.
///
/// # Panics
/// Where nothing holds this thread's clock: time that was meant to pass
/// wouldn't have, and the caller would be asserting on a clock that never moved.
pub fn advance(by: Duration) {
    HELD.with(|h| match h.get() {
        Some(now) => h.set(Some(now + by)),
        None => panic!("advance on a clock nothing holds: take `now::held()` first"),
    });
    HELD_WALL.with(|h| h.set(h.get().map(|wall| wall + by)));
}

/// Stand in for this thread's machine sleeping for `by`: the wall clock moves
/// on, and the monotonic clock, as it does on every platform we target, does
/// not. @spec TUI-RUN-075
///
/// # Panics
/// Where nothing holds this thread's clock, as for [`advance`].
pub fn suspend(by: Duration) {
    HELD_WALL.with(|h| match h.get() {
        Some(wall) => h.set(Some(wall + by)),
        None => panic!("suspend on a clock nothing holds: take `now::held()` first"),
    });
}

impl Drop for Held {
    fn drop(&mut self) {
        HELD.with(|h| h.set(self.previous));
        HELD_WALL.with(|h| h.set(self.previous_wall));
    }
}
