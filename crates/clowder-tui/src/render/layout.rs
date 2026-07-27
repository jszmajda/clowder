//! Where things go. Fixed chrome around a flexible body, stated as an algebra
//! rather than a table of sizes so the regions always sum to the terminal's
//! height and no arithmetic drifts as the chrome changes.
//! @spec TUI-RENDER-002, TUI-RENDER-004..008

use ratatui::layout::Rect;

/// The body's horizontal division, chosen by width.
/// @spec TUI-RENDER-004, TUI-RENDER-005, TUI-RENDER-006, TUI-RENDER-007
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

/// Compute the layout for an area.
///
/// `prev` carries the previous frame's layout so the breakpoints can apply an
/// asymmetric dead-band: a terminal sitting on a boundary must not flip back
/// and forth as the pane wobbles by a column. @spec TUI-RENDER-008
pub fn compute(_area: Rect, _prev: Option<&Layout>) -> Layout {
    todo!("TUI-RENDER-002..008: chrome algebra, breakpoints, dead-band")
}
