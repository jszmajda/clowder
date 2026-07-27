//! The boundary between the operating system and the pure core: terminal
//! lifecycle, the loop, time, autosave, and the failure policy for a save that
//! will not load. The only place in clowder that reads a clock, touches the
//! filesystem, or handles a signal. @spec TUI-RUN-001..031

pub mod clock;
pub mod lock;
pub mod persist;
pub mod term;

use std::process::ExitCode;

pub use clock::TickAccumulator;
pub use lock::SessionLock;
pub use term::TerminalGuard;

/// Start up, run the loop, shut down. Returns a non-success code only for a
/// refusal to start — a lock held by a live session, a save from a newer
/// version, or a save that cannot be read at all.
/// @spec TUI-RUN-001, TUI-RUN-007, TUI-RUN-011, TUI-RUN-013
pub fn run() -> ExitCode {
    todo!("TUI-RUN-001..014: startup order, the loop, shutdown")
}

/// One iteration: poll input, apply commands, advance the sim, autosave if
/// due, render. Commands land before the frame's ticks, so a key pressed
/// during the preceding wait takes effect at the time it was pressed rather
/// than after the elapsed interval. @spec TUI-RUN-014, TUI-RUN-015
pub fn step() {
    todo!("TUI-RUN-014..017: the loop body, in order")
}
