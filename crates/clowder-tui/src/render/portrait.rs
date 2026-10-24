//! A kitten's portrait: the kitten drawn small in its own pose, its body
//! painted in its own fur, with the fur's and eyes' words beside it where they
//! fit, and the strip of its colours beneath. @spec TUI-RENDER-029

use clowder_core::{content, fur_words, Pose, Profile};
use ratatui::style::Style;
use ratatui::text::{Line, Span};

use crate::theme::fur::{eye_colour, fur_colour, fur_slot, FurPart, Patch};

/// A blank that wrapping never trims or breaks at, so the portrait keeps its
/// shape and a swatch stays with its name.
const BLANK: char = '\u{a0}';

/// Blank columns between the cat and the words beside it.
const GAP: usize = 3;

/// A pose's drawing: its rows, where `█` is fur and every other mark outline;
/// its eye cells, left then right, where the kitten is awake; and every fur
/// cell's part and patch, as the render design labels them.
struct Drawing {
    rows: &'static [&'static str],
    eyes: Option<[(usize, usize); 2]>,
    fur: &'static [(usize, usize, FurPart, Option<Patch>)],
}

const A: Option<Patch> = Some(Patch::A);
const B: Option<Patch> = Some(Patch::B);
const N: Option<Patch> = None;

/// The render design's drawing for each pose.
fn drawing(pose: Pose) -> Drawing {
    use FurPart::*;
    match pose {
        Pose::Sitting => Drawing {
            rows: &[
                "   /\\_/\\",
                "  ( o.o )",
                "   > ^ <",
                "  /\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\\  __",
                " (_\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}_)\u{2588}\u{2588}/",
            ],
            eyes: Some([(1, 4), (1, 6)]),
            fur: &[
                (3, 3, Top, A),
                (3, 7, Top, B),
                (3, 4, Back, A),
                (3, 6, Back, N),
                (3, 5, Chest, N),
                (4, 5, Chest, N),
                (4, 4, Under, A),
                (4, 6, Under, B),
                (4, 3, Paw, N),
                (4, 7, Paw, B),
                (4, 10, Tail, A),
                (4, 11, Tail, B),
            ],
        },
        Pose::Napping => Drawing {
            rows: &[
                "         /\\_/\\",
                "   _____( -.- )",
                "  /\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}  \\",
                " (_\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}_)",
            ],
            eyes: None,
            fur: &[
                (2, 9, Top, B),
                (2, 10, Top, B),
                (2, 11, Top, N),
                (2, 4, Back, A),
                (2, 5, Back, A),
                (2, 6, Back, A),
                (2, 7, Back, N),
                (2, 8, Back, N),
                (3, 10, Chest, B),
                (3, 11, Chest, N),
                (3, 4, Under, N),
                (3, 5, Under, A),
                (3, 6, Under, N),
                (3, 7, Under, N),
                (3, 8, Under, N),
                (3, 9, Under, B),
                (3, 12, Paw, A),
                (3, 13, Paw, N),
                (2, 3, Tail, N),
                (3, 3, Tail, B),
            ],
        },
        Pose::Loaf => Drawing {
            rows: &[
                "      /\\_/\\",
                "  ___( o.o )",
                " /\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\\",
                "(_\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}_)",
            ],
            eyes: Some([(1, 7), (1, 9)]),
            fur: &[
                (2, 8, Top, N),
                (2, 9, Top, A),
                (2, 10, Top, N),
                (2, 11, Top, B),
                (2, 3, Back, A),
                (2, 4, Back, A),
                (2, 5, Back, N),
                (2, 6, Back, B),
                (2, 7, Back, B),
                (3, 8, Chest, N),
                (3, 9, Chest, N),
                (3, 3, Under, A),
                (3, 4, Under, N),
                (3, 5, Under, N),
                (3, 6, Under, B),
                (3, 7, Under, B),
                (3, 10, Paw, N),
                (3, 11, Paw, A),
                (2, 2, Tail, N),
                (3, 2, Tail, N),
            ],
        },
    }
}

/// The portrait's lines for an inspector `width` columns wide: the rows of
/// the kitten's pose, the fur's and eyes' words beside it where both fit whole
/// and beneath it otherwise, then the swatch strip.
/// @spec TUI-RENDER-029, TUI-THEME-027, TUI-THEME-028, TUI-THEME-029
pub(crate) fn portrait_lines(profile: &Profile, width: usize) -> Vec<Line<'static>> {
    let d = drawing(profile.pose);
    let fur = fur_words(&profile.fur);
    let eyes = content::eye_look_def(profile.eyes);
    let widest = d.rows.iter().map(|r| r.chars().count()).max().unwrap_or(0);
    let beside = widest + GAP + fur.chars().count().max(eyes.words.chars().count()) <= width;

    let mut out: Vec<Line<'static>> = Vec::new();
    for (row, art) in d.rows.iter().enumerate() {
        let mut spans: Vec<Span<'static>> = Vec::new();
        for (col, ch) in art.chars().enumerate() {
            let fur_cell = d.fur.iter().find(|f| (f.0, f.1) == (row, col));
            let eye = d.eyes.and_then(|[l, r]| {
                [(l, eyes.left), (r, eyes.right)]
                    .into_iter()
                    .find(|e| e.0 == (row, col))
            });
            spans.push(match (fur_cell, eye) {
                (Some(&(_, _, part, patch)), _) => {
                    let slot = fur_slot(profile.fur.pattern, part, patch, col);
                    match profile
                        .fur
                        .colours
                        .get(slot)
                        .or(profile.fur.colours.first())
                    {
                        Some(c) => {
                            Span::styled(ch.to_string(), Style::default().fg(fur_colour(*c)))
                        }
                        None => Span::raw(ch.to_string()),
                    }
                }
                (None, Some((_, colour))) => {
                    Span::styled(ch.to_string(), Style::default().fg(eye_colour(colour)))
                }
                (None, None) if ch == ' ' => Span::raw(BLANK.to_string()),
                (None, None) => Span::raw(ch.to_string()),
            });
        }
        if beside && (row == 1 || row == 2) {
            let words = if row == 1 {
                fur.clone()
            } else {
                eyes.words.to_string()
            };
            let pad = widest - art.chars().count() + GAP;
            spans.push(Span::raw(BLANK.to_string().repeat(pad)));
            spans.push(Span::raw(words));
        }
        out.push(Line::from(spans));
    }
    if !beside {
        out.push(Line::raw(fur));
        out.push(Line::raw(eyes.words.to_string()));
    }

    // The strip: each colour's swatch and name, items two blanks apart, a
    // blank line either side so the cat, its colours, and the facts read as
    // three things. @spec TUI-THEME-030, TUI-RENDER-029
    out.push(Line::raw(""));
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
    out.push(Line::raw(""));
    out
}
