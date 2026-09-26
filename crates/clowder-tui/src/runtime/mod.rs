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
    /// Whether a save failure has already been reported this session, so the
    /// notice is said once rather than every cadence. @spec TUI-RUN-028
    pub save_failed: bool,
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

    // Said before the handover, so it survives on the player's own scrollback
    // instead of being wiped with the alternate screen. Rendering is unchanged
    // — there is no reduced palette to fall back to, by design. Without this,
    // a tmux that was never configured for RGB approximates every value and
    // the player reasonably concludes the game just looks like that.
    // @spec TUI-THEME-012, TUI-THEME-013
    if !crate::theme::truecolor_advertised() {
        eprintln!(
            "This terminal does not advertise 24-bit colour, so clowder's palette \n\
             will be approximated to the nearest colours it does support.\n\
             Set COLORTERM=truecolor (and, under tmux, a terminal-overrides entry \n\
             adding the RGB capability) to see it as designed."
        );
    }

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

    let mut view = ViewState::default();
    // TUI-NAV-024: the cold open is one section with one focused row. Without
    // this the first thing a player presses does nothing, and the hint bar's
    // "Enter do" is a promise the game does not keep.
    crate::nav::update(
        crate::nav::Action::SectionJump(crate::nav::Section::Meadow),
        &mut view,
        &state,
    );

    let mut app = App {
        state,
        view,
        clock: TickAccumulator::default(),
        save_path,
        last_save: Instant::now(),
        prev_layout: None,
        quitting: false,
        save_failed: false,
    };

    // Anything said from in here would be printed onto the alternate screen and
    // then wiped out by the restore that follows. Collect instead, and speak
    // once the player's own terminal is back.
    let mut parting = Vec::new();
    {
        let _guard = _guard;
        let backend = ratatui::backend::CrosstermBackend::new(std::io::stdout());
        let mut terminal = match ratatui::Terminal::new(backend) {
            Ok(t) => Some(t),
            Err(e) => {
                parting.push(format!("clowder could not start drawing: {e}"));
                app.quitting = true;
                None
            }
        };

        // The only place real clocks are read. Both are sampled together so
        // their difference is meaningful: the wall clock outrunning the
        // monotonic one is the machine having been suspended.
        // @spec TUI-RUN-021, TUI-RUN-022
        let mut last_mono = Instant::now();
        let mut last_wall = std::time::SystemTime::now();

        while !app.quitting && !term::termination_requested() {
            let now_mono = Instant::now();
            let now_wall = std::time::SystemTime::now();
            let reading = clock::Reading {
                monotonic: now_mono.duration_since(last_mono),
                // A backwards correction reads as zero elapsed here; the
                // accumulator keeps the monotonic figure. @spec TUI-RUN-023
                wall: now_wall.duration_since(last_wall).unwrap_or_default(),
            };
            last_mono = now_mono;
            last_wall = now_wall;

            // The poll timeout IS the frame cadence; nothing to do means we
            // wait here. Bounded so a flood cannot starve the frame.
            let mut input = Vec::new();
            let mut waited = false;
            while input.len() < App::MAX_INPUT_EVENTS_PER_FRAME {
                let timeout =
                    if waited { std::time::Duration::ZERO } else { App::FRAME_BUDGET };
                waited = true;
                match crossterm::event::poll(timeout) {
                    Ok(true) => match crossterm::event::read() {
                        Ok(ev) => input.push(ev),
                        Err(e) => {
                            parting.push(format!("clowder lost its input: {e}"));
                            break;
                        }
                    },
                    Ok(false) => break,
                    Err(e) => {
                        parting.push(format!("clowder lost its input: {e}"));
                        break;
                    }
                }
            }

            let Some(t) = terminal.as_mut() else { break };
            if let Err(e) = app.step(t, reading, &input) {
                parting.push(format!("clowder stopped: {e}"));
                break;
            }
        }
        // Every clean shutdown saves, and does not drain the backlog first: the
        // banked count goes into the save and the next load runs it, so
        // quitting mid-catch-up costs nothing and waits for nothing.
        // A failure does not change the
        // exit code — the session is ending either way, and the player is better
        // served by being told than by a status nobody reads.
        // @spec TUI-RUN-026, TUI-RUN-031
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
    /// How long a frame waits for input before going round again. The poll
    /// timeout IS the frame cadence: nothing to do means we wait here.
    const FRAME_BUDGET: std::time::Duration = std::time::Duration::from_millis(60);
    /// TUI-RUN-016. A paste or a held key can deliver faster than frames
    /// retire; past this many the rest waits for the next frame, so a flood
    /// can never starve the simulation or the renderer.
    const MAX_INPUT_EVENTS_PER_FRAME: usize = 64;
    /// TUI-RUN-025. A long catch-up drains across frames rather than blocking
    /// in one. The count, not a wall-time budget: the accumulator cannot know
    /// what a tick costs, so the caller chooses.
    const MAX_TICKS_PER_FRAME: u64 = 2_000;

    /// One iteration: poll input, apply commands, advance the sim, autosave if
    /// due, render — in that order.
    ///
    /// Generic over the backend so the ordering can be tested against an
    /// in-memory terminal; `reading` is passed in for the same reason, and
    /// because `run` is the only thing that should be reading real clocks.
    /// @spec TUI-RUN-014, TUI-RUN-015, TUI-RUN-016, TUI-RUN-017, TUI-RUN-026
    pub fn step<B: ratatui::backend::Backend>(
        &mut self,
        terminal: &mut ratatui::Terminal<B>,
        reading: clock::Reading,
        input: &[crossterm::event::Event],
    ) -> std::io::Result<()> {
        // The speed that priced this frame's interval is the one in force
        // while it elapsed — captured before any command can change it. A
        // SetSpeed arriving in this frame's input applies from here on, not
        // retroactively: the interval below predates the keypress. Without
        // this the whole of a post-suspend catch-up could be re-priced by a
        // speed change queued in the first frame after waking, which is the
        // very loss the save seam was fixed to prevent.
        // @spec TUI-RUN-020
        let speed_for_this_interval = self.state.speed;

        // ---- 1 & 2. input, bounded; commands land BEFORE this frame's ticks --
        // Events arrive as an argument rather than being polled here, for the
        // same reason the clock reading does: a step that reached for a real
        // terminal could only ever be tested against one. `run` does the
        // waiting and the reading; this decides what they mean.
        //
        // A key pressed during that wait is chronologically older than the time
        // the wait represents, so it takes effect at the time it was pressed
        // rather than after the elapsed interval — and never has to sit out a
        // catch-up burst. @spec TUI-RUN-014, TUI-RUN-015, TUI-RUN-016
        let mut last_resize = None;
        for event in input.iter().take(Self::MAX_INPUT_EVENTS_PER_FRAME) {
            match event {
                crossterm::event::Event::Key(key) => {
                    let action = crate::nav::keys::resolve(*key, &self.view);
                    match crate::nav::update(action, &mut self.view, &self.state) {
                        crate::nav::Outcome::Handled => {}
                        crate::nav::Outcome::Quit => self.quitting = true,
                        crate::nav::Outcome::Apply(cmd) => {
                            if let Err(e) = self.state.apply(cmd) {
                                self.view.footer = Some((
                                    crate::nav::plain_error(&e),
                                    Instant::now() + std::time::Duration::from_secs(4),
                                ));
                            }
                        }
                        crate::nav::Outcome::Repeat(cmd, n) => {
                            // Partial success is intended: take as many as you
                            // can afford, stop at the first refusal. A run
                            // rejected reads as Enter's rejection would; done
                            // any calls first, the footer also says how many
                            // — and, for a player-typed ×N, of how many, never
                            // for an X run against the shell-side cap, which
                            // is not a count the player asked for.
                            // @spec TUI-NAV-069, TUI-NAV-012
                            let mut done = 0u32;
                            let mut rejection = None;
                            for _ in 0..n {
                                match self.state.apply(cmd) {
                                    Ok(_) => done += 1,
                                    Err(e) => {
                                        rejection = Some(e);
                                        break;
                                    }
                                }
                            }
                            if let Some(e) = rejection {
                                let why = crate::nav::plain_error(&e);
                                let message = if done == 0 {
                                    why
                                } else if n == crate::nav::update::MAX_AFFORDABLE_CAP {
                                    format!("{why} — {done} done")
                                } else {
                                    format!("{why} — {done} of {n} done")
                                };
                                self.view.footer =
                                    Some((message, Instant::now() + std::time::Duration::from_secs(4)));
                            }
                        }
                    }
                }
                // Only the final size matters; recomputing layout for every
                // intermediate width of a drag is wasted work. @spec TUI-RUN-016
                crossterm::event::Event::Resize(w, h) => last_resize = Some((*w, *h)),
                _ => {}
            }
        }
        if let Some((w, h)) = last_resize {
            terminal.resize(ratatui::layout::Rect::new(0, 0, w, h))?;
        }

        // ---- 3. advance the simulation --------------------------------------
        // Unconditional, and before any drawing: a terminal too small to draw,
        // a pane nobody is looking at, a frame with nothing to redraw — none of
        // them stop the clock. Time never depends on what is on screen.
        // @spec TUI-RUN-017, TUI-RUN-025
        self.clock.observe(reading, speed_for_this_interval);
        let owed = self.clock.take(Self::MAX_TICKS_PER_FRAME);
        for _ in 0..owed {
            self.state.tick();
        }

        // A tick (or a command this frame applied) can remove a row an
        // arming stands on; that ending is checked every frame, independent
        // of input, so it takes effect the moment the row leaves rather than
        // waiting for a key that may never come. @spec TUI-NAV-076
        crate::nav::expire_unlisted_arming(&mut self.view, &self.state);

        // ---- 4. autosave ----------------------------------------------------
        // @spec TUI-RUN-026, TUI-RUN-028
        if persist::autosave_due(self.last_save.elapsed()) {
            match self.save() {
                Ok(()) => self.save_failed = false,
                Err(e) => {
                    // Reported once, then play continues and the next cadence
                    // tries again. Ending a healthy session over a transient
                    // disk problem destroys more than the failed write did.
                    if !self.save_failed {
                        self.save_failed = true;
                        self.view.footer = Some((
                            format!("could not save: {e}"),
                            Instant::now() + std::time::Duration::from_secs(6),
                        ));
                    }
                }
            }
            self.last_save = Instant::now();
        }

        // ---- 5. render ------------------------------------------------------
        // @spec TUI-RUN-014
        let mut used = None;
        terminal.draw(|f| {
            used = Some(crate::render::frame(f, &self.state, &self.view, self.prev_layout.as_ref()));
        })?;
        self.prev_layout = used;
        Ok(())
    }

    /// Save at the present instant, carrying the ticks banked but not yet run
    /// as an explicit count. @spec TUI-RUN-027, TUI-RUN-031, SAVE-017
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
