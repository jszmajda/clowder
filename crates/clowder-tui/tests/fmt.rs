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
    assert_eq!(
        fit("Sunbeam", 12),
        "Sunbeam",
        "a short name needs no shortening"
    );
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

// ---------------------------------------------------------------------------
// TUI-RENDER-017 — the compact form
// ---------------------------------------------------------------------------

/// Digits on show, whatever else the rendering carries.
fn digit_count(s: &str) -> usize {
    s.chars().filter(char::is_ascii_digit).count()
}

/// The compact form's magnitude markers, one per three decades, and the
/// power of a thousand each stands for.
const MARKERS: [(&str, i32); 5] = [("K", 1), ("M", 2), ("B", 3), ("T", 4), ("Q", 5)];

/// A compact rendering read back as a value: its figure times its marker's
/// power of a thousand. `None` for the two bound readings and anything else
/// that is not a plain figure.
fn read_compact(s: &str) -> Option<f64> {
    let (figure, scale) = match MARKERS.iter().find(|(m, _)| s.ends_with(m)) {
        Some((m, k)) => (&s[..s.len() - m.len()], 1000f64.powi(*k)),
        None => (s, 1.0),
    };
    figure.parse::<f64>().ok().map(|f| f * scale)
}

/// A deterministic spread of values across every magnitude the compact form
/// names and past both of its bounds, each with a ragged run of digits so no
/// rounding is ever trivially exact.
fn magnitude_sweep() -> Vec<f64> {
    let mut out = Vec::new();
    let mut seed: u64 = 0x2545_f491_4f6c_dd1d;
    for decade in -12..=20 {
        for _ in 0..40 {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let mantissa = 1.0 + (seed >> 11) as f64 / (1u64 << 53) as f64 * 9.0;
            out.push(mantissa * 10f64.powi(decade));
        }
    }
    out
}

/// The design's own examples and every edge the spec names, spelled out: four
/// digits at most, a tie rounding away from zero, a marker per three decades
/// with a rounding that reaches a thousand carrying into the next, three
/// places at most below one, `<0.001` for a value at or above the display
/// floor that rounds to nothing, `>999Q` past the last marker or for a value
/// that is not finite, and `0` for zero of either sign, residue below the
/// floor, and anything below zero.
/// @spec TUI-RENDER-017
#[test]
fn the_compact_form_reads_as_the_spec_spells_it_at_every_edge() {
    let cases: [(f64, &str); 48] = [
        (0.0, "0"),
        (-0.0, "0"),
        (3.0, "3"),
        (12.5, "12.5"),
        (999.9, "999.9"),
        (999.5, "999.5"),
        // Rounded to four digits, a tie away from zero.
        (1234.0, "1.234K"),
        (1234.5, "1.235K"),
        (12_345.0, "12.35K"),
        (1000.5, "1.001K"),
        (123_456_789.0, "123.5M"),
        (4_294_967_295.0, "4.295B"),
        // Each marker, once its rounded value reaches a thousand.
        (1000.0, "1K"),
        (1.0e6, "1M"),
        (1.0e9, "1B"),
        (1.0e12, "1T"),
        (1.0e15, "1Q"),
        // A rounding that reaches a thousand of one marker carries.
        (999.95, "1K"),
        (9_999.5, "10K"),
        (999_949.0, "999.9K"),
        (999_960.0, "1M"),
        (999_960_000.0, "1B"),
        (999_960_000_000.0, "1T"),
        (999_960_000_000_000.0, "1Q"),
        // Below one: three places at most, rounded to the nearest.
        (0.5, "0.5"),
        (0.125, "0.125"),
        (0.004, "0.004"),
        (0.1234, "0.123"),
        (0.0625, "0.063"),
        (0.9994, "0.999"),
        (0.9996, "1"),
        (0.0005, "0.001"),
        (0.001, "0.001"),
        // At or above the display floor, rounding to nothing in three places.
        (0.0004, "<0.001"),
        (0.000_012_3, "<0.001"),
        (1.0e-9, "<0.001"),
        // Past the last marker, or not finite.
        (999.94e15, "999.9Q"),
        (999.95e15, ">999Q"),
        (1.0e18, ">999Q"),
        (1.0e300, ">999Q"),
        (f64::INFINITY, ">999Q"),
        (f64::NAN, ">999Q"),
        // Below the display floor, and below zero.
        (0.9e-9, "0"),
        (5.0e-17, "0"),
        (-5.0e-17, "0"),
        (-1.0e-12, "0"),
        (-5.0, "0"),
        (-12_345.0, "0"),
    ];
    for (value, expected) in cases {
        assert_eq!(
            compact(value),
            expected,
            "compact({value:e}) must read {expected:?} (TUI-RENDER-017)"
        );
    }
}

/// The quantified half of the form, over values at every magnitude it names
/// and past both bounds: never more than four digits, never wider than six
/// columns, never a figure of a thousand or more under a marker (the carry),
/// and — for every value it states as a figure — rounded to the nearest, so
/// the figure read back is within half a unit of its last digit of the value.
/// @spec TUI-RENDER-017
#[test]
fn every_compact_figure_is_four_digits_at_most_rounded_to_the_nearest_and_six_columns_at_most() {
    let floor = 1.0e-9;
    let top = 999.95e15;
    for value in magnitude_sweep() {
        let rendered = compact(value);
        assert!(
            width(&rendered) <= COMPACT_MAX_WIDTH,
            "compact({value:e}) = {rendered:?} is wider than six columns (TUI-RENDER-017)"
        );
        assert!(
            digit_count(&rendered) <= 4,
            "compact({value:e}) = {rendered:?} shows more than four digits (TUI-RENDER-017)"
        );
        if value < floor {
            assert_eq!(
                rendered, "0",
                "compact({value:e}) is below the display floor (TUI-RENDER-017)"
            );
            continue;
        }
        if value >= top {
            assert_eq!(
                rendered, ">999Q",
                "compact({value:e}) reaches a thousand Q once rounded (TUI-RENDER-017)"
            );
            continue;
        }
        if value < 0.0005 {
            assert_eq!(
                rendered, "<0.001",
                "compact({value:e}) rounds to nothing in three places (TUI-RENDER-017)"
            );
            continue;
        }
        let figure = MARKERS
            .iter()
            .find(|(m, _)| rendered.ends_with(m))
            .map(|(m, _)| &rendered[..rendered.len() - m.len()])
            .unwrap_or(&rendered);
        let f: f64 = figure.parse().unwrap_or_else(|_| {
            panic!(
                "compact({value:e}) = {rendered:?} is not a figure and a marker (TUI-RENDER-017)"
            )
        });
        assert!(
            f < 1000.0,
            "compact({value:e}) = {rendered:?}: a rounding that reaches a thousand carries into the \
             next marker (TUI-RENDER-017)"
        );
        if figure.len() != rendered.len() {
            assert!(
                f >= 1.0,
                "compact({value:e}) = {rendered:?} wears a marker it has not reached"
            );
        }
        let unit = if value < 1.0 {
            0.001
        } else {
            10f64.powi(value.log10().floor() as i32 - 3)
        };
        let back = read_compact(&rendered).expect("a figure reads back");
        assert!(
            (back - value).abs() <= unit * 0.5 * (1.0 + 1.0e-9),
            "compact({value:e}) = {rendered:?} reads back as {back:e}, further than half a unit of \
             its last digit from the value; the form rounds to the nearest (TUI-RENDER-017)"
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-RENDER-018 — the exact form
// ---------------------------------------------------------------------------

/// A long whole part as the spec writes it: the fewest digits that read back
/// as the same float, and zeros in the places past them.
fn shortest_whole(value: f64) -> String {
    let sci = format!("{value:e}");
    let (mantissa, exp) = sci.split_once('e').expect("scientific form");
    let exp: usize = exp.parse().expect("a non-negative exponent");
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    assert!(digits.len() <= exp + 1, "fixture: {value:e} has a fraction");
    format!("{digits}{}", "0".repeat(exp + 1 - digits.len()))
}

/// The design's own examples and every edge the spec names: every digit of
/// the whole part with no marker, exponent, or grouping; the fraction at three
/// places or at the fourth significant figure, whichever lies further, a tie
/// rounding away from zero, trailing zeros dropped; binary noise rounded away;
/// `0` for zero of either sign and for residue below the display floor; and a
/// figure below zero behind `-`.
/// @spec TUI-RENDER-018
#[test]
fn the_exact_form_reads_as_the_spec_spells_it_at_every_edge() {
    let cases: Vec<(f64, String)> = vec![
        (3.0, "3".into()),
        (1234.5, "1234.5".into()),
        (1_234_567.0, "1234567".into()),
        (4_294_967_295.0, "4294967295".into()),
        (1.0e12, "1000000000000".into()),
        (0.25, "0.25".into()),
        // Three places, or the fourth significant figure where that is further.
        (0.1234, "0.1234".into()),
        (0.001_234_56, "0.001235".into()),
        (0.000_123_456_78, "0.0001235".into()),
        (0.0625, "0.0625".into()),
        (12_345.678_9, "12345.679".into()),
        (1.234_567, "1.235".into()),
        // A tie rounds away from zero.
        (2.0625, "2.063".into()),
        (12.0625, "12.063".into()),
        (-2.0625, "-2.063".into()),
        // A rounding that carries into the whole part.
        (0.999_999_6, "1".into()),
        (999.999_6, "1000".into()),
        // Binary noise is not precision.
        (0.1 + 0.2, "0.3".into()),
        (1.1 * 1.1, "1.21".into()),
        // Down to the display floor, and not past it.
        (1.0e-9, "0.000000001".into()),
        (1.5e-9, "0.0000000015".into()),
        (0.9e-9, "0".into()),
        (5.0e-17, "0".into()),
        (-5.0e-17, "0".into()),
        (0.0, "0".into()),
        (-0.0, "0".into()),
        // Below zero, behind `-`.
        (-3.0, "-3".into()),
        (-0.25, "-0.25".into()),
        (-1234.5, "-1234.5".into()),
        // A whole part longer than the float's precision.
        (1.0e25, format!("1{}", "0".repeat(25))),
        (
            1.234_567_890_123_456_7e25,
            shortest_whole(1.234_567_890_123_456_7e25),
        ),
        (
            123_456_789_012_345_680.0,
            shortest_whole(123_456_789_012_345_680.0),
        ),
        (-1.0e20, format!("-1{}", "0".repeat(20))),
    ];
    for (value, expected) in cases {
        assert_eq!(
            exact(value),
            expected,
            "exact({value:e}) must read {expected:?} (TUI-RENDER-018)"
        );
    }
}

/// The quantified half, over values at every magnitude from the display floor
/// up and of both signs: only digits, a point, and a leading `-` — no marker,
/// exponent, or grouping; no trailing zero in the fraction and no dangling
/// point; no more fraction than three places or the fourth significant figure
/// allows; and rounded to the nearest at that place.
/// @spec TUI-RENDER-018
#[test]
fn every_exact_figure_is_every_whole_digit_and_a_fraction_rounded_at_its_place() {
    for magnitude in magnitude_sweep()
        .into_iter()
        .filter(|v| *v >= 1.0e-9 && *v < 1.0e15)
    {
        for value in [magnitude, -magnitude] {
            let rendered = exact(value);
            let body = rendered.strip_prefix('-').unwrap_or(&rendered);
            assert_eq!(
                rendered.starts_with('-'),
                value < 0.0,
                "exact({value:e}) = {rendered:?}: a figure below zero, and only one, stands behind `-`"
            );
            assert!(
                !body.is_empty() && body.chars().all(|c| c.is_ascii_digit() || c == '.'),
                "exact({value:e}) = {rendered:?} holds something other than digits and a point: no \
                 marker, exponent, or grouping (TUI-RENDER-018)"
            );
            assert_no_decimal_noise(&rendered, &format!("exact({value:e})"));
            let e = magnitude.log10().floor() as i32;
            let places = 3.max(3 - e) as usize;
            assert!(
                decimals(&rendered) <= places,
                "exact({value:e}) = {rendered:?} carries more than {places} places, past both three \
                 places and the fourth significant figure (TUI-RENDER-018)"
            );
            let back: f64 = rendered.parse().expect("an exact figure parses");
            let half = 0.5 * 10f64.powi(-(places as i32));
            assert!(
                (back - value).abs() <= half * (1.0 + 1.0e-9) + value.abs() * 1.0e-15,
                "exact({value:e}) = {rendered:?} is further than half a unit of its last place from \
                 the value; the fraction rounds to the nearest (TUI-RENDER-018)"
            );
            let whole = body.split('.').next().unwrap_or("");
            assert!(
                whole == "0" || !whole.starts_with('0'),
                "exact({value:e}) = {rendered:?} pads its whole part"
            );
        }
    }
}

/// A figure that is not finite has no digits to state, and is said in words:
/// never `inf`, `NaN`, `∞`, or any digit.
/// @spec TUI-RENDER-018
#[test]
fn the_exact_form_says_a_figure_that_is_not_finite_in_words() {
    for value in [f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
        let rendered = exact(value);
        let lower = rendered.to_lowercase();
        assert!(
            !rendered.chars().any(|c| c.is_ascii_digit() || c == '∞'),
            "exact({value}) = {rendered:?} prints a figure that is not finite (TUI-RENDER-018)"
        );
        assert!(
            !lower
                .split(|c: char| !c.is_alphabetic())
                .any(|w| w == "inf" || w == "infinity" || w == "nan"),
            "exact({value}) = {rendered:?} prints the float's own spelling (TUI-RENDER-018)"
        );
        assert!(
            rendered.split_whitespace().count() >= 2
                && rendered.chars().all(|c| c.is_alphabetic() || c == ' '),
            "exact({value}) = {rendered:?}: a figure that is not finite is stated in words \
             (TUI-RENDER-018)"
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-RENDER-012 — the sidebar's rate form
// ---------------------------------------------------------------------------

/// A store's net rate in the sidebar is the compact form of its size behind
/// `+` above zero and `-` below, with no sign where its size reads `0` — zero
/// of either sign, or residue below the display floor.
/// @spec TUI-RENDER-012
#[test]
fn the_sidebar_rate_is_the_compact_size_behind_its_sign_and_unsigned_when_still() {
    let cases: [(f64, &str); 16] = [
        (1.5, "+1.5"),
        (-1.5, "-1.5"),
        (-12_345.0, "-12.35K"),
        (1234.5, "+1.235K"),
        (0.0004, "+<0.001"),
        (-0.0004, "-<0.001"),
        (1.0e-9, "+<0.001"),
        (999.95e15, "+>999Q"),
        (f64::INFINITY, "+>999Q"),
        (f64::NEG_INFINITY, "->999Q"),
        (0.0, "0"),
        (-0.0, "0"),
        (1.0e-12, "0"),
        (-1.0e-12, "0"),
        (5.0e-17, "0"),
        (-0.9e-9, "0"),
    ];
    for (value, expected) in cases {
        assert_eq!(
            rate(value),
            expected,
            "rate({value:e}) must read {expected:?} (TUI-RENDER-012)"
        );
    }
}

/// Quantified: at every magnitude and either sign, the sidebar rate is the
/// compact form of the rate's size behind its sign, and so at most seven
/// columns.
/// @spec TUI-RENDER-012
#[test]
fn every_sidebar_rate_is_its_sign_and_the_compact_form_of_its_size() {
    for magnitude in magnitude_sweep() {
        for value in [magnitude, -magnitude] {
            let rendered = rate(value);
            let size = compact(value.abs());
            let expected = if size == "0" {
                "0".to_string()
            } else if value > 0.0 {
                format!("+{size}")
            } else {
                format!("-{size}")
            };
            assert_eq!(rendered, expected, "rate({value:e}) (TUI-RENDER-012)");
            assert!(
                width(&rendered) <= 7,
                "rate({value:e}) = {rendered:?} is wider than seven columns"
            );
        }
    }
}
