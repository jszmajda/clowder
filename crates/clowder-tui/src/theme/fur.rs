//! Fur: the one place colour depicts rather than means. A kitten's portrait
//! and its swatches are painted in its true fur and eye colours — outside the
//! mid-luminance band, since they carry nothing the words beside them do not —
//! and nothing else ever wears them. @spec TUI-THEME-027

use clowder_core::{EyeColour, FurColour, FurPattern};
use ratatui::style::Color;

/// A fur colour's painted value, true to life. @spec TUI-THEME-027
pub fn fur_colour(c: FurColour) -> Color {
    let (r, g, b) = match c {
        FurColour::Black => (0x1C, 0x1A, 0x1A),
        FurColour::White => (0xF4, 0xF1, 0xEA),
        FurColour::Grey => (0x8A, 0x8D, 0x93),
        FurColour::Silver => (0xC3, 0xC6, 0xCC),
        FurColour::Brown => (0x7A, 0x55, 0x36),
        FurColour::Chocolate => (0x5E, 0x3A, 0x26),
        FurColour::Ginger => (0xD9, 0x82, 0x2B),
        FurColour::Cream => (0xEE, 0xD9, 0xB0),
        FurColour::Cinnamon => (0xA0, 0x60, 0x3A),
        FurColour::Fawn => (0xD4, 0xB7, 0x9A),
        FurColour::Seal => (0x4A, 0x33, 0x28),
        FurColour::Lilac => (0xB7, 0xA5, 0xB4),
    };
    Color::Rgb(r, g, b)
}

/// An eye colour's painted value. @spec TUI-THEME-027
pub fn eye_colour(c: EyeColour) -> Color {
    let (r, g, b) = match c {
        EyeColour::Green => (0x6D, 0xAF, 0x4F),
        EyeColour::Gold => (0xE3, 0xC0, 0x4A),
        EyeColour::Blue => (0x5B, 0x9B, 0xD5),
        EyeColour::Amber => (0xD6, 0x96, 0x2A),
        EyeColour::Copper => (0xB8, 0x64, 0x2E),
        EyeColour::Hazel => (0x9C, 0x8A, 0x3E),
    };
    Color::Rgb(r, g, b)
}

/// Where on the cat a portrait's fur cell sits, as the render leaf labels
/// every fur cell of every pose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FurPart {
    /// The shoulders, or the back nearest the head.
    Top,
    Back,
    Chest,
    /// The belly and lower body.
    Under,
    Paw,
    Tail,
}

/// The two sets of cells a patched coat's second and third colours fall on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Patch {
    A,
    B,
}

/// The slot whose colour a pattern paints a fur cell in, by the cell's part,
/// its patch, and its column — the theme design's rule, slot 0 wherever it
/// names none, so one rule paints a pattern in every pose.
/// @spec TUI-THEME-028
pub fn fur_slot(pattern: FurPattern, part: FurPart, patch: Option<Patch>, col: usize) -> usize {
    use FurPart::*;
    let on = |yes: bool| usize::from(yes);
    match pattern {
        FurPattern::Solid => 0,
        // The stripes run across the body.
        FurPattern::Tabby => on(col % 2 == 0),
        // The white bib and socks.
        FurPattern::Tuxedo => on(matches!(part, Chest | Paw)),
        // The white front and belly.
        FurPattern::Bicolour => on(matches!(part, Chest | Under | Paw)),
        FurPattern::Tortoiseshell => on(patch == Some(Patch::A)),
        // Warm and dark patches on white.
        FurPattern::Calico => match patch {
            Some(Patch::A) => 1,
            Some(Patch::B) => 2,
            None => 0,
        },
        // The dark points.
        FurPattern::Colourpoint => on(matches!(part, Paw | Tail)),
        // Colour near the head and on the tail.
        FurPattern::Van => on(matches!(part, Top | Tail)),
        // The pale undercoat showing beneath.
        FurPattern::Smoke => on(part == Under),
    }
}
