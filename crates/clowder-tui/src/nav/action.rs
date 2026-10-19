//! What a key resolves to. Keys map to actions; actions become core commands
//! or move the view. Nothing here decides whether a command will succeed —
//! the core is the single arbiter. @spec TUI-NAV-002, TUI-NAV-014

use super::section::Section;

/// A resolved intent, before it becomes a core command or a view change.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    CursorUp,
    CursorDown,
    SectionNext,
    SectionPrev,
    SectionJump(Section),
    GroupNext,
    GroupPrev,
    /// Enter: the focused row's primary action once, or a commit of an armed
    /// row. @spec TUI-NAV-002, TUI-NAV-015
    Activate,
    ToggleInspector,
    /// `x` — open the count prompt. @spec TUI-NAV-011
    CountPrompt,
    /// `X` — act to the maximum affordable. @spec TUI-NAV-012
    MaxAffordable,
    Increment,
    Decrement,
    OpenPalette,
    OpenJournal,
    OpenHelp,
    ObserveSky,
    SpeedUp,
    SpeedDown,
    /// A lowercase letter outside the global table, in a context where a field
    /// or full-screen overlay does not hold the keyboard: acts on whichever
    /// row it is a live mnemonic for, or nothing if it is not one right now.
    /// @spec TUI-NAV-046
    Mnemonic(char),
    /// `n` — a kitten's own row-bound key, standing for no row of its own:
    /// while the cursor is on a kitten's row and the core reports the command
    /// gate of AppointElder for that kitten open, issues AppointElder for it;
    /// everywhere else it is an unbound key. @spec TUI-NAV-094
    AppointElder,
    /// `d` — open or fold the Ideas section's done group; an unbound key
    /// wherever the group is not live. @spec TUI-NAV-115
    ToggleDone,
    /// `{` — scroll the stores sidebar toward its first store.
    SidebarUp,
    /// `}` — scroll the stores sidebar toward its last store.
    SidebarDown,
    /// A printable character routed into an open field rather than to its
    /// global binding — which is exactly what stops a digit jumping sections
    /// mid-type. @spec TUI-NAV-037, TUI-NAV-038
    Type(char),
    /// Erase one character from an open field. @spec TUI-NAV-037
    Erase,
    /// Esc: close the topmost transient only. @spec TUI-NAV-030
    Dismiss,
    Quit,
    /// A bound key that is not meaningful right now — earns a footer note,
    /// unlike an unbound key, which is silent. @spec TUI-NAV-042
    NotNow(&'static str),
    /// Unbound in this context: ignored, no footer. @spec TUI-NAV-042
    Ignored,
}

/// How a row's command takes a count. The distinction is the row's, not the
/// key's: `x` passes N to a count-taking command and repeat-calls a repeat-N
/// one. @spec TUI-NAV-031
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CountSemantics {
    /// The count is a real argument — squads, caravans, years.
    CountTaking,
    /// The shell calls the command N times; partial success is intended.
    RepeatN,
    /// Neither counted nor usefully repeated. @spec TUI-NAV-041
    Single,
}
