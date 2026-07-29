//! Time. Two clocks, because neither alone is correct.
//!
//! A monotonic clock drives ordinary accumulation, so a wall-clock correction
//! can neither rewind the sim nor inject time it never observed. But on every
//! platform we target the monotonic clock *stops while the machine is
//! suspended* — a laptop closed for eight hours with clowder running would
//! wake having advanced a few seconds, silently punishing the player for
//! walking away. So a wall-clock checkpoint runs alongside it, and the excess
//! is real absence converted through the same accumulator.
//!
//! Both clocks are *passed in* rather than read here. The specs this component
//! must satisfy — suspend detection, a backwards wall clock, the shared
//! absence cap — are precisely the ones that need headless tests at arbitrary
//! instants, and a component that reads the clock itself cannot have them.
//! @spec TUI-RUN-018..025, TUI-RUN-030

use std::time::Duration;

use clowder_core::content::TICKS_PER_SECOND;
use clowder_core::CATCHUP_CAP_SECONDS;

const TICKS_PER_SECOND_F: f64 = TICKS_PER_SECOND as f64;
const CATCHUP_CAP_SECONDS_F: f64 = CATCHUP_CAP_SECONDS as f64;

/// A reading of both clocks, taken together by the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reading {
    /// Elapsed on the monotonic clock since the previous reading.
    pub monotonic: Duration,
    /// Elapsed on the wall clock since the previous reading. Less than
    /// `monotonic` means the wall clock was corrected backwards.
    pub wall: Duration,
}

/// Banked time, held as three distinct quantities.
///
/// Conflating the first two is the easy mistake: a speed change applies to the
/// sub-tick remainder and *not* to the whole-tick backlog, whose ticks were
/// converted at the speed in force when their time elapsed. Rescaling those
/// would retroactively rewrite history the player already lived through.
///
/// The third exists because the backlog alone cannot answer how much real time
/// it represents. Dividing ticks by the *current* speed is wrong whenever the
/// backlog accrued at a different one — a backlog banked at the fastest speed
/// and saved at the slowest would back-date by thirty-two times the true
/// interval, and could spuriously cross the absence cap. Banking the span as
/// it accrues makes the answer exact regardless of speed history.
/// @spec TUI-RUN-019, TUI-RUN-027, TUI-RUN-030
#[derive(Debug, Default, Clone, PartialEq)]
pub struct TickAccumulator {
    /// Real time not yet worth a whole tick, in seconds.
    fractional: f64,
    /// Whole ticks converted but not yet run. @spec TUI-RUN-019
    backlog: u64,
    /// Real time the unrun backlog represents, in seconds. @spec TUI-RUN-027
    unrun_seconds: f64,
}

impl TickAccumulator {
    /// Fold an interval into banked time. Uses the larger of the two clocks'
    /// elapsed — the excess of wall over monotonic being absence the monotonic
    /// clock did not see — and caps the result at the shared absence limit.
    /// @spec TUI-RUN-018, TUI-RUN-021..024
    pub fn observe(&mut self, reading: Reading, speed: f64) {
        // @spec TUI-RUN-018, TUI-RUN-021, TUI-RUN-022, TUI-RUN-023, TUI-RUN-024
        let monotonic = reading.monotonic.as_secs_f64();
        let wall = reading.wall.as_secs_f64();
        // TUI-RUN-021/022: the monotonic clock drives ordinary accumulation;
        // when the wall clock shows more elapsed, that excess is suspend
        // absence the monotonic clock never saw. TUI-RUN-023: when the wall
        // clock shows less (a backwards correction), the monotonic reading
        // wins outright.
        let mut elapsed = monotonic.max(wall);

        // TUI-RUN-024: one shared absence cap with the load path.
        if elapsed > CATCHUP_CAP_SECONDS_F {
            elapsed = CATCHUP_CAP_SECONDS_F;
        }

        // TUI-RUN-018/020/030: convert THIS interval at THIS speed, now. What
        // is banked is a fraction of a *tick* — already converted — so a later
        // speed change cannot reach back and re-rate time the player already
        // lived at another pace. Banking seconds instead would do exactly
        // that: the remainder would silently convert at whatever speed
        // happened to be set when it was next folded.
        let ticks_f = elapsed * TICKS_PER_SECOND_F * speed + self.fractional;
        let whole_ticks = ticks_f.floor().max(0.0) as u64;
        self.fractional = ticks_f - whole_ticks as f64;
        if whole_ticks > 0 {
            // TUI-RUN-032: bank the span these ticks represent as they accrue.
            // Deriving it later from the backlog and the *current* speed would
            // be wrong by the ratio between speeds across any change.
            self.unrun_seconds += whole_ticks as f64 / (TICKS_PER_SECOND_F * speed);
            self.backlog += whole_ticks;
        }
    }

    /// Draw down at most `max_ticks` from the backlog, so a long catch-up
    /// drains across frames instead of blocking the loop. The caller chooses
    /// the bound, having measured what a frame can afford — the accumulator
    /// cannot know what a tick costs. @spec TUI-RUN-025
    pub fn take(&mut self, max_ticks: u64) -> u64 {
        // @spec TUI-RUN-025, TUI-RUN-031, TUI-RUN-032
        let n = max_ticks.min(self.backlog);
        if n > 0 {
            // The banked span was accrued per-conversion, possibly at
            // different speeds; a drain removes its proportional share so
            // the remaining span still describes the remaining backlog
            // exactly, without re-deriving it from a single current speed.
            let per_tick = self.unrun_seconds / self.backlog as f64;
            self.unrun_seconds = (self.unrun_seconds - per_tick * n as f64).max(0.0);
            self.backlog -= n;
        }
        n
    }

    /// The real-time span the unrun backlog represents.
    ///
    /// A save is stamped with the instant the *simulation* has reached — now
    /// less this — because a banked save stamped with the wall clock would
    /// tell the next load it was caught up when it was not, and that load
    /// computes its own elapsed from that origin. The owed interval would
    /// simply vanish. @spec TUI-RUN-027
    pub fn unrun_span(&self) -> Duration {
        Duration::from_secs_f64(self.unrun_seconds.max(0.0))
    }

    pub fn backlog(&self) -> u64 {
        self.backlog
    }
}
