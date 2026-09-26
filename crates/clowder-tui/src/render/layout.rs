//! Where things go. Fixed chrome around a flexible body, stated as an algebra
//! rather than a table of sizes so the regions always sum to the terminal's
//! height and no arithmetic drifts as the chrome changes.
//! @spec TUI-RENDER-002, TUI-RENDER-004..008

use clowder_core::ResourceId;
use ratatui::layout::Rect;

use crate::render::fmt;

/// The body's horizontal division, chosen by width.
///
/// Deliberately unannotated as a whole: each variant carries the spec it
/// serves, and the widths that choose between them are annotated in `compute`,
/// which is where the rule lives. A blanket annotation here would name this
/// type as the implementation of four breakpoint specs, and it is the shape
/// they produce rather than the rule that picks one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Body {
    /// Under 80 columns or 24 rows. Nothing else is drawn — and the sim keeps
    /// ticking regardless. @spec TUI-RENDER-004
    TooSmall(Rect),
    /// 80–99: the inspector appears only in place of the list.
    /// @spec TUI-RENDER-005
    Narrow { sidebar: Rect, main: Rect },
    /// 100+: sidebar, list, and inspector side by side. At 120+ the list is
    /// capped and the surplus goes to the inspector.
    /// @spec TUI-RENDER-006, TUI-RENDER-007
    Wide { sidebar: Rect, list: Rect, inspector: Rect },
}

/// One frame's regions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    pub header: Rect,
    pub tabs: Rect,
    /// Present only while an unread away-report banner shows; it takes its row
    /// from the body rather than growing the frame. @spec TUI-RENDER-003
    pub banner: Option<Rect>,
    pub body: Body,
    /// One row at the floor; more only where width allows.
    pub ticker: Rect,
    pub hints: Rect,
}

/// Below this width or height, only the too-small card is drawn.
/// @spec TUI-RENDER-004
const FLOOR_W: u16 = 80;
const FLOOR_H: u16 = 24;

/// Where the inspector stops being a swap over the list and becomes a pane of
/// its own. @spec TUI-RENDER-005, TUI-RENDER-006
const PANE_BREAKPOINT: u16 = 100;

/// Where the list stops growing and hands surplus width to the inspector.
///
/// The navigation leaf states the same breakpoint from the other side — three
/// columns, list capped for scannability, surplus to the inspector — so both
/// are cited here rather than only the one whose folder this file sits in.
/// @spec TUI-RENDER-007, TUI-NAV-048
const CAP_BREAKPOINT: u16 = 120;

/// The asymmetric dead-band applied at both breakpoints: a layout is entered
/// at its boundary and left this many columns below it. Exact width deferred
/// to measurement (render design, Deferred 2); chosen small enough that the
/// cap breakpoint's dead-band cannot reach down into the 80-99 range that
/// TUI-RENDER-005 governs.
///
/// The navigation leaf asks for the same dead-band at the inspector's
/// breakpoint specifically; one band at every breakpoint satisfies both.
/// @spec TUI-RENDER-008, TUI-NAV-047
const DEAD_BAND: u16 = 4;

/// Turn an identifier-shaped tag into a spaced label — the same treatment
/// `render::humanize` gives a fixed row, kept local so this leaf does not
/// reach across into `mod.rs` for it.
fn humanize(tag: &str) -> String {
    let mut out = String::with_capacity(tag.len() + 4);
    for (i, c) in tag.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

/// The narrowest name column at which every two names in the resource
/// vocabulary that differ whole still differ when shortened to it — below
/// this, the sidebar's `fit` (TUI-RENDER-014) could make two different
/// stores read alike. Computed once: the vocabulary is fixed for the life of
/// the process. @spec TUI-RENDER-019
fn name_floor() -> usize {
    use std::sync::OnceLock;
    static FLOOR: OnceLock<usize> = OnceLock::new();
    *FLOOR.get_or_init(|| {
        let names: Vec<String> = ResourceId::ALL.iter().map(|r| humanize(&format!("{r:?}"))).collect();
        (1..=64)
            .find(|w| {
                names.iter().enumerate().all(|(i, a)| {
                    names.iter().skip(i + 1).all(|b| a == b || fmt::fit(a, *w) != fmt::fit(b, *w))
                })
            })
            .unwrap_or(64)
    })
}

/// The sidebar's width: at least seventeen columns for its bar line
/// (TUI-RENDER-019), and at least the stock-and-cap pair's thirteen columns,
/// a blank column, and the name floor beside them.
/// @spec TUI-RENDER-005, TUI-RENDER-006, TUI-RENDER-019
fn sidebar_width() -> u16 {
    17u16.max(13 + 1 + name_floor() as u16)
}

/// The minimum the inspector is left in the 100-119 range, which in turn
/// pins where the list's growth caps out at the 120 breakpoint. Exact value
/// deferred to measurement (render design, Deferred 1).
/// @spec TUI-RENDER-006, TUI-RENDER-007
const MIN_INSPECTOR_WIDTH: u16 = 20;

/// The list's width once capped: the width it has grown to by
/// `CAP_BREAKPOINT` columns, after which every extra column goes to the
/// inspector instead. @spec TUI-RENDER-007
fn capped_list_width() -> u16 {
    CAP_BREAKPOINT - sidebar_width() - MIN_INSPECTOR_WIDTH
}

/// The ticker's row count at the floor. Whether it earns more rows where
/// height allows is deferred (render design, Deferred 4).
const TICKER_ROWS: u16 = 1;

/// Compute the layout for an area.
///
/// `prev` carries the previous frame's layout so the breakpoints can apply an
/// asymmetric dead-band: a terminal sitting on a boundary must not flip back
/// and forth as the pane wobbles by a column. @spec TUI-RENDER-008
pub fn compute(area: Rect, prev: Option<&Layout>, banner: bool) -> Layout {
    // @spec TUI-RENDER-004
    if area.width < FLOOR_W || area.height < FLOOR_H {
        let zero = Rect::new(area.x, area.y, 0, 0);
        return Layout {
            header: zero,
            tabs: zero,
            banner: None,
            body: Body::TooSmall(area),
            ticker: zero,
            hints: zero,
        };
    }

    let w = area.width;
    let mut y = area.y;

    // @spec TUI-RENDER-002
    let header = Rect::new(area.x, y, w, 1);
    y += 1;
    let tabs = Rect::new(area.x, y, w, 1);
    y += 1;

    let ticker_rows = TICKER_ROWS;
    // TUI-RENDER-003: a banner takes its row from the BODY, never by growing
    // the frame — so the regions still sum to the terminal's height with one
    // showing. Computed here rather than in the caller, because the sum
    // invariant belongs to whatever owns the chrome algebra; carved anywhere
    // else, the one place that guarantees it stops being one place.
    let banner_rect = if banner { Some(Rect::new(area.x, y, w, 1)) } else { None };
    let banner_rows = if banner { 1 } else { 0 };
    let body_height = area.height - 2 - ticker_rows - 1 - banner_rows;
    let body_top = y + banner_rows;

    // @spec TUI-RENDER-005, TUI-RENDER-006, TUI-RENDER-008
    let prev_was_wide = matches!(prev.map(|p| &p.body), Some(Body::Wide { .. }));
    let pane_wide =
        w >= PANE_BREAKPOINT || (prev_was_wide && w >= PANE_BREAKPOINT - DEAD_BAND);

    let sidebar_w = sidebar_width();
    let body = if !pane_wide {
        let sidebar = Rect::new(area.x, body_top, sidebar_w, body_height);
        let main = Rect::new(area.x + sidebar_w, body_top, w - sidebar_w, body_height);
        Body::Narrow { sidebar, main }
    } else {
        // @spec TUI-RENDER-007, TUI-RENDER-008
        let capped_list_w = capped_list_width();
        let prev_was_capped = matches!(
            prev.map(|p| &p.body),
            Some(Body::Wide { list, .. }) if list.width == capped_list_w
        );
        let capped =
            w >= CAP_BREAKPOINT || (prev_was_capped && w >= CAP_BREAKPOINT - DEAD_BAND);

        let list_width = if capped { capped_list_w } else { w - sidebar_w - MIN_INSPECTOR_WIDTH };

        let sidebar = Rect::new(area.x, body_top, sidebar_w, body_height);
        let list = Rect::new(area.x + sidebar_w, body_top, list_width, body_height);
        let inspector_width = w - sidebar_w - list_width;
        let inspector = Rect::new(
            area.x + sidebar_w + list_width,
            body_top,
            inspector_width,
            body_height,
        );
        Body::Wide { sidebar, list, inspector }
    };

    y = body_top + body_height;
    let ticker = Rect::new(area.x, y, w, ticker_rows);
    y += ticker_rows;
    let hints = Rect::new(area.x, y, w, 1);

    // @spec TUI-RENDER-003
    Layout { header, tabs, banner: banner_rect, body, ticker, hints }
}
