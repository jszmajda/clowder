//! The row model. Every player-facing command that belongs to a section is
//! reachable as a row in that section — mnemonics and the palette are pure
//! accelerators that gate nothing. @spec TUI-NAV-039

use clowder_core::{AdventureId, ColonyId, Command, ContentId, GameState, ResourceId, SiteId};

use super::action::CountSemantics;
use super::section::Section;

/// A row's stable identity. The cursor remembers *this*, never a list index,
/// so returning to a section restores the same item even after intervening
/// reveals inserted rows above it. @spec TUI-NAV-008
///
/// Jobs are addressed through `Content(ContentId::Job(..))` rather than a
/// variant of their own: two spellings of one row would make cursor-memory
/// equality miss whenever the two were used inconsistently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowKey {
    Content(ContentId),
    Resource(ResourceId),
    Colony(ColonyId),
    Site(SiteId),
    Adventure(Option<AdventureId>),
    /// A kitten by roster position. The roster is push-only for the life of a
    /// meadow — nothing is ever removed from it, which tenet 1 guarantees —
    /// so position is stable identity. A Move replaces the roster wholesale,
    /// and TUI-NAV-032's fallback covers that. @spec TUI-NAV-008
    Kitten(usize),
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

/// Build a section's rows from current state.
///
/// The single source of the row list: input resolution and drawing both read
/// it, so a row the cursor can reach is exactly a row that gets drawn. Two
/// builders would drift the moment either changed.
/// @spec TUI-NAV-009, TUI-NAV-039
pub fn rows(_state: &GameState, _section: Section) -> Vec<Row> {
    todo!("TUI-NAV-009/039: every sectioned command as a row, in group order")
}

/// Which sections have at least one revealed row, in digit order — the tab bar
/// exists only where content does. @spec TUI-NAV-006
pub fn revealed_sections(_state: &GameState) -> Vec<Section> {
    todo!("TUI-NAV-006: sections with any revealed row")
}
