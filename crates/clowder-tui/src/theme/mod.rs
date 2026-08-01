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
        match self {
            Role::Inherited => None,
            Role::Dim => Some(Color::Rgb(0x7C, 0x78, 0x73)),
            Role::Ready => Some(Color::Rgb(0x3E, 0x8E, 0x5A)),
            Role::Partial => Some(Color::Rgb(0xA8, 0x80, 0x2F)),
            Role::Attention => Some(Color::Rgb(0xC0, 0x66, 0x3A)),
            Role::Milestone => Some(Color::Rgb(0x7E, 0x6B, 0xA8)),
        }
    }
}

/// The chrome accent for a season. Confined to inert chrome — the header rule
/// and the pane divider — so no colour is ever ambiguous between a state and a
/// season. @spec TUI-THEME-008, TUI-THEME-009
pub fn seasonal_accent(season: Season) -> Color {
    match season {
        Season::Spring => Color::Rgb(0x7F, 0xA0, 0x5A),
        Season::Summer => Color::Rgb(0xC9, 0xA2, 0x27),
        Season::Autumn => Color::Rgb(0xB5, 0x71, 0x3C),
        Season::Winter => Color::Rgb(0x6E, 0x8B, 0xA8),
    }
}

/// Whether the terminal advertises truecolor. A tmux that has not been
/// configured for RGB approximates every value silently, and the player sees a
/// muddy game they will reasonably assume just looks like that.
/// @spec TUI-THEME-012
pub fn truecolor_advertised() -> bool {
    match std::env::var("COLORTERM") {
        Ok(value) => {
            let value = value.to_ascii_lowercase();
            value == "truecolor" || value == "24bit"
        }
        Err(_) => false,
    }
}
