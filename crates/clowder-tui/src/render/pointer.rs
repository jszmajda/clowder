//! The pointer record: where a frame drew what a pointer can address.
//!
//! Only the drawing knows where a frame put each thing, so each frame yields
//! a record of it as it draws, and a pointer event is resolved against the
//! record of the last frame drawn — never against a layout worked out again.
//! The record is an output of drawing and never an input to it.
//! @spec TUI-RENDER-025, TUI-RENDER-026

use ratatui::layout::Rect;

use crate::nav::{RowKey, Section};

/// What one frame drew that a pointer can address, and the terminal size it
/// was drawn at. A record names only what its frame drew.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PointerRecord {
    size: (u16, u16),
    tabs: Vec<(Section, Rect)>,
    rows: Vec<(Rect, Section, RowKey)>,
    list: Option<(Rect, Section)>,
    journal: Option<(Rect, Option<usize>)>,
}

fn within(r: &Rect, x: u16, y: u16) -> bool {
    r.x <= x && x < r.right() && r.y <= y && y < r.bottom()
}

impl PointerRecord {
    /// A record of a frame that drew nothing a pointer can address.
    pub(crate) fn blank(size: (u16, u16)) -> Self {
        PointerRecord { size, ..Self::default() }
    }

    pub(crate) fn record_tabs(&mut self, tabs: Vec<(Section, Rect)>) {
        self.tabs = tabs;
    }

    pub(crate) fn record_list(&mut self, space: Rect, section: Section, rows: Vec<(Rect, RowKey)>) {
        self.list = Some((space, section));
        self.rows = rows.into_iter().map(|(r, k)| (r, section, k)).collect();
    }

    pub(crate) fn record_journal(&mut self, region: Rect, max_scroll: Option<usize>) {
        self.journal = Some((region, max_scroll));
    }

    /// The terminal size the frame was drawn at.
    pub fn size(&self) -> (u16, u16) {
        self.size
    }

    /// Whether the frame drew nothing a pointer can address.
    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty() && self.rows.is_empty() && self.list.is_none() && self.journal.is_none()
    }

    /// The section whose tab label stands over this cell.
    pub fn tab_at(&self, x: u16, y: u16) -> Option<Section> {
        self.tabs.iter().find(|(_, r)| within(r, x, y)).map(|(s, _)| *s)
    }

    /// The section and identity of the row whose line this cell is on.
    pub fn row_at(&self, x: u16, y: u16) -> Option<(Section, RowKey)> {
        self.rows.iter().find(|(r, _, _)| within(r, x, y)).map(|(_, s, k)| (*s, k.clone()))
    }

    /// The section whose list's space this cell is in.
    pub fn list_at(&self, x: u16, y: u16) -> Option<Section> {
        self.list.as_ref().filter(|(r, _)| within(r, x, y)).map(|(_, s)| *s)
    }

    /// Whether this cell is in the Journal's region.
    pub fn journal_at(&self, x: u16, y: u16) -> bool {
        self.journal.as_ref().is_some_and(|(r, _)| within(r, x, y))
    }

    /// The scroll at which the Journal's last line stands at the foot of its
    /// region, or none where every line fits or no Journal was drawn.
    pub fn journal_max_scroll(&self) -> Option<usize> {
        self.journal.as_ref().and_then(|(_, m)| *m)
    }
}
