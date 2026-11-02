//! The shell's one read of the monotonic clock.
//!
//! Everything the shell paces or times out on real time asks here, so a test
//! can hold the clock and move it by hand rather than sleep: a test of a
//! two-second behaviour advances two seconds and takes none. The hold belongs to
//! one thread, because a crate's tests share a process and run side by side, and
//! a hold reaching past its own test would stop the clock under all the others.
//! @spec TUI-RUN-066, TUI-RUN-067

use std::cell::Cell;
use std::time::{Duration, Instant};

thread_local! {
    static HELD: Cell<Option<Instant>> = const { Cell::new(None) };
}

/// The monotonic clock as this thread reads it: where a hold has left it, or
/// real time where nothing holds it. @spec TUI-RUN-066
pub fn instant() -> Instant {
    HELD.with(Cell::get).unwrap_or_else(Instant::now)
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
}

/// Stop this thread's clock where it reads now, until [`advance`] moves it.
/// @spec TUI-RUN-067
pub fn held() -> Held {
    let now = instant();
    let previous = HELD.with(|h| h.replace(Some(now)));
    Held { previous }
}

/// Move this thread's held clock forward by `by`, which takes no real time.
///
/// # Panics
/// Where nothing holds this thread's clock: time that was meant to pass
/// wouldn't have, and the caller would be asserting on a clock that never moved.
pub fn advance(by: Duration) {
    HELD.with(|h| match h.get() {
        Some(now) => h.set(Some(now + by)),
        None => panic!("advance on a clock nothing holds: take `now::held()` first"),
    });
}

impl Drop for Held {
    fn drop(&mut self) {
        HELD.with(|h| h.set(self.previous));
    }
}
