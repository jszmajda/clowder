//! View state: everything the shell owns that the core does not. Nothing here
//! mirrors a value the core holds — if the shell needs a number the core
//! computes, it asks. @spec TUI-NAV-008, TUI-NAV-015, TUI-NAV-030

use std::collections::BTreeSet;
use std::time::Instant;

use clowder_core::{AchievementId, Command, ResourceId};

use super::rows::RowKey;
use super::section::Section;

/// A full-screen transient. Takes the cursor, section, and action keys for its
/// own navigation; speed, quit, and Esc stay live. @spec TUI-NAV-052
#[derive(Debug, Clone, PartialEq)]
pub enum Overlay {
    /// Scrollback over the retained journal. Holds its own scroll anchor.
    /// @spec TUI-NAV-026
    Journal {
        anchor: usize,
    },
    Help,
    /// The clowder's scrapbook of achievements. Holds its own scroll anchor,
    /// as the Journal does. @spec TUI-NAV-174
    Achievements {
        anchor: usize,
    },
    /// Find-anything over revealed content, narrowing by substring as
    /// characters arrive. @spec TUI-NAV-038
    Palette {
        query: String,
        selected: usize,
    },
}

/// A count being typed. While open, all printable input goes to the field —
/// which is what stops a digit jumping sections mid-type. @spec TUI-NAV-037
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CountPrompt {
    pub digits: String,
    /// The row this prompt was opened on, by identity — never the cursor's
    /// current position, so the prompt's Enter always reaches the row it was
    /// opened on, even if the cursor has since moved. `None` only in a
    /// default-constructed value nothing in this crate opens a prompt from.
    /// @spec TUI-NAV-072
    pub row: Option<RowKey>,
}

/// A heavy row awaiting its second Enter. Advisory only: it reserves nothing,
/// and the commit re-runs the real command so the core re-validates against
/// current state. @spec TUI-NAV-043
#[derive(Debug, Clone, PartialEq)]
pub struct ArmedRow {
    pub key: RowKey,
    /// The exact command the commit will re-run. A count-taking heavy row is
    /// armed *by* its count prompt (TUI-NAV-036), and rows are rebuilt every
    /// frame from state, so the typed count has nowhere else to survive
    /// between arming and committing. @spec TUI-NAV-036, TUI-NAV-043
    pub command: Command,
    /// Wall-clock expiry, silently. Deliberately not a sim tick: TUI-NAV-015
    /// says about eight *seconds*, and a tick-based expiry would last one
    /// second at the fastest speed and half a minute at the slowest.
    /// @spec TUI-NAV-015
    pub expires_at: Instant,
}

/// An unread bulk-time report awaiting its banner. Reports queue newest-first
/// and each clears on its own signal, so a single flag will not do.
/// @spec TUI-NAV-044, TUI-NAV-050
#[derive(Debug, Clone, PartialEq)]
pub struct PendingReport {
    /// Index into the journal's retained entries.
    pub entry: usize,
    pub dismissed: bool,
}

/// What a section looked like when last visited, so new content can re-badge
/// a section the player has already been to. A monotone "visited" flag could
/// only ever badge once. @spec TUI-NAV-006, TUI-NAV-020, TUI-NAV-044
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SeenMark {
    pub visited: bool,
    /// Count of revealed rows at last visit; anything beyond it is `★N new`.
    pub revealed_at_last_visit: usize,
}

/// The whole of the shell's own state.
#[derive(Debug, Clone, Default)]
pub struct ViewState {
    pub section: Option<Section>,
    /// Remembered focus per section, by identity. Indexed by
    /// `Section::index()`. @spec TUI-NAV-008
    pub focus: [Option<RowKey>; 10],
    /// Whether the player has called for the inspector. Whether that shows as
    /// a pane or a swap is a pure function of width, derived by layout — not
    /// stored, so it cannot disagree after a resize.
    /// @spec TUI-NAV-017, TUI-NAV-029
    pub inspector_wanted: bool,
    pub overlay: Option<Overlay>,
    pub prompt: Option<CountPrompt>,
    pub armed: Option<ArmedRow>,
    /// A transient footer note and its wall-clock expiry; the plain-language
    /// face of a rejection. @spec TUI-NAV-014
    pub footer: Option<(String, Instant)>,
    /// Per-section watermarks. @spec TUI-NAV-044
    pub seen: [SeenMark; 10],
    /// Unread reports, newest first. @spec TUI-NAV-050
    pub reports: Vec<PendingReport>,
    /// The nap length in years for a queued report that came from an
    /// accepted nap, keyed by the same journal-entry index `PendingReport`
    /// holds. The core's retained report does not itself say whether a nap
    /// or an absence folded it, so this is the one fact the banner needs
    /// that nothing else on `GameState` carries — read once, off the
    /// `Event::Napped` the commit returned, and kept here rather than
    /// re-derived, since it cannot be re-derived at all. @spec TUI-NAV-055
    pub nap_years: Vec<(usize, u32)>,
    /// Which journal entries the currently open Journal pins at its top —
    /// the reports unread at the moment it opened, newest first. Frozen for
    /// that Journal's whole lifetime: opening it reads every unread report
    /// at once (TUI-NAV-050), but the entry must not leave the place the
    /// player opened onto while they are still looking at it, so rendering
    /// reads this rather than which reports are unread *now*. Cleared when
    /// the Journal closes. @spec TUI-NAV-027, TUI-NAV-050
    pub journal_pinned: Vec<usize>,
    /// When the last Enter-driven action fired, for the ~10/s repeat limit.
    /// @spec TUI-NAV-010
    pub last_repeat: Option<Instant>,
    /// Set the instant a field's Enter acts on the row beneath it (fires,
    /// repeats, or jumps). While set, a held or auto-repeating continuation of
    /// that same press is swallowed rather than acting again; only a
    /// genuinely distinct Enter — judged the same way a heavy row's commit
    /// is — clears it and reaches the row. @spec TUI-NAV-061
    pub field_guard: bool,
    /// Whether the Ideas section's done group is open. Every sitting begins
    /// with it folded, nothing of it is saved, and it folds again whenever
    /// the group stops being listed. @spec TUI-NAV-112, TUI-NAV-113
    pub done_open: bool,
    /// The first store the sidebar is scrolled to, by identity, or `None` for
    /// the top. Every sitting begins at the top, and nothing of it is saved.
    pub sidebar_top: Option<ResourceId>,
    /// The achievements the player has been shown: the record as the game was
    /// loaded, before any catch-up replayed, and every one the Achievements
    /// overlay has drawn since. What the core reports earned beyond it is what
    /// `✦N earned` counts. Nothing of it is saved, so a restart begins it
    /// again from the record. @spec TUI-NAV-174, TUI-NAV-020
    pub achievements_seen: BTreeSet<AchievementId>,
    /// The achievements the open overlay marks ` new`: those unseen when it
    /// opened, and those earned while it stands open. Cleared when it closes.
    /// @spec TUI-NAV-175
    pub achievements_new: BTreeSet<AchievementId>,
}
