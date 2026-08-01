//! The key map. Only the tmux-safe set is ever bound: plain printable
//! characters, Tab/BackTab, Enter, Esc, Space, Backspace, and the arrows, plus
//! Ctrl-C as emergency quit. Uppercase is the modifier tier — Shift emits
//! plain printable characters, which tmux never intercepts.
//! @spec TUI-NAV-001, TUI-NAV-004

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::action::Action;
use super::section::Section;
use super::view::{Overlay, ViewState};

/// Resolve a key to an action in the current context. Context matters: a digit
/// jumps sections on a bare screen and types into an open prompt.
/// @spec TUI-NAV-001, TUI-NAV-002, TUI-NAV-003, TUI-NAV-004, TUI-NAV-005,
/// TUI-NAV-006, TUI-NAV-007, TUI-NAV-028, TUI-NAV-030, TUI-NAV-033,
/// TUI-NAV-037, TUI-NAV-040, TUI-NAV-042, TUI-NAV-046, TUI-NAV-052
pub fn resolve(key: KeyEvent, view: &ViewState) -> Action {
    // Ctrl-C is the one chord, and it quits from every context, even mid-type
    // in a field. @spec TUI-NAV-001, TUI-NAV-052
    if is_ctrl_c(key) {
        return Action::Quit;
    }

    // Nothing outside the tmux-safe set is ever bound, in any context.
    // @spec TUI-NAV-001
    if !is_tmux_safe(key) {
        return Action::Ignored;
    }

    // Esc closes the topmost transient only, and is a no-op with nothing
    // open. No Esc-prefixed sequence exists — resolution holds no state to
    // build one from — so a late Esc under tmux escape-time can never act,
    // quit, or lose anything. @spec TUI-NAV-030
    if key.code == KeyCode::Esc {
        return Action::Dismiss;
    }

    let field_open = view.prompt.is_some() || matches!(view.overlay, Some(Overlay::Palette { .. }));

    // A field captures the keyboard: every printable character (and digit)
    // routes into it, and every global single-key binding is suspended.
    // @spec TUI-NAV-037
    if field_open {
        return match key.code {
            KeyCode::Enter => Action::Activate,
            KeyCode::Backspace => Action::Erase,
            KeyCode::Char(c) => Action::Type(c),
            _ => Action::Ignored,
        };
    }

    // A full-screen overlay (Journal or help) takes the cursor, section and
    // action keys for its own navigation; the speed keys, q, Ctrl-C and Esc
    // stay live (Esc and Ctrl-C are already handled above).
    // @spec TUI-NAV-052
    if view.overlay.is_some() {
        return match key.code {
            KeyCode::Char('<') => Action::SpeedDown,
            KeyCode::Char('>') => Action::SpeedUp,
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Down | KeyCode::Char('j') => Action::CursorDown,
            KeyCode::Up | KeyCode::Char('k') => Action::CursorUp,
            KeyCode::Char('[') => Action::GroupPrev,
            KeyCode::Char(']') => Action::GroupNext,
            _ => Action::Ignored,
        };
    }

    // The global key table: a focused screen with nothing transient open (a
    // bare screen, or one with an armed row / report banner awaiting Esc).
    // @spec TUI-NAV-002, TUI-NAV-003, TUI-NAV-004, TUI-NAV-005, TUI-NAV-006,
    // TUI-NAV-007, TUI-NAV-028, TUI-NAV-033, TUI-NAV-040, TUI-NAV-042,
    // TUI-NAV-046
    match key.code {
        KeyCode::Up => Action::CursorUp,
        KeyCode::Char('k') => Action::CursorUp,
        KeyCode::Down => Action::CursorDown,
        KeyCode::Char('j') => Action::CursorDown,
        KeyCode::Tab => Action::SectionNext,
        KeyCode::BackTab => Action::SectionPrev,
        KeyCode::Char('l') => Action::SectionNext,
        KeyCode::Char('h') => Action::SectionPrev,
        KeyCode::Right => Action::SectionNext,
        KeyCode::Left => Action::SectionPrev,
        KeyCode::Char('[') => Action::GroupPrev,
        KeyCode::Char(']') => Action::GroupNext,
        KeyCode::Enter => Action::Activate,
        KeyCode::Char(' ') => Action::ToggleInspector,
        KeyCode::Char('i') => Action::ToggleInspector,
        KeyCode::Char('x') => Action::CountPrompt,
        KeyCode::Char('X') => Action::MaxAffordable,
        KeyCode::Char('+') => Action::Increment,
        KeyCode::Char('=') => Action::Increment,
        KeyCode::Char('-') => Action::Decrement,
        KeyCode::Backspace => Action::Decrement,
        KeyCode::Char('/') => Action::OpenPalette,
        KeyCode::Char('e') => Action::OpenJournal,
        KeyCode::Char('o') => Action::ObserveSky,
        KeyCode::Char('<') => Action::SpeedDown,
        KeyCode::Char('>') => Action::SpeedUp,
        KeyCode::Char('?') => Action::OpenHelp,
        KeyCode::Char('q') => Action::Quit,
        KeyCode::Char(c) => match Section::from_digit(c) {
            Some(section) => Action::SectionJump(section),
            None => Action::Ignored,
        },
        _ => Action::Ignored,
    }
}

fn is_ctrl_c(key: KeyEvent) -> bool {
    key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL
}

/// Whether a key is inside the tmux-safe set at all. Nothing outside it is
/// ever bound. Plain printable characters (Shift included, however the
/// terminal reports it), Tab/BackTab, Enter, Esc, Space, Backspace, and the
/// four arrows. @spec TUI-NAV-001
pub fn is_tmux_safe(key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char(c) => {
            let plain_or_shifted =
                key.modifiers == KeyModifiers::NONE || key.modifiers == KeyModifiers::SHIFT;
            plain_or_shifted && (' '..='~').contains(&c)
        }
        KeyCode::Tab | KeyCode::BackTab => {
            key.modifiers == KeyModifiers::NONE || key.modifiers == KeyModifiers::SHIFT
        }
        KeyCode::Enter
        | KeyCode::Esc
        | KeyCode::Backspace
        | KeyCode::Up
        | KeyCode::Down
        | KeyCode::Left
        | KeyCode::Right => key.modifiers == KeyModifiers::NONE,
        _ => false,
    }
}
