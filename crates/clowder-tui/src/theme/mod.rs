//! Colour, and what it is allowed to mean.
//!
//! Colour encodes *state*, never identity: names carry identity for free, and
//! spending the palette on fifty items would leave nothing meaning anything
//! when something actually needs the player. Every meaning colour carries is
//! carried first by a glyph, a word, or a fill — turn all colour off and the
//! game is still completely playable. @spec TUI-THEME-003, TUI-THEME-010

use clowder_core::Season;
use ratatui::style::Color;

/// The six roles. There is no seventh: every added colour dilutes the signal
/// that colour *is* meaning. @spec TUI-THEME-004
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// The terminal's own foreground. Ordinary text takes no colour at all.
    Inherited,
    /// Revealed but not actionable, owned and done, flavour, chrome rules.
    /// @spec TUI-THEME-014
    Dim,
    /// Affordable now.
    Ready,
    /// Partially affordable.
    Partial,
    /// Wants notice — a chip, a capped store, an armed row. @spec TUI-THEME-015
    Attention,
    /// A moment worth remembering, and newly revealed content.
    Milestone,
}

impl Role {
    /// `None` means "use the terminal's own foreground" — clowder paints no
    /// ground, so light and dark both work with nothing detected.
    /// @spec TUI-THEME-001, TUI-THEME-002
    pub fn color(self) -> Option<Color> {
        todo!("TUI-THEME-002/004: the mid-luminance palette")
    }
}

/// The chrome accent for a season. Confined to inert chrome — the header rule
/// and the pane divider — so no colour is ever ambiguous between a state and a
/// season. @spec TUI-THEME-008, TUI-THEME-009
pub fn seasonal_accent(_season: Season) -> Color {
    todo!("TUI-THEME-008: the four seasonal accents")
}

/// Whether the terminal advertises truecolor. A tmux that has not been
/// configured for RGB approximates every value silently, and the player sees a
/// muddy game they will reasonably assume just looks like that.
/// @spec TUI-THEME-012
pub fn truecolor_advertised() -> bool {
    todo!("TUI-THEME-012: detect advertised truecolor support")
}
