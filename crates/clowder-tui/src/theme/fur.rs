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

/// The slot whose colour a pattern paints the portrait's fur cell at `row`,
/// `col` in — the theme design's mask, slot 0 wherever it names none.
/// @spec TUI-THEME-028
pub fn fur_slot(pattern: FurPattern, row: usize, col: usize) -> usize {
    let at = |cells: &[(usize, usize)]| cells.contains(&(row, col));
    let muzzle = row == 2 && [3, 4, 6, 7].contains(&col);
    match pattern {
        FurPattern::Solid => 0,
        // The stripes run down the face.
        FurPattern::Tabby => usize::from(col % 2 == 0),
        FurPattern::Tuxedo => usize::from(muzzle),
        // The blaze and the white lower face.
        FurPattern::Bicolour => usize::from((row == 1 && [4, 5, 6].contains(&col)) || row == 2),
        FurPattern::Tortoiseshell => usize::from(at(&[
            (0, 8),
            (1, 1),
            (1, 5),
            (1, 6),
            (1, 9),
            (2, 2),
            (2, 7),
        ])),
        // Warm and dark patches on a white face.
        FurPattern::Calico => {
            if at(&[(0, 2), (1, 1), (1, 2), (1, 6)]) {
                1
            } else if at(&[(0, 8), (1, 8), (1, 9), (2, 8)]) {
                2
            } else {
                0
            }
        }
        // The dark ears and mask.
        FurPattern::Colourpoint => usize::from(row == 0 || muzzle),
        // Colour on the head only.
        FurPattern::Van => usize::from(row == 0 || (row == 1 && [1, 2, 8, 9].contains(&col))),
        // A pale undercoat showing on the lower face.
        FurPattern::Smoke => usize::from(row == 2),
    }
}
