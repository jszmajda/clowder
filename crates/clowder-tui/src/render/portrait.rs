//! A kitten's portrait: a small face painted in its own fur, with the fur's
//! and eyes' words beside it where they fit, and the strip of its colours
//! beneath. @spec TUI-RENDER-029

use clowder_core::{content, fur_words, Profile};
use ratatui::style::Style;
use ratatui::text::{Line, Span};

use crate::theme::fur::{eye_colour, fur_colour, fur_slot};

/// A blank that wrapping never trims or breaks at, so the portrait keeps its
/// shape and a swatch stays with its name.
const BLANK: char = '\u{a0}';

/// The face, four rows by eleven columns: `█` is fur, `o` an eye, every other
/// mark outline (the `^` the nose), a space a blank cell.
const ART: [&str; 4] = [
    " /\u{2588}\\___/\u{2588}\\ ",
    "(\u{2588}\u{2588}o\u{2588}\u{2588}\u{2588}o\u{2588}\u{2588})",
    "=\\\u{2588}\u{2588}\u{2588}^\u{2588}\u{2588}\u{2588}/=",
    "  \\_____/  ",
];
const ART_WIDTH: usize = 11;
/// Blank columns between the face and the words beside it.
const GAP: usize = 3;

/// The portrait's lines for an inspector `width` columns wide: the four rows
/// of the face, the fur's and eyes' words beside it where both fit whole and
/// beneath it otherwise, then the swatch strip.
/// @spec TUI-RENDER-029, TUI-THEME-027, TUI-THEME-028, TUI-THEME-029
pub(crate) fn portrait_lines(profile: &Profile, width: usize) -> Vec<Line<'static>> {
    let fur = fur_words(&profile.fur);
    let eyes = content::eye_look_def(profile.eyes);
    let beside = ART_WIDTH + GAP + fur.chars().count().max(eyes.words.chars().count()) <= width;

    let mut out: Vec<Line<'static>> = Vec::new();
    for (row, art) in ART.iter().enumerate() {
        let mut spans: Vec<Span<'static>> = Vec::new();
        // Trailing blanks are dropped; every other blank holds its column.
        for (col, ch) in art.trim_end().chars().enumerate() {
            spans.push(match ch {
                '\u{2588}' => {
                    let slot = fur_slot(profile.fur.pattern, row, col);
                    match profile
                        .fur
                        .colours
                        .get(slot)
                        .or(profile.fur.colours.first())
                    {
                        Some(c) => Span::styled("\u{2588}", Style::default().fg(fur_colour(*c))),
                        None => Span::raw("\u{2588}"),
                    }
                }
                'o' => {
                    let colour = if col < ART_WIDTH / 2 {
                        eyes.left
                    } else {
                        eyes.right
                    };
                    Span::styled("o", Style::default().fg(eye_colour(colour)))
                }
                ' ' => Span::raw(BLANK.to_string()),
                other => Span::raw(other.to_string()),
            });
        }
        if beside && (row == 1 || row == 2) {
            let words = if row == 1 {
                fur.clone()
            } else {
                eyes.words.to_string()
            };
            spans.push(Span::raw(BLANK.to_string().repeat(GAP)));
            spans.push(Span::raw(words));
        }
        out.push(Line::from(spans));
    }
    if !beside {
        out.push(Line::raw(fur));
        out.push(Line::raw(eyes.words.to_string()));
    }

    // The strip: each colour's swatch and name, items two blanks apart.
    // @spec TUI-THEME-030
    let mut strip: Vec<Span<'static>> = Vec::new();
    for (i, c) in profile.fur.colours.iter().enumerate() {
        if i > 0 {
            strip.push(Span::raw("  "));
        }
        strip.push(Span::styled(
            "\u{2588}\u{2588}",
            Style::default().fg(fur_colour(*c)),
        ));
        strip.push(Span::raw(format!(
            "{BLANK}{}",
            content::fur_colour_name(*c)
        )));
    }
    out.push(Line::from(strip));
    out
}
