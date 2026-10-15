//! Waking up — a load's catch-up replayed after the terminal is taken, a
//! bounded count per frame, behind the waking screen, until the game opens.
//!
//! Mirrors `App::step`'s shape for the same reason: events and the clock
//! reading arrive as arguments rather than being read here, so a wake runs
//! headless in tests exactly as play does. @spec TUI-RUN-033..039,
//! TUI-NAV-102, TUI-NAV-103

use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use clowder_core::{Advance, CatchUp};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::Backend;
use ratatui::layout::Rect;
use ratatui::Terminal;

use crate::nav;
use crate::render;
use crate::runtime::clock::Reading;
use crate::runtime::{App, TickAccumulator, BRIEFEST_WAIT};

/// Waking's own per-frame bound: larger than live play's, since the waking
/// screen has only its progress and the quit key to keep responsive.
/// @spec TUI-RUN-033
pub const WAKE_TICKS_PER_FRAME: u64 = 50_000;

/// Whether a load is still waking through its catch-up, or has opened.
///
/// The size gap is inherent to advancing one frame at a time (TUI-RUN-033) —
/// boxing `Waking` would cost every frame of a wake an allocation to save
/// space a few-frames-long wake never notices.
#[allow(clippy::large_enum_variant)]
pub enum Wake {
    Waking(Waking),
    Awake(App),
}

/// A load's catch-up in progress behind the waking screen.
pub struct Waking {
    catch_up: CatchUp,
    pub clock: TickAccumulator,
    pub save_path: PathBuf,
    pub last_save: Instant,
    pub quitting: bool,
    bound: u64,
}

/// A waking frame that could not be written to the terminal, handing back the
/// wake as that frame left it — the bound it replayed before its draw kept —
/// so the shutdown that follows can save it. @spec TUI-RUN-047, TUI-RUN-048
pub struct Refused {
    pub error: io::Error,
    pub waking: Waking,
}

impl std::fmt::Debug for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Refused")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

/// A key that answers quitting, and only quitting, while waking: `q` (plain)
/// and Ctrl-C, exactly as everywhere (TUI-NAV-016). @spec TUI-NAV-103
fn is_quit_key(key: KeyEvent) -> bool {
    (key.code == KeyCode::Char('q') && key.modifiers == KeyModifiers::NONE)
        || (key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL)
}

impl Wake {
    /// Begin a wake at waking's own bound. @spec TUI-RUN-033
    pub fn begin<B: Backend>(
        catch_up: CatchUp,
        save_path: PathBuf,
        terminal: &mut Terminal<B>,
    ) -> Result<Wake, Box<Refused>> {
        Self::begin_bounded(catch_up, save_path, terminal, WAKE_TICKS_PER_FRAME)
    }

    /// As [`Wake::begin`], at a caller-chosen bound — the production entry
    /// point is `begin` at [`WAKE_TICKS_PER_FRAME`]; tests use a smaller bound
    /// so a wake of a few frames costs a few hundred ticks. @spec TUI-RUN-033
    pub fn begin_bounded<B: Backend>(
        catch_up: CatchUp,
        save_path: PathBuf,
        terminal: &mut Terminal<B>,
        bound: u64,
    ) -> Result<Wake, Box<Refused>> {
        let waking = Waking {
            catch_up,
            clock: TickAccumulator::default(),
            save_path,
            last_save: Instant::now(),
            quitting: false,
            bound,
        };
        waking.advance_and_draw(terminal)
    }
}

impl Waking {
    pub fn total_ticks(&self) -> u64 {
        self.catch_up.total_ticks()
    }

    pub fn remaining_ticks(&self) -> u64 {
        self.catch_up.remaining_ticks()
    }

    /// No waiting while waking: the replay runs at the machine's pace, with
    /// frames as its only pauses. The check for input is the briefest wait
    /// the input source honours, never none. @spec TUI-RUN-035, TUI-RUN-059
    pub fn poll_timeout(&self) -> Duration {
        BRIEFEST_WAIT
    }

    /// One waking frame: quitting answers alone, every other key is
    /// discarded, the real time passing is banked at the save's speed through
    /// the ordinary accumulator (no live tick runs), and the catch-up
    /// advances by at most this wake's bound. @spec TUI-RUN-035, TUI-RUN-036,
    /// TUI-NAV-103
    pub fn step<B: Backend>(
        mut self,
        terminal: &mut Terminal<B>,
        reading: Reading,
        input: &[Event],
    ) -> Result<Wake, Box<Refused>> {
        for event in input {
            match event {
                Event::Key(key) if is_quit_key(*key) => self.quitting = true,
                // Only the final size matters, as for the awake loop.
                Event::Resize(w, h) => {
                    if let Err(error) = terminal.resize(Rect::new(0, 0, *w, *h)) {
                        return Err(Box::new(Refused {
                            error,
                            waking: self,
                        }));
                    }
                }
                // Every other key is discarded, not held: it would otherwise
                // act on a meadow the player has not yet seen.
                _ => {}
            }
        }

        // TUI-RUN-036: no live tick runs while waking. Real time keeps
        // passing, and the ordinary accumulator observes it at the save's
        // speed — suspend excess and the absence cap included — banking it
        // to run as live ticks once the game opens.
        let speed = self.catch_up.speed();
        self.clock.observe(reading, speed);

        self.advance_and_draw(terminal)
    }

    /// Shutdown taken while waking: set the catch-up down and save promptly,
    /// the owed count carrying the catch-up's remaining ticks together with
    /// any live ticks banked while waking, and the save carrying the core's
    /// report so far. @spec TUI-RUN-039
    pub fn shut_down(self) -> io::Result<()> {
        let sd = self.catch_up.set_down();
        let owed = sd.owed_ticks + self.clock.backlog();
        clowder_core::write_save_carrying(
            &self.save_path,
            &sd.state,
            crate::runtime::now_unix(),
            owed,
            Some(&sd.report_so_far),
        )
    }

    /// Advance this wake's bound. Where that finishes the catch-up, open the
    /// game against exactly the state a one-call load would have produced —
    /// no live tick in that frame, the opening taken and the load's report
    /// queued, and the time banked while waking carried forward to run once
    /// play begins (TUI-RUN-038). Otherwise draw the waking screen having
    /// replayed no more than this bound, and come back pending.
    /// @spec TUI-RUN-033, TUI-RUN-034, TUI-RUN-037, TUI-RUN-038
    fn advance_and_draw<B: Backend>(
        mut self,
        terminal: &mut Terminal<B>,
    ) -> Result<Wake, Box<Refused>> {
        let bound = self.bound;
        match self.catch_up.advance(bound) {
            Advance::Finished(loaded) => {
                let mut view = nav::ViewState::default();
                // @spec TUI-NAV-024
                nav::opening(&mut view, &loaded.state);
                // @spec TUI-NAV-027
                nav::queue_load_report(&mut view, &loaded);
                Ok(Wake::Awake(App {
                    state: loaded.state,
                    view,
                    clock: self.clock,
                    save_path: self.save_path,
                    last_save: self.last_save,
                    prev_layout: None,
                    pointer_record: None,
                    quitting: self.quitting,
                    save_failed: false,
                }))
            }
            Advance::Pending(next) => {
                self.catch_up = next;
                // @spec TUI-NAV-102
                // @spec TUI-RUN-047, TUI-RUN-048
                let drawn = terminal.draw(|f| {
                    render::waking(
                        f,
                        self.catch_up.total_ticks(),
                        self.catch_up.remaining_ticks(),
                    )
                });
                match drawn {
                    Ok(_) => Ok(Wake::Waking(self)),
                    Err(error) => Err(Box::new(Refused {
                        error,
                        waking: self,
                    })),
                }
            }
        }
    }
}
