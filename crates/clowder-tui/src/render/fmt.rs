//! Numbers and names, fitted to the space available.
//!
//! Compact where glanced at, exact where read: every abbreviated number on
//! screen has an exact form one keypress away in the inspector, which is what
//! keeps whimsy from costing clarity. @spec TUI-RENDER-012, TUI-RENDER-013

use ratatui::text::Span;

/// Single-width magnitude markers, one per group of three decades. Four
/// significant figures, a decimal point, and a single-width marker is the
/// six-column budget a compact number may not exceed.
const MAGNITUDE_SUFFIXES: [&str; 6] = ["", "K", "M", "B", "T", "Q"];

/// Drop trailing zero decimals (and a dangling decimal point) so a rendering
/// never claims precision it does not carry.
fn trim_decimal_noise(s: &str) -> String {
    if !s.contains('.') {
        return s.to_string();
    }
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// A stock or cost for a glanceable surface: compact, to a fixed
/// significant-figure width so columns stay aligned, with precision scaled to
/// magnitude so nothing carries decimals it has not earned.
/// @spec TUI-RENDER-012, TUI-RENDER-013
pub fn compact(value: f64) -> String {
    if value == 0.0 {
        return "0".to_string();
    }

    let sign = if value < 0.0 { "-" } else { "" };
    let abs = value.abs();

    if abs < 1.0 {
        // Below one, there is no magnitude marker to spend width on: the
        // decimal point walks out to the first significant digit and four
        // significant figures follow it.
        let exp = abs.log10().floor() as i32;
        let leading_zeros = (-exp - 1).max(0) as usize;
        let decimals = leading_zeros + 4;
        let formatted = format!("{abs:.decimals$}");
        return format!("{sign}{}", trim_decimal_noise(&formatted));
    }

    let exp = abs.log10().floor() as i32;
    let group = ((exp / 3).max(0) as usize).min(MAGNITUDE_SUFFIXES.len() - 1);
    let divisor = 1000f64.powi(group as i32);
    let mantissa = abs / divisor;
    let digits_before = ((mantissa.log10().floor() as i32) + 1).clamp(1, 3) as usize;
    let decimals = 4usize.saturating_sub(digits_before);
    let formatted = format!("{mantissa:.decimals$}");

    format!(
        "{sign}{}{}",
        trim_decimal_noise(&formatted),
        MAGNITUDE_SUFFIXES[group]
    )
}

/// The same value unabbreviated, for the inspector. @spec TUI-RENDER-012
pub fn exact(value: f64) -> String {
    format!("{value}")
}

/// A signed per-second rate for the sidebar.
/// @spec TUI-RENDER-012, TUI-RENDER-013
pub fn rate(per_second: f64) -> String {
    let sign = if per_second < 0.0 { "-" } else { "+" };
    format!("{sign}{}", compact(per_second.abs()))
}

/// Display width of a single character, measured the way the renderer
/// measures it.
fn char_width(c: char) -> usize {
    Span::raw(c.to_string()).width()
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
