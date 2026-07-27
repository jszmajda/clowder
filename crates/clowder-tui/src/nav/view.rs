//! View state: everything the shell owns that the core does not. Nothing here
//! mirrors a value the core holds — if the shell needs a number the core
//! computes, it asks. @spec TUI-NAV-008, TUI-NAV-015, TUI-NAV-030

use super::rows::RowKey;
use super::section::Section;

/// Where the inspector is. @spec TUI-NAV-017
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectorMode {
    /// Wide terminal: a persistent pane, always live.
    Pane,
    /// Narrow terminal: hidden until Space swaps it in over the list.
    Swapped,
    Hidden,
}

/// A full-screen transient. Takes the cursor, section, and action keys for its
/// own navigation; speed, quit, and Esc stay live. @spec TUI-NAV-052
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Overlay {
    Journal,
    Help,
    Palette,
}

/// A count being typed. While open, all printable input goes to the field —
/// which is what stops a digit jumping sections mid-type. @spec TUI-NAV-037
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CountPrompt {
    pub digits: String,
}

/// A heavy row awaiting its second Enter. Advisory only: it reserves nothing,
/// and the commit re-runs the real command so the core re-validates against
/// current state. @spec TUI-NAV-043
#[derive(Debug, Clone, PartialEq)]
pub struct ArmedRow {
    pub key: RowKey,
    /// Tick at which arming lapses, silently. @spec TUI-NAV-015
    pub expires_at: u64,
}

/// The whole of the shell's own state.
#[derive(Debug, Clone, Default)]
pub struct ViewState {
    pub section: Option<Section>,
    /// Remembered focus per section, by identity. Indexed by
    /// `Section::index()`. @spec TUI-NAV-008
    pub focus: [Option<RowKey>; 10],
    pub inspector: Option<InspectorMode>,
    pub overlay: Option<Overlay>,
    pub prompt: Option<CountPrompt>,
    pub armed: Option<ArmedRow>,
    /// A transient footer note; the plain-language face of a rejection.
    /// @spec TUI-NAV-014
    pub footer: Option<(String, u64)>,
    /// Sections visited at least once — clears their new-content badge.
    /// @spec TUI-NAV-044
    pub visited: [bool; 10],
    pub banner_dismissed: bool,
}
