//! Numbers and names, fitted to the space available.
//!
//! Space is the scarce resource on a glanceable surface, and these three specs
//! decide how it is spent. The properties under test are the *reasons*, not the
//! shapes:
//!
//! - a compact number stays inside a column no matter its magnitude, and shows
//!   the same significant figures at every magnitude, so a column of them
//!   aligns and a value's size is legible at a glance (TUI-RENDER-012);
//! - every abbreviated number has an exact, unabbreviated form — that pairing
//!   is what keeps whimsy from costing clarity (TUI-RENDER-012);
//! - precision follows magnitude, so nothing carries decimals it has not
//!   earned (TUI-RENDER-013);
//! - a shortened name says so with a visible ellipsis, because a hard cut
//!   hides the loss (TUI-RENDER-014).
//!
//! `clowder-tui` is a binary with no library target, so the module under test
//! is included by path — the real implementation file, not a copy.
//!
//! Display width is measured with ratatui's own `Span::width`, which is the
//! measure the renderer itself uses; counting `char`s or bytes would agree with
//! it only for ASCII, and names are content, not UI glyphs.

use clowder_tui::render::fmt;

use fmt::{compact, exact, fit, rate};
use ratatui::backend::TestBackend;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use ratatui::Terminal;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Display width as the renderer measures it.
fn width(s: &str) -> usize {
    Span::raw(s).width()
}

/// The columns a compact number may occupy. Four significant figures, a
/// decimal point, and a single-width magnitude marker is six; anything wider
/// has stopped being compact and a column sized for it has stopped being
/// glanceable.
const COMPACT_MAX_WIDTH: usize = 6;

/// Digits shown after the decimal point, if any.
fn decimals(s: &str) -> usize {
    match s.split_once('.') {
        None => 0,
        Some((_, tail)) => tail.chars().take_while(char::is_ascii_digit).count(),
    }
}

/// Significant digits on show: everything that is not a digit (sign, decimal
/// point, magnitude marker, any padding) dropped, then leading zeros dropped.
fn significant_digits(s: &str) -> usize {
    let digits: String = s.chars().filter(char::is_ascii_digit).collect();
    digits.trim_start_matches('0').len()
}

/// Whether the rendering shows any non-zero digit — i.e. whether a player
/// reading it would see a quantity rather than nothing at all.
fn shows_a_nonzero_value(s: &str) -> bool {
    s.chars().any(|c| c.is_ascii_digit() && c != '0')
}

/// A rendering carries no decimals it has not earned: no fractional part that
/// ends in a zero, and no dangling point. @spec TUI-RENDER-013
fn assert_no_decimal_noise(rendered: &str, what: &str) {
    let trimmed = rendered.trim_end();
    assert!(
        !trimmed.ends_with('.'),
        "{what} rendered as {rendered:?}, which ends in a bare decimal point"
    );
    if let Some((_, tail)) = trimmed.split_once('.') {
        let frac: String = tail.chars().take_while(char::is_ascii_digit).collect();
        assert!(
            !frac.ends_with('0'),
            "{what} rendered as {rendered:?}: a trailing zero decimal claims \
             precision the value does not carry (TUI-RENDER-013)"
        );
    }
}

/// The forms a "visible ellipsis" may honestly take. The arrow does not name
/// one (see openQuestions), so both are accepted; what is *not* accepted is a
/// silent cut.
const ELLIPSES: [&str; 2] = ["\u{2026}", "..."];

fn ends_with_ellipsis(s: &str) -> bool {
    ELLIPSES.iter().any(|e| s.ends_with(e))
}

fn without_ellipsis(s: &str) -> &str {
    for e in ELLIPSES {
        if let Some(head) = s.strip_suffix(e) {
            return head;
        }
    }
    s
}

/// Parse an exact rendering back to a number, tolerating any grouping
/// separators it may use.
fn parse_exact(s: &str) -> f64 {
    let cleaned: String = s
        .chars()
        .filter(|c| !matches!(c, ',' | '_' | '\'' | ' ' | '\u{2009}' | '\u{202f}'))
        .collect();
    cleaned.parse::<f64>().unwrap_or_else(|e| {
        panic!(
            "the exact form {s:?} did not parse back as a number ({e}); \
             TUI-RENDER-012 requires an exact, unabbreviated rendering"
        )
    })
}

/// Values with a full run of non-zero digits, one per decade — so a fixed
/// significant-figure rule has the same number of digits to show at every
/// magnitude and no trailing zero can be legitimately dropped.
fn decade_sweep(top: i32) -> Vec<f64> {
    (0..=top).map(|n| 1.234_f64 * 10f64.powi(n)).collect()
}

/// Names long enough to need cutting at any realistic column width.
const LONG_NAMES: [&str; 4] = [
    "a name far too long for its column",
    "Cafe\u{301} au Lait de la Maison",
    "\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{306a}\u{307e}\u{3048}\u{3067}\u{3059}",
    "\u{65e5}\u{672c} mixed \u{8a9e} widths",
];

// ---------------------------------------------------------------------------
// TUI-RENDER-012 — compact where glanced at, exact where read
// ---------------------------------------------------------------------------

/// Alignment is the reason for the compact form: a column can only be sized
/// once, so a number's rendering must not grow with its magnitude.
/// @spec TUI-RENDER-012
#[test]
fn a_compact_number_stays_inside_its_column_at_every_magnitude() {
    let small = width(&compact(1.0));

    for value in decade_sweep(15) {
        let rendered = compact(value);
        let w = width(&rendered);
        assert!(
            w <= COMPACT_MAX_WIDTH,
            "compact({value}) rendered as {rendered:?} ({w} columns); a compact \
             number may not exceed {COMPACT_MAX_WIDTH} columns or the column \
             sized for it stops being glanceable (TUI-RENDER-012)"
        );
        assert!(
            w <= small.max(COMPACT_MAX_WIDTH),
            "compact({value}) rendered as {rendered:?}: width grew with \
             magnitude, so a column of these cannot align (TUI-RENDER-012)"
        );
    }
}

/// "Fixed significant-figure width" — the same digits of information at every
/// magnitude, which is what makes two stacked values comparable at a glance.
/// @spec TUI-RENDER-012
#[test]
fn a_compact_number_shows_the_same_significant_figures_at_every_magnitude() {
    let sweep = decade_sweep(12);
    let baseline = compact(sweep[0]);
    let expected = significant_digits(&baseline);

    assert!(
        expected > 0,
        "compact({}) rendered as {baseline:?}, showing no significant digits",
        sweep[0]
    );

    for value in &sweep {
        let rendered = compact(*value);
        assert_eq!(
            significant_digits(&rendered),
            expected,
            "compact({value}) rendered as {rendered:?}, showing a different \
             number of significant figures than compact({}) = {baseline:?}; \
             TUI-RENDER-012 fixes the significant-figure width",
            sweep[0]
        );
    }
}

/// The inspector's half of the contract: exact means every digit, no magnitude
/// marker, no scientific shorthand. @spec TUI-RENDER-012
#[test]
fn the_exact_form_is_unabbreviated_and_loses_nothing() {
    for value in [3.0, 42.0, 999.0, 1234.5, 0.25, 1_234_567.0, 1e12] {
        let rendered = exact(value);

        assert!(
            !rendered.chars().any(|c| c.is_ascii_alphabetic()),
            "exact({value}) rendered as {rendered:?}; a magnitude marker or an \
             exponent is an abbreviation, and the inspector's number is \
             unabbreviated (TUI-RENDER-012)"
        );
        assert_eq!(
            parse_exact(&rendered),
            value,
            "exact({value}) rendered as {rendered:?}, which does not read back \
             as the value it stands for (TUI-RENDER-012)"
        );
    }
}

/// The pairing is the whole contract: whatever the compact form rounds away,
/// the exact form one keypress later still has. @spec TUI-RENDER-012
#[test]
fn what_the_compact_form_rounds_away_the_exact_form_still_carries() {
    let value = 1_234_567.0;

    let glanced = compact(value);
    let read = exact(value);

    assert!(
        width(&glanced) < width(&read),
        "compact({value}) = {glanced:?} is no shorter than exact({value}) = \
         {read:?}; there is nothing compact about it (TUI-RENDER-012)"
    );
    assert!(
        significant_digits(&glanced) < significant_digits(&read),
        "compact({value}) = {glanced:?} kept every digit; a glanceable surface \
         abbreviates (TUI-RENDER-012)"
    );

    let read_digits: String = read.chars().filter(char::is_ascii_digit).collect();
    assert_eq!(
        read_digits, "1234567",
        "exact({value}) = {read:?} does not carry the digits the compact form \
         dropped, so the abbreviation is unrecoverable (TUI-RENDER-012)"
    );
}

/// A rate column is a column too: a resource that is draining and one that is
/// filling must line up. Whether the gain wears a `+` or a leading space is
/// unstated — that they are the same width is not.
/// @spec TUI-RENDER-012
#[test]
fn a_rate_column_aligns_whether_the_resource_gains_or_drains() {
    for magnitude in [0.5_f64, 3.0, 12.34, 1234.0, 1.234e6] {
        let gain = rate(magnitude);
        let drain = rate(-magnitude);

        assert_eq!(
            width(&gain),
            width(&drain),
            "rate(+{magnitude}) = {gain:?} and rate(-{magnitude}) = {drain:?} \
             render at different widths, so a sidebar of rates cannot align \
             (TUI-RENDER-012)"
        );
        assert!(
            drain.trim_start().starts_with('-'),
            "rate(-{magnitude}) rendered as {drain:?}, which does not read as a \
             loss"
        );
        assert!(
            width(&gain) <= COMPACT_MAX_WIDTH + 3,
            "rate({magnitude}) rendered as {gain:?}; a sidebar rate is a \
             compact number with a sign, not an exact one (TUI-RENDER-012)"
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-RENDER-013 — precision follows magnitude
// ---------------------------------------------------------------------------

/// The design states this one outright: a stock of 3 reads as `3`, not `3.00`.
/// @spec TUI-RENDER-013
#[test]
fn a_stock_of_three_reads_as_three() {
    assert_eq!(
        compact(3.0),
        "3",
        "a stock of 3 must read as `3`; trailing decimals imply a precision the \
         value does not carry (TUI-RENDER-013)"
    );
    assert_eq!(
        compact(0.0),
        "0",
        "an empty stock reads as `0` (TUI-RENDER-013)"
    );
    assert_eq!(
        exact(3.0),
        "3",
        "exact means unabbreviated, not padded with decimals the value does not \
         carry (TUI-RENDER-012, TUI-RENDER-013)"
    );
}

/// No rendering anywhere may end in decimal noise — not the round values, not
/// the ragged ones, not the rates. @spec TUI-RENDER-013
#[test]
fn no_number_carries_decimals_it_has_not_earned() {
    let values = [
        0.0, 1.0, 3.0, 5.0, 12.5, 50.0, 100.0, 500.0, 1000.0, 1234.0, 12_500.0, 1e6, 1.234e9,
    ];

    for value in values {
        assert_no_decimal_noise(&compact(value), &format!("compact({value})"));
        assert_no_decimal_noise(&exact(value), &format!("exact({value})"));
        assert_no_decimal_noise(&rate(value), &format!("rate({value})"));
        assert_no_decimal_noise(&rate(-value), &format!("rate(-{value})"));
    }
}

/// Precision is spent where it means something: within a magnitude band, the
/// bigger the number the fewer decimals it earns. @spec TUI-RENDER-013
#[test]
fn precision_shrinks_as_magnitude_grows() {
    // Two bands, so the check is about magnitude within a band rather than
    // about crossing into an abbreviation.
    for band in [[1.234_f64, 12.34, 123.4], [1234.0, 12_340.0, 123_400.0]] {
        let rendered: Vec<String> = band.iter().map(|v| compact(*v)).collect();
        let places: Vec<usize> = rendered.iter().map(|s| decimals(s)).collect();

        for pair in places.windows(2) {
            assert!(
                pair[0] >= pair[1],
                "within one magnitude band the decimals must not grow with the \
                 value: {band:?} rendered as {rendered:?} ({places:?} decimal \
                 places) (TUI-RENDER-013)"
            );
        }
        assert!(
            places[0] > places[places.len() - 1],
            "precision must follow magnitude: {band:?} rendered as {rendered:?} \
             with an unchanging {places:?} decimal places (TUI-RENDER-013)"
        );
    }
}

/// The other edge of the same rule: a small value must not be rounded into
/// looking like nothing is happening. A fixed significant-figure form has
/// digits to spend down here; a fixed decimal-place one does not.
/// @spec TUI-RENDER-012, TUI-RENDER-013
#[test]
fn a_small_value_does_not_round_away_to_nothing() {
    for value in [0.5_f64, 0.05, 0.004] {
        let glanced = compact(value);
        assert!(
            shows_a_nonzero_value(&glanced),
            "compact({value}) rendered as {glanced:?}, which reads as nothing at \
             all; significant figures, not fixed decimal places (TUI-RENDER-012, \
             TUI-RENDER-013)"
        );

        let moving = rate(value);
        assert!(
            shows_a_nonzero_value(&moving),
            "rate({value}) rendered as {moving:?}, which claims the resource is \
             not moving (TUI-RENDER-012, TUI-RENDER-013)"
        );
    }
}

/// A zero rate is a zero rate however the float spells it: `-0.0` must not
/// display a drain that is not there. @spec TUI-RENDER-013
#[test]
fn a_zero_rate_never_shows_a_minus_sign() {
    let zero = rate(0.0);
    let negative_zero = rate(-0.0);

    assert_eq!(
        negative_zero, zero,
        "rate(-0.0) rendered as {negative_zero:?} and rate(0.0) as {zero:?}; the \
         same quantity must render the same way, and a minus sign here claims a \
         drain that is not happening (TUI-RENDER-013)"
    );
}

// ---------------------------------------------------------------------------
// TUI-RENDER-014 — a cut name says it was cut
// ---------------------------------------------------------------------------

/// Truncation is for names that do not fit. One that fits — including one that
/// fits exactly — is left alone, and measuring it in bytes rather than columns
/// would cut a name that fits perfectly well. @spec TUI-RENDER-014
#[test]
fn a_name_that_fits_is_left_alone() {
    assert_eq!(fit("Sunbeam", 12), "Sunbeam", "a short name needs no shortening");
    assert_eq!(
        fit("Sunbeam", 7),
        "Sunbeam",
        "a name that fills its column exactly still fits"
    );
    assert_eq!(
        fit("Cafe\u{301}", 4),
        "Cafe\u{301}",
        "four columns of multi-byte name fit four columns; a byte-length test \
         would wrongly cut it (TUI-RENDER-014)"
    );
    assert_eq!(
        fit("\u{65e5}\u{672c}", 4),
        "\u{65e5}\u{672c}",
        "two double-width characters fill four columns exactly (TUI-RENDER-014)"
    );
}

/// The rejected alternative was a hard cut, because it hides that anything was
/// lost. What is kept must still read as the beginning of the name, and the
/// end must say that there is more. @spec TUI-RENDER-014
#[test]
fn a_shortened_name_shows_that_it_was_cut() {
    let name = "a name far too long for its column";
    let fitted = fit(name, 10);

    assert!(
        width(&fitted) <= 10,
        "fit({name:?}, 10) rendered as {fitted:?}, {} columns wide",
        width(&fitted)
    );
    assert!(
        ends_with_ellipsis(&fitted),
        "fit({name:?}, 10) rendered as {fitted:?}: a shortened name carries an \
         explicit ellipsis so the player can see text was cut (TUI-RENDER-014)"
    );

    let kept = without_ellipsis(&fitted);
    assert!(
        !kept.is_empty() && name.starts_with(kept),
        "fit({name:?}, 10) kept {kept:?}, which is not the beginning of the \
         name; a truncated name must still be recognisable (TUI-RENDER-014)"
    );

    let hard_cut: String = name.chars().take(10).collect();
    assert_ne!(
        fitted, hard_cut,
        "fit({name:?}, 10) hard-cut the name, hiding the loss (TUI-RENDER-014)"
    );
}

/// The invariant the frame depends on: a fitted name never spends more columns
/// than it was given, at any width, for any name. @spec TUI-RENDER-014
#[test]
fn a_fitted_name_never_overflows_its_column() {
    for name in LONG_NAMES {
        for column in 0..=24_usize {
            let fitted = fit(name, column);
            let w = width(&fitted);

            assert!(
                w <= column,
                "fit({name:?}, {column}) rendered as {fitted:?}, {w} columns \
                 wide — wider than the column it was fitted to (TUI-RENDER-014)"
            );

            if width(name) <= column {
                assert_eq!(
                    fitted, name,
                    "fit({name:?}, {column}) shortened a name that fits"
                );
            } else if column >= 4 {
                // Wide enough for at least one character plus either ellipsis
                // form; narrower columns are unstated (see openQuestions).
                assert!(
                    ends_with_ellipsis(&fitted),
                    "fit({name:?}, {column}) rendered as {fitted:?} with no \
                     ellipsis, though the name did not fit (TUI-RENDER-014)"
                );
                let kept = without_ellipsis(&fitted);
                assert!(
                    name.starts_with(kept),
                    "fit({name:?}, {column}) kept {kept:?}, which is not the \
                     beginning of the name (TUI-RENDER-014)"
                );
            }
        }
    }
}

/// Names are content, not UI glyphs: TUI-NAV-021 constrains the symbols the
/// shell draws, but a name may hold double-width or multi-byte characters.
/// Cutting by bytes panics; cutting by `char` count overflows the column.
/// @spec TUI-RENDER-014
#[test]
fn wide_and_multibyte_names_are_measured_in_columns_not_bytes() {
    let wide = "\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{306a}\u{307e}\u{3048}";

    // Seven double-width characters: fourteen columns of name.
    assert_eq!(width(wide), 14, "the fixture is a double-width name");

    for column in [5_usize, 6, 7, 8, 11] {
        let fitted = fit(wide, column);
        let w = width(&fitted);
        assert!(
            w <= column,
            "fit({wide:?}, {column}) rendered as {fitted:?}, {w} columns wide; a \
             double-width character cannot be half-drawn (TUI-RENDER-014)"
        );
        let kept = without_ellipsis(&fitted);
        assert!(
            wide.starts_with(kept),
            "fit({wide:?}, {column}) kept {kept:?}, which is not a prefix of the \
             name — a byte-wise cut corrupts a multi-byte character \
             (TUI-RENDER-014)"
        );
    }

    // Combining marks: the accent belongs to the letter before it.
    let accented = "Cafe\u{301} au Lait de la Maison";
    for column in 3..=12_usize {
        let fitted = fit(accented, column);
        assert!(
            width(&fitted) <= column,
            "fit({accented:?}, {column}) rendered as {fitted:?}, wider than its \
             column (TUI-RENDER-014)"
        );
        assert!(
            accented.starts_with(without_ellipsis(&fitted)),
            "fit({accented:?}, {column}) rendered as {fitted:?}, which is not the \
             beginning of the name (TUI-RENDER-014)"
        );
    }
}

/// The column claim, checked against a real render rather than a width
/// function: nothing a fitted name draws may land in the next column.
/// @spec TUI-RENDER-014
#[test]
fn a_fitted_name_leaves_the_next_column_untouched() {
    const COLUMN: u16 = 10;
    const TERMINAL: u16 = 24;

    for name in LONG_NAMES {
        let fitted = fit(name, COLUMN as usize);

        let mut terminal = Terminal::new(TestBackend::new(TERMINAL, 1)).expect("test terminal");
        terminal
            .draw(|frame| frame.render_widget(Paragraph::new(fitted.clone()), frame.area()))
            .expect("drawing a fitted name");

        let buffer = terminal.backend().buffer().clone();
        for x in COLUMN..TERMINAL {
            let symbol = buffer[(x, 0)].symbol();
            assert!(
                symbol == " " || symbol.is_empty(),
                "fit({name:?}, {COLUMN}) drew {fitted:?}, which reached column \
                 {x} — beyond the {COLUMN} columns it was given, into whatever \
                 the row draws next (TUI-RENDER-014)"
            );
        }
    }
}
