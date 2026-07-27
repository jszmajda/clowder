//! The reducer: an action plus current state becomes a view change, a core
//! command, or both.
//!
//! Key resolution is deliberately state-light — it answers "what did the
//! player ask for", not "what happens". This is where the two meet, because
//! the answer depends on the focused row, and rows come from game state.
//! @spec TUI-NAV-002, TUI-NAV-011, TUI-NAV-015, TUI-NAV-041, TUI-NAV-051

use clowder_core::{Command, GameState};

use super::action::Action;
use super::view::ViewState;

/// What the loop should do with an action once the view has absorbed it.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// Nothing further; the view changed (or deliberately did not).
    Handled,
    /// Apply this to the core, once.
    Apply(Command),
    /// Apply this to the core up to `n` times, stopping at the first
    /// rejection — partial success is intended. @spec TUI-NAV-031
    Repeat(Command, u32),
    /// Save and exit. @spec TUI-NAV-016
    Quit,
}

/// Fold an action into the view, yielding whatever the core must be told.
///
/// Heavy rows never reach `Apply` on a first Enter — they arm, and only a
/// distinct second Enter commits. @spec TUI-NAV-015, TUI-NAV-051
pub fn update(_action: Action, _view: &mut ViewState, _state: &GameState) -> Outcome {
    todo!("TUI-NAV-002/011/015/041/051: fold action into view, emit commands")
}
