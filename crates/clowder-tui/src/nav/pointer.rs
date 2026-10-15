//! What a pointer event means: it moves around and does nothing more.
//!
//! A left press puts the cursor on a tab or a row; the wheel moves the cursor
//! through a list, or scrolls the Journal. No pointer event ever becomes a
//! core command, opens or closes anything, or retires the report banner —
//! tmux hands the program the click that merely focused its pane. Events are
//! resolved against the record of the last frame drawn, never a layout worked
//! out again. @spec TUI-NAV-125, TUI-NAV-126, TUI-NAV-127

use clowder_core::GameState;
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

use super::action::Action;
use super::rows::{cursor_index, drawn_rows, has_tab};
use super::update::{enter_section, guard_after_field, move_cursor, Outcome};
use super::view::{Overlay, ViewState};
use crate::render::pointer::PointerRecord;

/// The open Journal's own scrolling by key: `↓`/`j` and `↑`/`k` one line, as a
/// wheel step moves it, `[` to its top and `]` to its foot, never past where
/// the last frame drew its last line — so the keys reach every line the wheel
/// does. True where the Journal is open and `action` is one of those keys,
/// whether or not it moved; such a key ends no arming and leaves the banner
/// showing. @spec TUI-NAV-134
pub fn journal_key(action: &Action, view: &mut ViewState, record: Option<&PointerRecord>) -> bool {
    let Some(Overlay::Journal { anchor }) = view.overlay else {
        return false;
    };
    let max = record
        .and_then(PointerRecord::journal_max_scroll)
        .unwrap_or(0);
    let next = match action {
        Action::CursorDown => (anchor + 1).min(max),
        Action::CursorUp => anchor.saturating_sub(1),
        Action::GroupPrev => 0,
        Action::GroupNext => max,
        _ => return false,
    };
    view.overlay = Some(Overlay::Journal { anchor: next });
    true
}

/// Resolve one mouse event against `record`, the last frame drawn, at the
/// terminal's `present_size`. Always `Handled`: nothing here reaches the core.
/// @spec TUI-NAV-125, TUI-NAV-126, TUI-NAV-127, TUI-NAV-128, TUI-NAV-129,
/// TUI-NAV-130, TUI-NAV-131, TUI-NAV-132, TUI-NAV-133
pub fn pointer_event(
    event: MouseEvent,
    view: &mut ViewState,
    state: &GameState,
    record: &PointerRecord,
    present_size: (u16, u16),
) -> Outcome {
    // Only an unmodified left press and a vertical wheel step answer.
    if event.modifiers != KeyModifiers::NONE {
        return Outcome::Handled;
    }
    let step = match event.kind {
        MouseEventKind::Down(MouseButton::Left) => None,
        MouseEventKind::ScrollDown => Some(1isize),
        MouseEventKind::ScrollUp => Some(-1isize),
        _ => return Outcome::Handled,
    };
    // A record describes the size it was drawn at and no other.
    if record.size() != present_size {
        return Outcome::Handled;
    }
    let (x, y) = (event.column, event.row);

    match &view.overlay {
        // The Journal scrolls under the wheel over it; it takes no press.
        Some(Overlay::Journal { anchor }) => {
            if let Some(step) = step.filter(|_| record.journal_at(x, y)) {
                let max = record.journal_max_scroll().unwrap_or(0);
                let next = if step > 0 {
                    (anchor + 1).min(max)
                } else {
                    anchor.saturating_sub(1)
                };
                if next != *anchor {
                    view.overlay = Some(Overlay::Journal { anchor: next });
                }
            }
            return Outcome::Handled;
        }
        // Help, the palette and a count prompt hold everything where it is.
        Some(_) => return Outcome::Handled,
        None => {}
    }
    if view.prompt.is_some() {
        return Outcome::Handled;
    }
    let Some(focus) = view.section else {
        return Outcome::Handled;
    };

    match step {
        None => {
            if let Some(tab) = record.tab_at(x, y) {
                // A visit, as the section's digit makes it, the banner left showing.
                if tab != focus && has_tab(state, view.section, tab) {
                    view.armed = None;
                    enter_section(view, state, tab);
                    guard_after_field(view);
                }
            } else if let Some((section, key)) = record.row_at(x, y) {
                if section != focus {
                    return Outcome::Handled;
                }
                let list = drawn_rows(state, section, view);
                let idx = section.index();
                let here = cursor_index(&list, view.focus[idx].as_ref()).map(|i| &list[i].key);
                let listed = list.iter().any(|r| r.selectable() && r.key == key);
                if listed && here != Some(&key) {
                    view.armed = None;
                    view.focus[idx] = Some(key);
                    guard_after_field(view);
                }
            }
        }
        Some(step) => {
            if record.list_at(x, y) == Some(focus) {
                let before = view.focus[focus.index()].clone();
                move_cursor(view, state, step);
                if view.focus[focus.index()] != before {
                    view.armed = None;
                    guard_after_field(view);
                }
            }
        }
    }
    Outcome::Handled
}
