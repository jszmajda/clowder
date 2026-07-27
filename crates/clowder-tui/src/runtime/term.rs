//! Terminal lifecycle. Raw mode and the alternate screen are entered once and
//! must be undone on *every* exit path — three mechanisms, because each covers
//! a path the others structurally cannot: a Drop guard for normal returns and
//! error propagation, a panic hook for panics, and signal handlers for the
//! kill that never unwinds at all. A tool that lives in a tmux pane is killed
//! by the pane far more often than it is quit.
//! @spec TUI-RUN-002, TUI-RUN-003, TUI-RUN-004

/// Restores the terminal when dropped. @spec TUI-RUN-002
#[derive(Debug)]
pub struct TerminalGuard {
    _private: (),
}

impl TerminalGuard {
    /// Enter raw mode and the alternate screen, and install the panic hook.
    /// Called only after the save has been read, so a load failure can still
    /// print to an ordinary terminal. @spec TUI-RUN-001, TUI-RUN-003
    pub fn enter() -> std::io::Result<Self> {
        todo!("TUI-RUN-002/003: raw mode, alternate screen, panic hook")
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // @spec TUI-RUN-002
    }
}

/// Install SIGTERM/SIGHUP handlers. They only record; the loop takes the
/// ordinary shutdown path, because handler-side work is not async-signal-safe.
/// Ctrl-C is not here — in raw mode the terminal delivers it as a key.
/// @spec TUI-RUN-004
pub fn install_signal_handlers() -> std::io::Result<()> {
    todo!("TUI-RUN-004: record the signal, let the loop shut down")
}

/// Whether a termination signal has arrived. @spec TUI-RUN-004
pub fn termination_requested() -> bool {
    todo!("TUI-RUN-004: observe the recorded signal")
}
