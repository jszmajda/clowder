//! Terminal lifecycle. Raw mode and the alternate screen are entered once and
//! must be undone on *every* exit path — three mechanisms, because each covers
//! a path the others structurally cannot: a Drop guard for normal returns and
//! error propagation, a panic hook for panics, and signal handlers for the
//! kill that never unwinds at all. A tool that lives in a tmux pane is killed
//! by the pane far more often than it is quit.
//! Mouse reporting is part of the same taking and giving back: turned on after
//! the screen is taken, off before it is given back, on every one of these
//! paths. @spec TUI-RUN-002, TUI-RUN-003, TUI-RUN-004, TUI-RUN-060,
//! TUI-RUN-061, TUI-RUN-062

use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};

/// Set by the signal handlers, observed by the event loop. Never touched by
/// anything other than an atomic store/load, so it is safe to write from a
/// signal handler.
static TERMINATION_REQUESTED: AtomicBool = AtomicBool::new(false);

/// Whether the terminal is taken and not yet given back, so that a panic's
/// restoration and the guard's drop during the unwinding that follows it give
/// it back once between them. @spec TUI-RUN-061
static TAKEN: AtomicBool = AtomicBool::new(false);

/// Restores the terminal when dropped. @spec TUI-RUN-002
#[derive(Debug)]
pub struct TerminalGuard {
    _private: (),
}

impl TerminalGuard {
    /// Enter raw mode and the alternate screen, turn on mouse reporting, and
    /// install the panic hook. Called only after the save has been read, so a
    /// load failure can still print to an ordinary terminal. Where any step
    /// fails, everything entered so far is undone before the failure is
    /// returned. @spec TUI-RUN-001, TUI-RUN-003, TUI-RUN-060, TUI-RUN-062
    pub fn enter() -> std::io::Result<Self> {
        TAKEN.store(true, Ordering::SeqCst);
        let entered = crossterm::terminal::enable_raw_mode()
            .and_then(|()| crossterm::execute!(io::stdout(), crossterm::terminal::EnterAlternateScreen))
            .and_then(|()| write_flushed(MOUSE_REPORTING_ON));
        if let Err(err) = entered {
            // Don't leave any of it on if entering failed partway through.
            restore_terminal();
            return Err(err);
        }

        install_panic_hook();

        Ok(TerminalGuard { _private: () })
    }
}

/// Normal tracking (`?1000`: each button's press and release, and each wheel
/// step) in the SGR encoding (`?1006`, so a column past 223 arrives intact),
/// and nothing else: no motion tracking, no other encoding. Written here
/// rather than through the terminal library's mouse-capture command, which
/// also asks for button-motion, any-motion and urxvt reports.
/// @spec TUI-RUN-060
const MOUSE_REPORTING_ON: &[u8] = b"\x1b[?1000h\x1b[?1006h";

/// The reverse, in the reverse order. @spec TUI-RUN-061
const MOUSE_REPORTING_OFF: &[u8] = b"\x1b[?1006l\x1b[?1000l";

fn write_flushed(bytes: &[u8]) -> io::Result<()> {
    let mut out = io::stdout();
    out.write_all(bytes)?;
    out.flush()
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // @spec TUI-RUN-002
        restore_terminal();
    }
}

/// Turn off mouse reporting, leave the alternate screen, and disable raw
/// mode, in that order. Best-effort: called from
/// both the Drop guard and the panic hook, and a restore that is already
/// failing (terminal gone, stdout closed) has nothing better to fall back to.
fn restore_terminal() {
    if !TAKEN.swap(false, Ordering::SeqCst) {
        return;
    }
    // @spec TUI-RUN-061
    let _ = write_flushed(MOUSE_REPORTING_OFF);
    let _ = crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen);
    let _ = crossterm::terminal::disable_raw_mode();
}

/// Restore the terminal before the default panic hook prints, so the panic
/// message lands on a usable screen instead of inside raw mode / the
/// alternate screen. @spec TUI-RUN-003
fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        restore_terminal();
        default_hook(panic_info);
    }));
}

/// Install SIGTERM/SIGHUP handlers. They only record; the loop takes the
/// ordinary shutdown path, because handler-side work is not async-signal-safe.
/// Ctrl-C is not here — in raw mode the terminal delivers it as a key.
/// @spec TUI-RUN-004
pub fn install_signal_handlers() -> std::io::Result<()> {
    #[cfg(unix)]
    {
        unix_signals::install()
    }
    #[cfg(not(unix))]
    {
        Ok(())
    }
}

/// Whether a termination signal has arrived. @spec TUI-RUN-004
pub fn termination_requested() -> bool {
    TERMINATION_REQUESTED.load(Ordering::SeqCst)
}

#[cfg(unix)]
mod unix_signals {
    use super::{Ordering, TERMINATION_REQUESTED};
    use std::os::raw::c_int;

    const SIGHUP: c_int = 1;
    const SIGTERM: c_int = 15;

    type SigHandler = extern "C" fn(c_int);

    extern "C" {
        // POSIX `signal(2)`. Rust already links libc on every unix target,
        // so no extra crate dependency is needed to reach it.
        fn signal(signum: c_int, handler: SigHandler) -> SigHandler;
    }

    /// The only thing async-signal-safe enough to do here: record that a
    /// termination signal arrived. Everything else — saving, restoring the
    /// terminal — happens later on the event loop's own stack.
    /// @spec TUI-RUN-004
    extern "C" fn record_termination(_signum: c_int) {
        TERMINATION_REQUESTED.store(true, Ordering::SeqCst);
    }

    pub fn install() -> std::io::Result<()> {
        // SIG_ERR is `(void (*)(int))-1`; as a function pointer cast to
        // usize that is usize::MAX on every platform we target.
        const SIG_ERR: usize = usize::MAX;

        // SAFETY: `record_termination` only performs an atomic store, which
        // is safe to run in a signal handler context.
        unsafe {
            if signal(SIGTERM, record_termination) as usize == SIG_ERR {
                return Err(std::io::Error::last_os_error());
            }
            if signal(SIGHUP, record_termination) as usize == SIG_ERR {
                return Err(std::io::Error::last_os_error());
            }
        }

        Ok(())
    }
}
