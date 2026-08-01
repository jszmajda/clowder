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
    // TUI-RUN-009. The refusal below tells the player about --force, so the
    // binary had better honour it.
    let force = std::env::args().any(|a| a == "--force");
    let save_path = persist::resolve_save_path();

    // TUI-RUN-029: the lock comes before the save is read. Acquiring it after
    // would leave a window in which two launches both read the same save and
    // both believe they are alone. Every refusal below returns while `_lock`
    // is still in scope, so its Drop releases it — a refusal never leaves a
    // lock behind for the next launch to inherit.
    let _lock = match SessionLock::acquire(&save_path, force) {
        Ok(l) => l,
        Err(lock::LockError::Held { pid }) => {
            eprintln!(
                "clowder is already open in another session (process {pid}).\n\
                 Only one session may hold a meadow at a time.\n\
                 If that session is gone, start again with --force to take the lock."
            );
            return ExitCode::FAILURE;
        }
        Err(lock::LockError::Io(e)) => {
            eprintln!("clowder could not take its session lock beside {}: {e}", save_path.display());
            return ExitCode::FAILURE;
        }
    };

    // TUI-RUN-013 vs TUI-RUN-012: read the bytes here rather than letting the
    // core's reader decide, because it reports *every* non-missing IO failure
    // as a corrupt save — and the policy for corrupt is to move the file
    // aside. A permission error or a directory in the save's place would then
    // destroy a save whose contents were never even seen. Unreadable is not
    // unparseable: if we could not read it, we do not touch it.
    let contents = match std::fs::read(&save_path) {
        // Bytes we could read but not decode are *corrupt*, not unreadable —
        // the contents are known, which is the case the rescue policy exists
        // for. Refusing here would leave the player hard-stuck on exactly the
        // damage TUI-RUN-012 promises to preserve.
        Ok(bytes) => Some(String::from_utf8_lossy(&bytes).into_owned()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            eprintln!(
                "clowder could not read its save at {}: {e}\n\
                 The file has been left exactly as it is.",
                save_path.display()
            );
            return ExitCode::FAILURE;
        }
    };

    let now_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let state = match contents {
        // TUI-RUN-010: an ordinary first run, not an error.
        None => GameState::new(now_unix),
        Some(c) => match clowder_core::load_string(&c, now_unix) {
            Ok(loaded) => loaded.state,
            // TUI-RUN-011: never guess forward over a save that is currently fine.
            Err(clowder_core::LoadError::TooNew { found }) => {
                eprintln!(
                    "This meadow was saved by a newer clowder (save version {found}).\n\
                     Opening it with this one could damage it, so it has been left alone.\n\
                     Update clowder, or move {} aside yourself.",
                    save_path.display()
                );
                return ExitCode::FAILURE;
            }
            // TUI-RUN-012: readable but not parseable. Move it aside — never
            // over-write it. Saves are human-readable precisely so a broken one
            // can be rescued by hand, and replacing it destroys the only thing
            // that made that promise real.
            Err(clowder_core::LoadError::Corrupt { detail }) => {
                match persist::set_aside(&save_path) {
                    Ok(moved) => {
                        eprintln!(
                            "clowder could not make sense of its save ({detail}).\n\
                             Your old save is safe at {} — nothing was overwritten.\n\
                             Starting a new meadow.",
                            moved.display()
                        );
                        GameState::new(now_unix)
                    }
                    Err(e) => {
                        eprintln!(
                            "clowder could not make sense of its save ({detail}), and could \
                             not move it aside either ({e}).\n\
                             Nothing has been changed."
                        );
                        return ExitCode::FAILURE;
                    }
                }
            }
            Err(clowder_core::LoadError::NotFound) => GameState::new(now_unix),
        },
    };

    // Past every refusal: the terminal may now be taken over (TUI-RUN-001).
    let _guard = match TerminalGuard::enter() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("clowder could not prepare the terminal: {e}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(e) = term::install_signal_handlers() {
        eprintln!("clowder could not install its signal handlers: {e}");
        return ExitCode::FAILURE;
    }

    let mut app = App {
        state,
        view: ViewState::default(),
        clock: TickAccumulator::default(),
        save_path,
        last_save: Instant::now(),
        prev_layout: None,
        quitting: false,
    };

    // Anything said from in here would be printed onto the alternate screen and
    // then wiped out by the restore that follows. Collect instead, and speak
    // once the player's own terminal is back.
    let mut parting = Vec::new();
    {
        let _guard = _guard;
        while !app.quitting && !term::termination_requested() {
            if let Err(e) = app.step() {
                parting.push(format!("clowder stopped: {e}"));
                break;
            }
        }
        // TUI-RUN-026: every clean shutdown saves. A failure does not change the
        // exit code — the session is ending either way, and the player is better
        // served by being told than by a status nobody reads.
        if let Err(e) = app.save() {
            parting.push(format!("clowder could not save on the way out: {e}"));
        }
    }
    for line in parting {
        eprintln!("{line}");
    }
    ExitCode::SUCCESS
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
        // The stamp is simply now, and the ticks this session banked but never
        // ran are written alongside it as a count.
        //
        // Back-dating the stamp so the interval re-derives them was the older
        // scheme, and it quietly re-priced the backlog at whatever speed was
        // set at save time: ticks earned fast and saved slow were lost, ticks
        // earned slow and saved fast were conjured, by the ratio between the
        // two speeds. What the player earned is theirs, so it is stored as
        // what it is. @spec TUI-RUN-027, SAVE-017
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        clowder_core::write_save_owing(&self.save_path, &self.state, now, self.clock.backlog())
    }
}
