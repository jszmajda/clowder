//! Numbers and names, fitted to the space available.
//!
//! Compact where glanced at, exact where read: every abbreviated number on
//! screen has an exact form one keypress away in the inspector, which is what
//! keeps whimsy from costing clarity. @spec TUI-RENDER-012, TUI-RENDER-013,
//! TUI-RENDER-017, TUI-RENDER-018

use ratatui::text::Span;

/// Single-width magnitude markers, one per group of three decades. Four
/// significant figures, a decimal point, and a single-width marker is the
/// six-column budget a compact number may not exceed.
const MAGNITUDE_SUFFIXES: [&str; 6] = ["", "K", "M", "B", "T", "Q"];

/// The magnitude below which a value reads as nothing at all: `0` for a
/// stock, no sign for a rate. Shared with anything that must decide, before
/// formatting, whether a figure will read as more than zero — such as the
/// while-away banner's largest gains (TUI-NAV-055), which leaves off a gain
/// that would draw as `0` rather than draw it. @spec TUI-RENDER-017,
/// TUI-RENDER-018
pub(crate) const DISPLAY_FLOOR: f64 = 1.0e-9;

/// Drop trailing zero decimals (and a dangling decimal point) so a rendering
/// never claims precision it does not carry.
fn trim_decimal_noise(s: &str) -> String {
    if !s.contains('.') {
        return s.to_string();
    }
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// The shortest round-trip decimal digits of a positive, finite value, and
/// the power-of-ten position of the first one — `abs = 0.d0 d1 d2… ×
/// 10^(exp+1)`, read straight off Rust's own shortest round-trip scientific
/// formatting so no magnitude, however large, invents precision the float
/// does not carry.
fn digits_and_exp(abs: f64) -> (Vec<u8>, i32) {
    let sci = format!("{abs:e}");
    let (mantissa, exp_s) = sci.split_once('e').expect("scientific form always has an exponent");
    let exp: i32 = exp_s.parse().expect("a parseable exponent");
    let digits: Vec<u8> = mantissa.bytes().filter(u8::is_ascii_digit).map(|b| b - b'0').collect();
    (digits, exp)
}

/// Round a positive, finite value to exactly `n` significant digits, tie away
/// from zero — the one rounding both the compact and the exact form share,
/// carry (a run of 9s rolling over into a new leading digit) included, which
/// is what lets a rounding that reaches a thousand carry into the next
/// magnitude marker (TUI-RENDER-017) or a whole part into a new digit
/// (TUI-RENDER-018). Returns the `n` (or `n+1`, on carry) digits and the
/// updated exponent of the first one.
fn round_to_digits(abs: f64, n: usize) -> (Vec<u8>, i32) {
    let (mut digits, mut exp) = digits_and_exp(abs);
    if digits.len() > n {
        let round_up = digits[n] >= 5;
        digits.truncate(n);
        if round_up {
            let mut i = digits.len();
            loop {
                if i == 0 {
                    digits.insert(0, 1);
                    exp += 1;
                    digits.truncate(n);
                    break;
                }
                i -= 1;
                if digits[i] == 9 {
                    digits[i] = 0;
                } else {
                    digits[i] += 1;
                    break;
                }
            }
        }
    } else {
        digits.resize(n, 0);
    }
    (digits, exp)
}

fn digits_to_string(digits: &[u8]) -> String {
    digits.iter().map(|d| (b'0' + d) as char).collect()
}

/// A stock or cost for a glanceable surface: compact, to a fixed
/// significant-figure width so columns stay aligned, with precision scaled to
/// magnitude so nothing carries decimals it has not earned.
/// @spec TUI-RENDER-012, TUI-RENDER-013, TUI-RENDER-017
pub fn compact(value: f64) -> String {
    if value.is_nan() || value.is_infinite() {
        return ">999Q".to_string();
    }
    if value <= 0.0 {
        return "0".to_string();
    }
    let abs = value;
    if abs < DISPLAY_FLOOR {
        return "0".to_string();
    }

    if abs < 1.0 {
        // Below one, there is no magnitude marker to spend width on: at most
        // three decimal places, rounded to the nearest, tie away from zero.
        let scaled = (abs * 1000.0).round();
        let scaled_i = scaled as i64;
        if scaled_i <= 0 {
            return "<0.001".to_string();
        }
        if scaled_i >= 1000 {
            // Rounds up to exactly one: no marker, no decimals.
            return "1".to_string();
        }
        return trim_decimal_noise(&format!("0.{scaled_i:03}"));
    }

    let (digits, exp) = round_to_digits(abs, 4);
    let group = (exp / 3) as usize;
    if group >= MAGNITUDE_SUFFIXES.len() {
        return ">999Q".to_string();
    }
    let before = (exp - group as i32 * 3 + 1) as usize;
    let whole = digits_to_string(&digits[..before]);
    let frac = trim_decimal_noise(&format!(".{}", digits_to_string(&digits[before..])));
    format!("{whole}{frac}{}", MAGNITUDE_SUFFIXES[group])
}

/// The same value unabbreviated, for the inspector: every digit of the whole
/// part, and a fraction rounded to the nearest at three places or the fourth
/// significant figure, whichever lies further, trailing zeros dropped.
/// @spec TUI-RENDER-012, TUI-RENDER-018
pub fn exact(value: f64) -> String {
    if !value.is_finite() {
        return "beyond reach".to_string();
    }
    if value == 0.0 {
        return "0".to_string();
    }
    let sign = value.is_sign_negative();
    let abs = value.abs();
    if abs < DISPLAY_FLOOR {
        return "0".to_string();
    }

    let (_, exp0) = digits_and_exp(abs);
    let desired = if exp0 >= 0 { (exp0 + 4) as usize } else { 4usize };
    let (digits, exp) = round_to_digits(abs, desired);

    let before = exp + 1;
    let (whole, frac) = if before <= 0 {
        ("0".to_string(), format!("{}{}", "0".repeat((-before) as usize), digits_to_string(&digits)))
    } else {
        let before = before as usize;
        (digits_to_string(&digits[..before]), digits_to_string(&digits[before..]))
    };
    let frac = frac.trim_end_matches('0');

    let mut out = String::new();
    if sign {
        out.push('-');
    }
    out.push_str(&whole);
    if !frac.is_empty() {
        out.push('.');
        out.push_str(frac);
    }
    out
}

/// A signed per-second rate for the sidebar: the compact form of the rate's
/// size, behind `+` above zero and `-` below, with no sign where the size
/// reads `0` — zero of either sign, or residue below the display floor.
/// @spec TUI-RENDER-012, TUI-RENDER-013
pub fn rate(per_second: f64) -> String {
    let size = compact(per_second.abs());
    if size == "0" {
        return size;
    }
    if per_second > 0.0 {
        format!("+{size}")
    } else {
        format!("-{size}")
    }
}

/// Display width of a single character, measured the way the renderer
/// measures it.
fn char_width(c: char) -> usize {
    Span::raw(c.to_string()).width()
}

/// A group header's line: two `─`, a space, the header's words, a space, and
/// `─` repeated to fill the rest of `width` — one line spanning the list's
/// whole width, carrying no cursor glyph, marker, count, or cost. Where
/// `width` cannot hold the leading rule, the words, and the trailing space,
/// the words are cut as `fit` cuts a name, and the result still fills `width`
/// exactly, so every one of its cells draws (TUI-RENDER-023).
pub fn group_header_line(words: &str, width: usize) -> String {
    const PREFIX: &str = "\u{2500}\u{2500} "; // "── "
    if width < PREFIX.chars().count() {
        return "\u{2500}".repeat(width);
    }
    let budget = width.saturating_sub(PREFIX.chars().count() + 1);
    let shown = fit(words, budget);
    let used = PREFIX.chars().count() + Span::raw(&shown).width();
    if used >= width {
        return format!("{PREFIX}{shown}");
    }
    let after_words = width - used;
    let dashes = after_words.saturating_sub(1);
    format!("{PREFIX}{shown} {}", "\u{2500}".repeat(dashes))
}

/// The Ideas section's done header: `done`, the number of rows the group
/// holds in parentheses, and ` · d to show` while folded or ` · d to fold`
/// while open, drawn as a group header. Where the list cannot hold the leading
/// rule, all those words, and a space, the key clause goes whole and the rest
/// is drawn as any header's words are. @spec TUI-NAV-111
pub fn done_header_line(count: usize, open: bool, width: usize) -> String {
    let short = format!("done ({count})");
    let full = format!("{short} \u{b7} d to {}", if open { "fold" } else { "show" });
    let leading = "\u{2500}\u{2500} ".chars().count();
    if width > leading + full.chars().count() {
        group_header_line(&full, width)
    } else {
        group_header_line(&short, width)
    }
}

/// Shorten a name to fit, with an explicit ellipsis so the player can see that
/// text was cut. The full name stays available in the inspector.
/// @spec TUI-RENDER-014
pub fn fit(name: &str, width: usize) -> String {
    let full_width = Span::raw(name).width();
    if full_width <= width {
        return name.to_string();
    }

    let ellipsis = '\u{2026}';
    let ellipsis_width = char_width(ellipsis);
    let budget = width.saturating_sub(ellipsis_width);

    let mut kept = String::new();
    let mut used = 0usize;
    for c in name.chars() {
        let cw = char_width(c);
        if used + cw > budget {
            break;
        }
        kept.push(c);
        used += cw;
    }

    if !kept.is_empty() && used + ellipsis_width <= width {
        kept.push(ellipsis);
        return kept;
    }

    // Not enough room for even one character plus the ellipsis: fall back to
    // a plain fit within the column, without claiming a cut it cannot show.
    let mut plain = String::new();
    let mut used = 0usize;
    for c in name.chars() {
        let cw = char_width(c);
        if used + cw > width {
            break;
        }
        plain.push(c);
        used += cw;
    }
    plain
}
