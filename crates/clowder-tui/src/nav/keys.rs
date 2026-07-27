//! The key map. Only the tmux-safe set is ever bound: plain printable
//! characters, Tab/BackTab, Enter, Esc, Space, Backspace, and the arrows, plus
//! Ctrl-C as emergency quit. Uppercase is the modifier tier — Shift emits
//! plain printable characters, which tmux never intercepts.
//! @spec TUI-NAV-001, TUI-NAV-004

use crossterm::event::KeyEvent;

use super::action::Action;
use super::view::ViewState;

/// Resolve a key to an action in the current context. Context matters: a digit
/// jumps sections on a bare screen and types into an open prompt.
/// @spec TUI-NAV-037, TUI-NAV-052
pub fn resolve(_key: KeyEvent, _view: &ViewState) -> Action {
    todo!("TUI-NAV-001..004, 037, 052: the context-aware key map")
}

/// Whether a key is inside the tmux-safe set at all. Nothing outside it is
/// ever bound. @spec TUI-NAV-001
pub fn is_tmux_safe(_key: KeyEvent) -> bool {
    todo!("TUI-NAV-001: the admissible key set")
}
