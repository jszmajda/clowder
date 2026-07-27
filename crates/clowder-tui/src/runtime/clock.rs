//! Time. Two clocks, because neither alone is correct.
//!
//! A monotonic clock drives ordinary accumulation, so a wall-clock correction
//! can neither rewind the sim nor inject time it never observed. But on every
//! platform we target the monotonic clock *stops while the machine is
//! suspended* — a laptop closed for eight hours with clowder running would
//! wake having advanced a few seconds, silently punishing the player for
//! walking away. So a wall-clock checkpoint runs alongside it, and the excess
//! is real absence converted through the same accumulator.
//! @spec TUI-RUN-018..025

use std::time::Duration;

/// Banked time, held as two distinct quantities. Conflating them is the easy
/// mistake: a speed change applies to the sub-tick remainder and *not* to the
/// backlog, whose ticks were converted at the speed in force when their time
/// elapsed. Rescaling those would retroactively rewrite history the player
/// already lived through. @spec TUI-RUN-030
#[derive(Debug, Default)]
pub struct TickAccumulator {
    /// Time not yet worth a whole tick.
    fractional: f64,
    /// Whole ticks converted but not yet run. @spec TUI-RUN-019
    backlog: u64,
}

impl TickAccumulator {
    /// Fold the interval since the last observation into banked time, using
    /// both clocks. Returns nothing — what was banked is drained by `take`.
    /// @spec TUI-RUN-018, TUI-RUN-021, TUI-RUN-022, TUI-RUN-023, TUI-RUN-024
    pub fn observe(&mut self, _speed: f64) {
        todo!("TUI-RUN-018/021-024: two clocks, suspend detection, the shared cap")
    }

    /// Draw down the backlog for at most `budget` of wall time, so a long
    /// catch-up drains across frames instead of blocking the loop. Returns how
    /// many ticks to run now. @spec TUI-RUN-025
    pub fn take(&mut self, _budget: Duration) -> u64 {
        todo!("TUI-RUN-025: time-budgeted drain, remainder stays banked")
    }

    /// The real-time span the unrun backlog represents.
    ///
    /// A save is stamped with the instant the *simulation* has reached — now
    /// less this — because a banked save stamped with the wall clock would
    /// tell the next load it was caught up when it was not, and that load
    /// computes its own elapsed from that origin. The owed interval would
    /// simply vanish. @spec TUI-RUN-027
    pub fn unrun_span(&self, _speed: f64) -> Duration {
        todo!("TUI-RUN-027: the interval the backlog represents, for back-dating")
    }

    pub fn backlog(&self) -> u64 {
        self.backlog
    }
}
