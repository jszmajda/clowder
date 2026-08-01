//! Transients: the event ticker, the Journal, help, the find palette, and the
//! returning player's report banner.
//!
//! Nothing here blocks. The sim keeps ticking and the chrome keeps rendering
//! behind every one of them — an overlay that stopped the world would be an
//! obligation, and this game does not issue those.
//! @spec TUI-NAV-026, TUI-NAV-027, TUI-NAV-038, TUI-NAV-050, TUI-NAV-055, TUI-NAV-056, TUI-NAV-057

use clowder_core::GameState;
use ratatui::{layout::Rect, Frame};

use crate::nav::ViewState;

/// The dim one-line ticker. Flavour is quieter than mechanical; milestones are
/// marked. Prominence comes from the core's classification, never from this
/// module deciding which events matter. @spec TUI-NAV-026, TUI-NAV-057
pub fn ticker(_f: &mut Frame, _area: Rect, _state: &GameState) {
    todo!("TUI-NAV-026/057: the event ticker")
}

/// Full scrollback over the retained journal, grouped by day.
/// @spec TUI-NAV-026
pub fn journal(_f: &mut Frame, _area: Rect, _state: &GameState, _view: &ViewState) {
    todo!("TUI-NAV-026: the Journal overlay")
}

/// The complete key map and glyph legend. There is no manual; this and the
/// hint bar are the manual. @spec TUI-NAV-025
pub fn help(_f: &mut Frame, _area: Rect) {
    todo!("TUI-NAV-025: the help overlay")
}

/// Find-anything over revealed content, narrowing by substring as characters
/// arrive. @spec TUI-NAV-038
pub fn palette(_f: &mut Frame, _area: Rect, _state: &GameState, _view: &ViewState) {
    todo!("TUI-NAV-038: the find palette")
}

/// The returning player's one-line banner: elapsed span, the largest gains, and
/// arrivals. Departures belong to the pinned entry — the first thing seen on
/// return leads with what grew.
/// @spec TUI-NAV-027, TUI-NAV-050, TUI-NAV-055, TUI-NAV-056
pub fn banner(_f: &mut Frame, _area: Rect, _state: &GameState, _view: &ViewState) {
    todo!("TUI-NAV-027/050/055/056: the away-report banner")
}
