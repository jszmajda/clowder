//! Numbers and names, fitted to the space available.
//!
//! Compact where glanced at, exact where read: every abbreviated number on
//! screen has an exact form one keypress away in the inspector, which is what
//! keeps whimsy from costing clarity. @spec TUI-RENDER-012, TUI-RENDER-013

/// A stock or cost for a glanceable surface: compact, to a fixed
/// significant-figure width so columns stay aligned, with precision scaled to
/// magnitude so nothing carries decimals it has not earned.
/// @spec TUI-RENDER-012, TUI-RENDER-013
pub fn compact(_value: f64) -> String {
    todo!("TUI-RENDER-012/013: compact form, precision by magnitude")
}

/// The same value unabbreviated, for the inspector. @spec TUI-RENDER-012
pub fn exact(_value: f64) -> String {
    todo!("TUI-RENDER-012: exact, unabbreviated form")
}

/// A signed per-second rate for the sidebar.
pub fn rate(_per_second: f64) -> String {
    todo!("TUI-RENDER-012/013: signed rate, precision by magnitude")
}

/// Shorten a name to fit, with an explicit ellipsis so the player can see that
/// text was cut. The full name stays available in the inspector.
/// @spec TUI-RENDER-014
pub fn fit(_name: &str, _width: usize) -> String {
    todo!("TUI-RENDER-014: truncate with a visible ellipsis")
}
