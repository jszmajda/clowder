//! The boundary between the operating system and the pure core: terminal
//! lifecycle, the loop, time, autosave, and the failure policy for a save that
//! will not load. The only place in clowder that reads a clock, touches the
//! filesystem, or handles a signal. @spec TUI-RUN-001..031

pub mod clock;
pub mod lock;
pub mod persist;
pub mod term;

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use clowder_core::GameState;

use crate::nav::ViewState;
use crate::render::Layout;

pub use clock::TickAccumulator;
pub use lock::SessionLock;
pub use term::TerminalGuard;

/// Everything one session owns. The loop is a method on this rather than a
/// free function: a step needs the game, the view, banked time, the previous
/// layout, and the save cadence all at once, and threading those through free
/// functions would mean either a long parameter list or globals.
pub struct App {
    pub state: GameState,
    pub view: ViewState,
    pub clock: TickAccumulator,
    pub save_path: PathBuf,
    pub last_save: Instant,
    /// Previous frame's layout, so breakpoints can apply their dead-band.
    /// @spec TUI-RENDER-008
    pub prev_layout: Option<Layout>,
    pub quitting: bool,
}

/// Start up, run the loop, shut down. Returns a non-success code only for a
/// refusal to start — a lock held by a live session, a save from a newer
/// version, or a save that cannot be read at all.
/// @spec TUI-RUN-001, TUI-RUN-007, TUI-RUN-011, TUI-RUN-013
pub fn run() -> ExitCode {
    todo!("TUI-RUN-001/006/029: lock, load, enter raw mode, loop, shut down")
}

impl App {
    /// One iteration: poll input, apply commands, advance the sim, autosave if
    /// due, render. Commands land before the frame's ticks, so a key pressed
    /// during the preceding wait takes effect at the time it was pressed
    /// rather than after the elapsed interval.
    /// @spec TUI-RUN-014, TUI-RUN-015, TUI-RUN-016, TUI-RUN-017
    pub fn step(&mut self) -> std::io::Result<()> {
        todo!("TUI-RUN-014..017: the loop body, in order")
    }

    /// Save, using a timestamp naming the instant the *simulation* has
    /// reached rather than the wall clock. @spec TUI-RUN-027, TUI-RUN-031
    pub fn save(&self) -> std::io::Result<()> {
        todo!("TUI-RUN-026/027/028: back-dated stamp, atomic write, tolerate failure")
    }
}
