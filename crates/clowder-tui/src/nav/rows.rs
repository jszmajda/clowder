//! The row model. Every player-facing command that belongs to a section is
//! reachable as a row in that section — mnemonics and the palette are pure
//! accelerators that gate nothing. @spec TUI-NAV-039

use clowder_core::{ColonyId, Command, ContentId, JobId, ResourceId};

use super::action::CountSemantics;

/// A row's stable identity. The cursor remembers *this*, never a list index,
/// so returning to a section restores the same item even after intervening
/// reveals inserted rows above it. @spec TUI-NAV-008
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowKey {
    Content(ContentId),
    Job(JobId),
    Resource(ResourceId),
    Colony(ColonyId),
    /// A kitten by arrival sequence — never by roster index, which shifts.
    Kitten(u64),
    /// A group header or a fixed row that is not content.
    Fixed(&'static str),
}

/// What a row does when acted on.
#[derive(Debug, Clone, PartialEq)]
pub enum RowKind {
    /// Fires a command. @spec TUI-NAV-002
    Act { command: Command, semantics: CountSemantics },
    /// `+`/`-` adjust a count. @spec TUI-NAV-013
    Count { increment: Command, decrement: Command },
    /// Inspectable but not actionable — a store, an album entry.
    Display,
    /// A group header; skipped by the cursor, jumped to by `[` and `]`.
    Group,
}

/// One line in a section's list.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub key: RowKey,
    pub kind: RowKind,
    /// Whether this row's action is heavy: armed by the first Enter and
    /// committed only by a distinct second one, never by auto-repeat, and
    /// never reachable by `x` or `X`. @spec TUI-NAV-015, TUI-NAV-051
    pub heavy: bool,
}

impl Row {
    /// Whether the cursor may land here. @spec TUI-NAV-032
    pub fn selectable(&self) -> bool {
        !matches!(self.kind, RowKind::Group)
    }
}
