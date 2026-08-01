//! Truth, one keypress away — and none at all when the pane fits.
//!
//! The inspector is where whimsy pays for itself: the row shows a flavour name,
//! and this states the plain mechanical fact behind it. A player must never
//! lose a decision because a name was cute.
//! @spec TUI-NAV-017, TUI-NAV-018, TUI-NAV-029, TUI-NAV-047, TUI-NAV-048

use clowder_core::GameState;
use ratatui::{layout::Rect, Frame};

use crate::nav::ViewState;

/// Flavour name, plain description, exact next cost with have/need per
/// component, time-to-afford at the current net rate, owned count and effects,
/// and — for anything not actionable — the plain reason, taken from the core
/// rather than decided here. @spec TUI-NAV-018
pub fn draw(_f: &mut Frame, _area: Rect, _state: &GameState, _view: &ViewState) {
    todo!("TUI-NAV-018: the inspector's contents")
}

/// Whether the inspector shows as a persistent pane, swaps in over the list, or
/// is absent — a pure function of width and whether the player asked for it, so
/// it cannot disagree with the layout after a resize.
/// @spec TUI-NAV-017, TUI-NAV-029
pub fn placement(_width: u16, _wanted: bool) -> Placement {
    todo!("TUI-NAV-017: pane above the breakpoint, swap below")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    Pane,
    Swapped,
    Absent,
}
