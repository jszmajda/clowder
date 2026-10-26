//! What a pointer event means: it moves around and does nothing more.
//!
//! A left press puts the cursor on a tab or a row; the wheel moves the cursor
//! through a list, or scrolls the Journal or the stores sidebar. No pointer
//! event ever becomes a
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
use crate::render::pointer::{PointerRecord, SidebarRecord};

/// How many stores a press of `{` or `}` moves the sidebar, its window
/// holding `drawn` stores: three, or one fewer than the window holds where
/// that is fewer, so the store at the window's leading edge stays in view —
/// and never fewer than one. @spec TUI-NAV-143
pub fn sidebar_step(drawn: usize) -> usize {
    3.min(drawn.saturating_sub(1)).max(1)
}

/// Move the sidebar `by` stores toward its last store (`down`) or its first,
/// from where `drawn` says the last frame drew it, stopping at either end.
/// Scrolled back to the first store, the view holds the top rather than a
/// store. Nothing changes where the sidebar has nowhere to go.
fn scroll_sidebar(view: &mut ViewState, drawn: &SidebarRecord, down: bool, by: usize) {
    let Some(max) = drawn.max_start else {
        return;
    };
    let next = if down {
        (drawn.start + by).min(max)
    } else {
        drawn.start.saturating_sub(by)
    };
    if next != drawn.start {
        view.sidebar_top = (next > 0).then(|| drawn.stores[next]);
    }
}

/// `{` and `}`: scroll the stores sidebar by the step, from where the last
/// frame drew it, which only that frame's record holds. True where `action`
/// is one of those keys, whether or not it moved. A scroll is neither an act
/// nor navigation: it ends no arming, retires no banner, counts as no visit,
/// and touches nothing but the sidebar's scroll. @spec TUI-NAV-140,
/// TUI-NAV-141, TUI-NAV-142, TUI-NAV-144
pub fn sidebar_key(action: &Action, view: &mut ViewState, record: Option<&PointerRecord>) -> bool {
    let down = match action {
        Action::SidebarDown => true,
        Action::SidebarUp => false,
        _ => return false,
    };
    if let Some(drawn) = record.and_then(PointerRecord::sidebar) {
        scroll_sidebar(view, drawn, down, sidebar_step(drawn.drawn));
    }
    true
}

/// The scroll anchor of the open overlay that scrolls — the Journal, or the
/// Achievements overlay, which scrolls as the Journal does (TUI-NAV-174).
fn scroll_anchor(view: &mut ViewState) -> Option<&mut usize> {
    match &mut view.overlay {
        Some(Overlay::Journal { anchor }) | Some(Overlay::Achievements { anchor }) => Some(anchor),
        _ => None,
    }
}

/// The open Journal's own scrolling by key — and the Achievements overlay's,
/// by the same keys: `↓`/`j` and `↑`/`k` one line, as a wheel step moves it,
/// `[` to its top and `]` to its foot, never past where the last frame drew
/// its last line — so the keys reach every line the wheel does. True where
/// either is open and `action` is one of those keys, whether or not it
/// moved; such a key ends no arming and leaves the banner showing.
/// @spec TUI-NAV-134, TUI-NAV-174
pub fn journal_key(action: &Action, view: &mut ViewState, record: Option<&PointerRecord>) -> bool {
    let Some(&mut anchor) = scroll_anchor(view) else {
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
    if let Some(anchor) = scroll_anchor(view) {
        *anchor = next;
    }
    true
}

/// Resolve one mouse event against `record`, the last frame drawn, at the
/// terminal's `present_size`. Always `Handled`: nothing here reaches the core.
/// @spec TUI-NAV-125, TUI-NAV-126, TUI-NAV-127, TUI-NAV-128, TUI-NAV-129,
/// TUI-NAV-130, TUI-NAV-131, TUI-NAV-132, TUI-NAV-133, TUI-NAV-145
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
        // The Journal scrolls under the wheel over it, and the Achievements
        // overlay as it does; neither takes a press. @spec TUI-NAV-174
        Some(Overlay::Journal { .. }) | Some(Overlay::Achievements { .. }) => {
            if let Some(step) = step.filter(|_| record.journal_at(x, y)) {
                let max = record.journal_max_scroll().unwrap_or(0);
                if let Some(anchor) = scroll_anchor(view) {
                    *anchor = if step > 0 {
                        (*anchor + 1).min(max)
                    } else {
                        anchor.saturating_sub(1)
                    };
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
    // The wheel over the sidebar scrolls it one store; a press there does
    // nothing. @spec TUI-NAV-145
    if record.sidebar_at(x, y) {
        if let (Some(step), Some(drawn)) = (step, record.sidebar()) {
            scroll_sidebar(view, drawn, step > 0, 1);
        }
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
