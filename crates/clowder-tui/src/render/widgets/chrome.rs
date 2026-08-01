//! The persistent furniture: header, tab bar, resource sidebar, hint bar.
//!
//! This is what the two-second glance contract is made of. Its whole job is to
//! be quiet: an empty attention cluster means nothing needs you, and that only
//! works if the eye has learned that something appearing there is information.
//! @spec TUI-NAV-019..025, TUI-NAV-034, TUI-NAV-044, TUI-NAV-045, TUI-NAV-054

use clowder_core::GameState;
use ratatui::{layout::Rect, Frame};

use crate::nav::ViewState;

/// Clowder name and generation, date, kitten count, mood as a word, speed, and
/// the right-aligned attention chips. @spec TUI-NAV-019, TUI-NAV-020
pub fn header(_f: &mut Frame, _area: Rect, _state: &GameState, _view: &ViewState) {
    todo!("TUI-NAV-019/020/021/022/044/054: header and the attention chips")
}

/// Revealed sections with their digits; a badge on one not yet visited.
/// Width-responsive: full labels where they fit, else a compact digit strip —
/// never a name truncated mid-word, never a tab scrolled out of reach.
/// @spec TUI-NAV-006, TUI-NAV-034
pub fn tabs(_f: &mut Frame, _area: Rect, _state: &GameState, _view: &ViewState) {
    todo!("TUI-NAV-006/034: the responsive tab bar")
}

/// A curated glance: revealed resources with stock, a fill bar, and a signed
/// rate. Display-only — the complete table is a list in the home section.
/// @spec TUI-NAV-019, TUI-NAV-022, TUI-NAV-049
pub fn sidebar(_f: &mut Frame, _area: Rect, _state: &GameState) {
    todo!("TUI-NAV-019/022: the resource sidebar")
}

/// Context keys, most-relevant-first, always ending with the help key. Drops
/// the least relevant first when the width runs out.
/// @spec TUI-NAV-023, TUI-NAV-024, TUI-NAV-045, TUI-NAV-046
pub fn hints(_f: &mut Frame, _area: Rect, _state: &GameState, _view: &ViewState) {
    todo!("TUI-NAV-023/024/045/046: the adaptive hint bar")
}
