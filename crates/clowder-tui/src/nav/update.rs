//! The reducer: an action plus current state becomes a view change, a core
//! command, or both.
//!
//! Key resolution is deliberately state-light — it answers "what did the
//! player ask for", not "what happens". This is where the two meet, because
//! the answer depends on the focused row, and rows come from game state.
//! @spec TUI-NAV-002, TUI-NAV-011, TUI-NAV-015, TUI-NAV-037, TUI-NAV-038,
//! TUI-NAV-041, TUI-NAV-051, TUI-NAV-058, TUI-NAV-060, TUI-NAV-061,
//! TUI-NAV-064, TUI-NAV-065, TUI-NAV-050, TUI-NAV-083, TUI-NAV-084,
//! TUI-NAV-112, TUI-NAV-113, TUI-NAV-114, TUI-NAV-116, TUI-NAV-118,
//! TUI-NAV-120, TUI-NAV-121

use std::time::{Duration, Instant};

use clowder_core::{Command, GameState, LockReason};

use super::action::{Action, CountSemantics};
use super::mnemonics;
use super::palette;
use super::rows::{
    done_count, drawn_rows, has_tab, ideas_place, lists_selectable, rows, tab_sections, Row,
    RowKey, RowKind,
};
use super::section::Section;
use super::view::{ArmedRow, CountPrompt, Overlay, SeenMark, ViewState};

/// What the loop should do with an action once the view has absorbed it.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// Nothing further; the view changed (or deliberately did not).
    Handled,
    /// Apply this to the core, once.
    Apply(Command),
    /// Apply this to the core up to `n` times, stopping at the first
    /// rejection — partial success is intended. @spec TUI-NAV-031
    Repeat(Command, u32),
    /// Save and exit. @spec TUI-NAV-016
    Quit,
}

/// How long a footer note stays on screen. @spec TUI-NAV-014
const FOOTER_DURATION: Duration = Duration::from_secs(4);

/// How long an armed row stays armed before silently expiring. @spec TUI-NAV-015
const ARM_DURATION: Duration = Duration::from_secs(8);

/// The minimum gap between two Enters for the second to count as a distinct,
/// deliberate press rather than the same held key still auto-repeating. Also
/// doubles as the ~10/s repeat-rate cap for ordinary rows. @spec TUI-NAV-010,
/// TUI-NAV-051
const REPEAT_GATE: Duration = Duration::from_millis(100);

/// The shell-side cap on how many times `X` may repeat-call a command, and on
/// the count prompt's own count (TUI-NAV-068) — the same cap, so the prompt
/// always shows the count `X` would have used. @spec TUI-NAV-012, TUI-NAV-068
pub const MAX_AFFORDABLE_CAP: u32 = 10_000;

/// Fold an action into the view, yielding whatever the core must be told.
///
/// Heavy rows never reach `Apply` on a first Enter — they arm, and only a
/// distinct second Enter commits. @spec TUI-NAV-015, TUI-NAV-051
pub fn update(action: Action, view: &mut ViewState, state: &GameState) -> Outcome {
    // Quitting works from anywhere and commits nothing on the way out.
    // @spec TUI-NAV-016
    if action == Action::Quit {
        return Outcome::Quit;
    }

    fold_unlisted_done(view, state);

    match action {
        Action::CursorUp
        | Action::CursorDown
        | Action::SectionNext
        | Action::SectionPrev
        | Action::SectionJump(_)
        | Action::GroupNext
        | Action::GroupPrev => {
            // A full-screen overlay (the Journal, the scrapbook, or help) takes the cursor and
            // group keys for its own scrolling — their navigation meaning is
            // suspended, so they neither move the section cursor nor end an
            // arming beneath the overlay. @spec TUI-NAV-076
            if view.overlay.is_some()
                && matches!(
                    action,
                    Action::CursorUp | Action::CursorDown | Action::GroupNext | Action::GroupPrev
                )
            {
                return Outcome::Handled;
            }
            // Any navigation silently disarms whatever was armed, and ends any
            // field-accept guard still standing over the row it left — and
            // retires whichever report banner is currently showing, whether
            // or not the press actually moves anything: a digit for an empty
            // section or a group jump with no groups is still a navigation
            // key, and the player's return to the place they pressed it from
            // is the same either way. @spec TUI-NAV-015, TUI-NAV-061, TUI-NAV-083
            view.armed = None;
            view.field_guard = false;
            dismiss_active_report(view);
            handle_navigation(action, view, state);
            Outcome::Handled
        }
        Action::Activate => handle_activate(view, state),
        Action::CountPrompt => handle_count_prompt(view, state),
        Action::MaxAffordable => handle_max_affordable(view, state),
        Action::Increment => handle_increment_decrement(view, state, true),
        Action::Decrement => handle_increment_decrement(view, state, false),
        Action::Mnemonic(c) => handle_mnemonic(view, state, c),
        Action::AppointElder => handle_appoint_elder(view, state),
        Action::ToggleDone => handle_toggle_done(view, state),
        // A sidebar scroll moves from where the last frame drew the sidebar,
        // which only that frame's record holds (`sidebar_key`, which the loop
        // runs first); with no record there is nothing to move from, and the
        // view stands. @spec TUI-NAV-144
        Action::SidebarUp | Action::SidebarDown => Outcome::Handled,
        Action::Type(c) => handle_type(view, c),
        Action::Erase => handle_erase(view),
        Action::Dismiss => handle_dismiss(view),
        Action::Quit => unreachable!("handled above"),

        // Whether Space or i sets or clears the call, or only answers with a
        // footer note, depends on the layout on screen at the press — the
        // same thing Esc's swap step depends on — which only the loop holds.
        // `toggle_inspector` below is the layout-aware version the loop
        // actually runs Space and i through; called from within `update`,
        // which holds no drawn layout to judge by, neither the narrow layout
        // nor the pane is on screen, so the call is left exactly as it stood,
        // as `dismiss` above defaults its own layout-judged step.
        // @spec TUI-NAV-088
        Action::ToggleInspector => toggle_inspector(view, false, false),
        // Opening the Journal reads every unread report at once — not just
        // the one showing — since every pinned entry is right there to read
        // the moment it opens. What was unread is frozen as this Journal's
        // own pinned set before any of it is marked read, so the entries
        // stay exactly where the player opened onto them for as long as this
        // Journal stays open. @spec TUI-NAV-026, TUI-NAV-025, TUI-NAV-038,
        // TUI-NAV-050
        Action::OpenJournal => {
            view.journal_pinned = view
                .reports
                .iter()
                .filter(|r| !r.dismissed)
                .map(|r| r.entry)
                .collect();
            for r in &mut view.reports {
                r.dismissed = true;
            }
            open_overlay(view, Overlay::Journal { anchor: 0 })
        }
        // @spec TUI-NAV-174
        Action::Achievements => {
            if matches!(view.overlay, Some(Overlay::Achievements { .. })) {
                view.overlay = None;
                view.achievements_new.clear();
            } else {
                view.achievements_new.clear();
                view.overlay = Some(Overlay::Achievements { anchor: 0 });
                show_achievements(view, state);
            }
            Outcome::Handled
        }
        Action::OpenHelp => open_overlay(view, Overlay::Help),
        Action::OpenPalette => open_overlay(
            view,
            Overlay::Palette {
                query: String::new(),
                selected: 0,
            },
        ),
        // @spec TUI-NAV-040
        Action::ObserveSky => {
            if state.pending_sky_event.is_some() {
                Outcome::Apply(Command::ObserveSky)
            } else {
                set_footer(view, "nothing in the sky just now");
                Outcome::Handled
            }
        }
        // @spec TUI-NAV-028
        Action::SpeedUp => nudge_speed(view, state, true),
        Action::SpeedDown => nudge_speed(view, state, false),

        // @spec TUI-NAV-042
        Action::NotNow(note) => {
            set_footer(view, note);
            Outcome::Handled
        }
        Action::Ignored => Outcome::Handled,
    }
}

// ---------------------------------------------------------------------------
// Opening a sitting
// ---------------------------------------------------------------------------

/// Runs once, before a sitting's first frame — a new game or a load, either
/// one. Enters the Meadow as a visit (TUI-NAV-024, TUI-NAV-032's landing
/// rule), and takes every tab the load found already showing as known: each
/// such section is marked seen at its current row count, the same mark a
/// real visit leaves, so nothing present at the opening sits behind a badge
/// or counts toward the new-content chip. Only a tab that *appears* after
/// this moment is new. Not itself a navigation key, so it retires no report
/// banner — a load's own report is queued separately and stays unread until
/// the returning player actually looks. @spec TUI-NAV-024, TUI-NAV-006
pub fn opening(view: &mut ViewState, state: &GameState) {
    for section in Section::ALL {
        if lists_selectable(state, section) {
            let idx = section.index();
            view.seen[idx] = SeenMark {
                visited: true,
                revealed_at_last_visit: rows(state, section).len(),
            };
        }
    }
    view.done_open = false;
    // What a sitting opens on was earned before it; a load whose absence
    // earned more says so itself (TUI-NAV-174).
    view.achievements_seen = state.achievements.keys().copied().collect();
    enter_section(view, state, Section::Meadow);
}

/// While the Achievements overlay is open, mark every achievement the core
/// reports earned as seen, and each that was unseen as ` new` for as long as
/// the overlay stays open. Run on opening and every frame, independent of
/// input, so one earned beneath the open overlay is seen as it is drawn.
/// @spec TUI-NAV-174, TUI-NAV-175
pub fn show_achievements(view: &mut ViewState, state: &GameState) {
    if !matches!(view.overlay, Some(Overlay::Achievements { .. })) {
        return;
    }
    for id in state.achievements.keys() {
        if view.achievements_seen.insert(*id) {
            view.achievements_new.insert(*id);
        }
    }
}

/// A tab's badge (TUI-NAV-006) tracks whether its section has been visited
/// *since it last appeared*, not merely once ever: a tab that leaves the bar
/// and returns is new information again, so its section's watermark is
/// cleared the moment the tab leaves — ready to badge again the moment a row
/// brings it back. Called every frame, independent of input, the same as
/// `expire_unlisted_arming`: the tab bar can change with no key pressed at
/// all, and the badge has to notice the moment it does. @spec TUI-NAV-006
pub fn reset_hidden_tabs(view: &mut ViewState, state: &GameState) {
    for section in Section::ALL {
        if !has_tab(state, view.section, section) {
            view.seen[section.index()].visited = false;
        }
    }
}

// ---------------------------------------------------------------------------
// Navigation
// ---------------------------------------------------------------------------

fn handle_navigation(action: Action, view: &mut ViewState, state: &GameState) {
    match action {
        // @spec TUI-NAV-005: the digit of a section with nothing in it yet is
        // a soft no-op, not a doorway to an empty room.
        Action::SectionJump(section) => {
            if has_tab(state, view.section, section) {
                enter_section(view, state, section);
            } else {
                set_footer(view, "nothing there yet");
            }
        }
        Action::CursorUp => move_cursor(view, state, -1),
        Action::CursorDown => move_cursor(view, state, 1),
        // @spec TUI-NAV-007, TUI-NAV-033
        Action::SectionNext => cycle_section(view, state, 1),
        Action::SectionPrev => cycle_section(view, state, -1),
        // @spec TUI-NAV-007
        Action::GroupNext => move_to_group(view, state, 1),
        Action::GroupPrev => move_to_group(view, state, -1),
        _ => {}
    }
}

/// Move the cursor by `delta` selectable rows, stopping at either end. Group
/// headers are skipped: they are signposts, not destinations.
/// @spec TUI-NAV-002, TUI-NAV-007
pub(super) fn move_cursor(view: &mut ViewState, state: &GameState, delta: isize) {
    let Some(section) = view.section else { return };
    let list = drawn_rows(state, section, view);
    let selectable: Vec<&Row> = list.iter().filter(|r| r.selectable()).collect();
    if selectable.is_empty() {
        return;
    }
    let idx = section.index();
    let here = view.focus[idx]
        .as_ref()
        .and_then(|k| selectable.iter().position(|r| &r.key == k))
        .unwrap_or(0) as isize;
    let next = (here + delta).clamp(0, selectable.len() as isize - 1) as usize;
    view.focus[idx] = Some(selectable[next].key.clone());
}

/// Step to the next or previous *revealed* section, wrapping around the ring —
/// so no screen is the end of the road. @spec TUI-NAV-007, TUI-NAV-033
fn cycle_section(view: &mut ViewState, state: &GameState, delta: isize) {
    let ring = tab_sections(state, view.section);
    if ring.is_empty() {
        return;
    }
    let here = view
        .section
        .and_then(|s| ring.iter().position(|r| *r == s))
        .unwrap_or(0) as isize;
    let n = ring.len() as isize;
    let next = ((here + delta) % n + n) % n;
    enter_section(view, state, ring[next as usize]);
}

/// Jump to the next or previous group header's first selectable row — the
/// long-list affordance for lists too tall to scroll through a row at a time.
/// @spec TUI-NAV-007
fn move_to_group(view: &mut ViewState, state: &GameState, delta: isize) {
    let Some(section) = view.section else { return };
    let list = drawn_rows(state, section, view);
    let idx = section.index();
    let here = view.focus[idx]
        .as_ref()
        .and_then(|k| list.iter().position(|r| &r.key == k))
        .unwrap_or(0);

    // Every group header's first selectable row, in list order.
    let heads: Vec<usize> = list
        .iter()
        .enumerate()
        .filter(|(_, r)| matches!(r.kind, RowKind::Group))
        .filter_map(|(i, _)| {
            list.iter()
                .enumerate()
                .skip(i + 1)
                .find(|(_, r)| r.selectable())
                .map(|(j, _)| j)
        })
        .collect();
    if heads.is_empty() {
        return;
    }
    let target = if delta > 0 {
        heads
            .iter()
            .copied()
            .find(|&h| h > here)
            .or_else(|| heads.first().copied())
    } else {
        heads
            .iter()
            .copied()
            .rev()
            .find(|&h| h < here)
            .or_else(|| heads.last().copied())
    };
    if let Some(t) = target {
        view.focus[idx] = Some(list[t].key.clone());
    }
}

/// Open an overlay, closing whatever was open. @spec TUI-NAV-052
fn open_overlay(view: &mut ViewState, overlay: Overlay) -> Outcome {
    view.overlay = Some(overlay);
    Outcome::Handled
}

/// Step one notch along the speed ladder. From a value that is not on the
/// ladder, the first press snaps to the nearest notch in the pressed
/// direction; at either end a further press does nothing.
/// @spec TUI-NAV-028
fn nudge_speed(view: &mut ViewState, state: &GameState, up: bool) -> Outcome {
    const LADDER: [f64; 6] = [0.25, 0.5, 1.0, 2.0, 4.0, 8.0];
    let now = state.speed;
    let next = if up {
        LADDER.iter().copied().find(|n| *n > now + 1e-9)
    } else {
        LADDER.iter().copied().rev().find(|n| *n < now - 1e-9)
    };
    match next {
        Some(v) => Outcome::Apply(Command::SetSpeed(v)),
        None => {
            set_footer(
                view,
                if up {
                    "already at the fastest"
                } else {
                    "already at the slowest"
                },
            );
            Outcome::Handled
        }
    }
}

/// Enter `section`, restoring the remembered item by identity, or landing on
/// the first actionable row, or the first row at all. This is navigation's one
/// shared doorway — every path that changes `view.section` runs through it —
/// so it is also where a visit marks the section seen, the same for a section
/// cycle, a digit jump, or a palette landing. Retiring the showing report
/// banner is every caller's own concern rather than this one's: the top-level
/// navigation dispatch already does it once for a section cycle or a digit
/// jump, and doing it again here would retire a *second* report out from
/// under a caller that already retired one this same press.
/// @spec TUI-NAV-008, TUI-NAV-032
pub(super) fn enter_section(view: &mut ViewState, state: &GameState, section: Section) {
    view.section = Some(section);
    let listed = rows(state, section).len();
    let list = drawn_rows(state, section, view);
    let idx = section.index();

    // A visit marks the section seen and clears its new-content watermark.
    view.seen[idx] = SeenMark {
        visited: true,
        revealed_at_last_visit: listed,
    };
    view.focus[idx] = landing_for(state, section, &list, view.focus[idx].as_ref());
}

/// The remembered row if it is still one of `list`'s selectable rows, or
/// else the first selectable one that is available, or else — nothing on
/// the list is affordable — simply the first selectable row: a section is
/// never entered with no cursor at all. A remembered row that has finished
/// into the folded done group gives its place to the row that closes up into
/// it. `list` is the drawn list. @spec TUI-NAV-008, TUI-NAV-032, TUI-NAV-117
fn landing_for(
    state: &GameState,
    section: Section,
    list: &[Row],
    remembered: Option<&RowKey>,
) -> Option<RowKey> {
    let remembered_still_present =
        remembered.is_some_and(|k| list.iter().any(|r| r.selectable() && &r.key == k));
    if remembered_still_present {
        return remembered.cloned();
    }
    if let Some(gone) = remembered.filter(|k| folded_away(state, section, list, k)) {
        return closes_up(list, gone);
    }
    list.iter()
        .find(|r| r.selectable() && is_available(state, section, r))
        .or_else(|| list.iter().find(|r| r.selectable()))
        .map(|r| r.key.clone())
}

/// While the section in focus lists no selectable row, TUI-NAV-085 keeps the
/// cursor's memory exactly as it was, so the same row is recognised if it
/// returns. The moment a selectable row is listed there again, the cursor
/// lands on it the same way entering the section would — the remembered row
/// if it is one of them, or the landing rule otherwise. Called every frame,
/// independent of input, since a row can return from a tick or a loaded
/// command with no key pressed at all; a section actually left and
/// re-entered lands through `enter_section`, the ordinary doorway, and never
/// reaches here. @spec TUI-NAV-085, TUI-NAV-008, TUI-NAV-032
///
/// The same pass hands the cursor on when the row it rests on has finished
/// into the folded done group, and fences that move off from a held press.
/// @spec TUI-NAV-117, TUI-NAV-119, TUI-NAV-120, TUI-NAV-112
pub fn relist_focus(view: &mut ViewState, state: &GameState) {
    fold_unlisted_done(view, state);
    let Some(section) = view.section else { return };
    let idx = section.index();
    let list = drawn_rows(state, section, view);
    let remembered_still_present = view.focus[idx]
        .as_ref()
        .is_some_and(|k| list.iter().any(|r| r.selectable() && &r.key == k));
    if remembered_still_present {
        return;
    }
    if let Some(gone) = view.focus[idx]
        .clone()
        .filter(|k| folded_away(state, section, &list, k))
    {
        view.focus[idx] = closes_up(&list, &gone);
        // No held or auto-repeating continuation of the press that finished
        // the row may act on the row the cursor was handed to.
        guard_after_field(view);
        return;
    }
    if !list.iter().any(|r| r.selectable()) {
        return;
    }
    view.focus[idx] = landing_for(state, section, &list, view.focus[idx].as_ref());
}

/// Whether `key` is a row of the Ideas section that is listed but not drawn:
/// one folded away in the done group. @spec TUI-NAV-113, TUI-NAV-117
fn folded_away(state: &GameState, section: Section, drawn: &[Row], key: &RowKey) -> bool {
    section == Section::Ideas
        && ideas_place(key).is_some()
        && !drawn.iter().any(|r| &r.key == key)
        && rows(state, section).iter().any(|r| &r.key == key)
}

/// The drawn row that closes up into `gone`'s place in the Ideas list: the
/// first drawn row following that place, else the last drawn row before it,
/// else none. @spec TUI-NAV-117
fn closes_up(drawn: &[Row], gone: &RowKey) -> Option<RowKey> {
    let place = ideas_place(gone)?;
    let placed: Vec<(&Row, (usize, usize))> = drawn
        .iter()
        .filter(|r| r.selectable())
        .filter_map(|r| Some((r, ideas_place(&r.key)?)))
        .collect();
    placed
        .iter()
        .find(|(_, p)| *p > place)
        .or_else(|| placed.iter().rev().find(|(_, p)| *p < place))
        .map(|(r, _)| r.key.clone())
}

/// A done group that is no longer listed is folded, so it comes back folded.
/// @spec TUI-NAV-112
fn fold_unlisted_done(view: &mut ViewState, state: &GameState) {
    if view.done_open && done_count(state) == 0 {
        view.done_open = false;
    }
}

/// `d` — opens the Ideas section's done group where it is folded and folds it
/// where it is open, once per distinct press, judged as a distinct Enter is.
/// It issues no command and is no navigation: it ends no arming, retires no
/// banner, counts as no visit, and leaves the layout alone. Where it is not
/// live it is an unbound key. @spec TUI-NAV-115, TUI-NAV-116
fn handle_toggle_done(view: &mut ViewState, state: &GameState) -> Outcome {
    let live = view.section == Some(Section::Ideas)
        && view.overlay.is_none()
        && view.prompt.is_none()
        && done_count(state) > 0;
    if !live {
        return Outcome::Handled;
    }
    let now = Instant::now();
    let distinct = view
        .last_repeat
        .is_none_or(|last| now.duration_since(last) >= REPEAT_GATE);
    view.last_repeat = Some(now);
    if !distinct {
        return Outcome::Handled;
    }

    let idx = Section::Ideas.index();
    view.done_open = !view.done_open;
    let drawn = drawn_rows(state, Section::Ideas, view);
    let on_drawn_row = view.focus[idx]
        .as_ref()
        .is_some_and(|k| drawn.iter().any(|r| r.selectable() && &r.key == k));
    if !on_drawn_row {
        view.focus[idx] = if view.done_open {
            landing_for(state, Section::Ideas, &drawn, None)
        } else {
            drawn
                .iter()
                .rev()
                .find(|r| r.selectable())
                .map(|r| r.key.clone())
        };
    }
    Outcome::Handled
}

/// Retires the one report banner currently showing — the first still-
/// undismissed entry in the newest-first queue — leaving every other queued
/// report exactly where it was, for its own turn. @spec TUI-NAV-050, TUI-NAV-083
fn dismiss_active_report(view: &mut ViewState) {
    if let Some(r) = view.reports.iter_mut().find(|r| !r.dismissed) {
        r.dismissed = true;
    }
}

/// Whether `row` is one TUI-NAV-032 lands on first: the core reports its item
/// Available and, in the Ideas section, the item is not finished — a finished
/// item stands only in the open done group, which no landing prefers.
/// @spec TUI-NAV-032, TUI-NAV-120
fn is_available(state: &GameState, section: Section, row: &Row) -> bool {
    match &row.key {
        RowKey::Content(id, "") => {
            state.availability(*id) == clowder_core::Availability::Available
                && !(section == Section::Ideas
                    && state.lock_reason(*id) == Some(LockReason::AlreadyYours))
        }
        _ => true,
    }
}

// ---------------------------------------------------------------------------
// Row lookup
// ---------------------------------------------------------------------------

/// The row currently under the cursor, if any.
fn current_row(view: &ViewState, state: &GameState) -> Option<Row> {
    let section = view.section?;
    let key = view.focus[section.index()].clone()?;
    drawn_rows(state, section, view)
        .into_iter()
        .find(|r| r.key == key)
}

/// The row `key` names, if it is still listed in the current section — never
/// the cursor's own position. Used to hold a count prompt to the row it was
/// opened on. @spec TUI-NAV-072
fn by_key(view: &ViewState, state: &GameState, key: &RowKey) -> Option<Row> {
    let section = view.section?;
    rows(state, section).into_iter().find(|r| &r.key == key)
}

/// Whether `row` is a one-shot the player already owns. @spec TUI-NAV-041
fn owned_one_shot(state: &GameState, row: &Row) -> bool {
    match &row.key {
        RowKey::Content(id, "") => state.lock_reason(*id) == Some(LockReason::AlreadyYours),
        _ => false,
    }
}

fn set_footer(view: &mut ViewState, message: impl Into<String>) {
    view.footer = Some((message.into(), Instant::now() + FOOTER_DURATION));
}

/// Enter, x, X, +/=, and -/Backspace all resolve to "the row under the
/// cursor"; with the focused section listing none, each of them finds
/// nothing and says so the same way, rather than reading as a lost keypress.
/// @spec TUI-NAV-085
fn no_row_to_act_on(view: &mut ViewState) -> Outcome {
    set_footer(view, "there's nothing here to act on");
    Outcome::Handled
}

// ---------------------------------------------------------------------------
// Activate (Enter) — TUI-NAV-002, TUI-NAV-015, TUI-NAV-051, TUI-NAV-036
// ---------------------------------------------------------------------------

fn handle_activate(view: &mut ViewState, state: &GameState) -> Outcome {
    // A field's accepting Enter is spent on the field; a held or
    // auto-repeating continuation of that same press must not also act on
    // the row it returns to — only a genuinely distinct later Enter does, the
    // guard renewing on every press so a continuous hold never wears it down.
    // @spec TUI-NAV-061
    if view.field_guard {
        let now = Instant::now();
        let distinct = view
            .last_repeat
            .is_none_or(|last| now.duration_since(last) >= REPEAT_GATE);
        if !distinct {
            view.last_repeat = Some(now);
            return Outcome::Handled;
        }
        // A real gap: the guard has done its job, and this Enter is free to
        // act on the row beneath — decided below, from the unchanged
        // `last_repeat` this same distinctness check just read.
        view.field_guard = false;
    }

    // The count prompt's Enter sets the count and, on a heavy row, arms it;
    // on an ordinary row it fires (or repeat-fires) at once. @spec TUI-NAV-011,
    // TUI-NAV-036
    if view.prompt.is_some() {
        return accept_count_prompt(view, state);
    }
    // The palette's Enter jumps to its top result, or does nothing with none
    // to aim at. @spec TUI-NAV-038, TUI-NAV-060, TUI-NAV-064, TUI-NAV-065
    if matches!(view.overlay, Some(Overlay::Palette { .. })) {
        return accept_palette(view, state);
    }

    let Some(row) = current_row(view, state) else {
        return no_row_to_act_on(view);
    };
    activate_row(view, state, &row)
}

/// Acts on `row` exactly as Enter on it would: the owned check, then dispatch
/// by row kind, arming rather than firing a heavy one. Shared by Enter's own
/// activation of the cursor's row and by a live mnemonic, which reaches the
/// same row without moving the cursor there. @spec TUI-NAV-002, TUI-NAV-046
fn activate_row(view: &mut ViewState, state: &GameState, row: &Row) -> Outcome {
    if owned_one_shot(state, row) {
        set_footer(view, "You already have that.");
        return Outcome::Handled;
    }

    match &row.kind {
        RowKind::Act { command, .. } => {
            if row.heavy {
                activate_heavy(view, &row.key, *command)
            } else {
                fire_once(view, *command)
            }
        }
        RowKind::Count { increment, .. } => fire_once(view, *increment),
        RowKind::Display | RowKind::Group => Outcome::Handled,
    }
}

/// A live mnemonic performs its row's action exactly as Enter would: it looks
/// up the row the letter stands for — never the row under the cursor — so
/// the press moves neither the cursor nor the focused section. Bound only
/// where `resolve` reaches the global table at all, so a field or full-screen
/// overlay never sees this action; an unbound or not-currently-live letter is
/// silently ignored, the same as any other key with no meaning here.
/// @spec TUI-NAV-046, TUI-NAV-081
fn handle_mnemonic(view: &mut ViewState, state: &GameState, c: char) -> Outcome {
    let Some(section) = view.section else {
        return Outcome::Handled;
    };
    let focus = view.focus[section.index()].clone();
    let Some(target) = mnemonics::target(state, section, focus.as_ref(), c) else {
        return Outcome::Handled;
    };
    let Some(row) = by_key(view, state, &target) else {
        return Outcome::Handled;
    };
    activate_row(view, state, &row)
}

/// `n` — the row-bound key TUI-NAV-093 leaves AppointElder to. While the
/// cursor sits on a kitten's row and the core reports that kitten's
/// AppointElder gate open, it issues AppointElder for that kitten exactly as
/// `fire_once` would: once, rate-limited on the same clock Enter's own row
/// actions share, moving neither the cursor nor the focused section. On any
/// other row, on a row-less screen, or with the gate shut, it is an unbound
/// key — handled, but silent, asking the core for nothing.
/// @spec TUI-NAV-094
fn handle_appoint_elder(view: &mut ViewState, state: &GameState) -> Outcome {
    let Some(row) = current_row(view, state) else {
        return Outcome::Handled;
    };
    let RowKey::Kitten(ix, _) = row.key else {
        return Outcome::Handled;
    };
    let command = Command::AppointElder(ix);
    if !state.command_gate_open(&command) {
        return Outcome::Handled;
    }
    fire_once(view, command)
}

/// Fire an ordinary row's command once, rate-limited to ~10/s so a held key
/// cannot outrun the shell's own pace. The cursor never moves on action.
/// @spec TUI-NAV-010
fn fire_once(view: &mut ViewState, command: Command) -> Outcome {
    let now = Instant::now();
    if let Some(last) = view.last_repeat {
        if now.duration_since(last) < REPEAT_GATE {
            // A press dropped while a fence stands renews the clock, so a
            // continuous hold never wears the fence down. @spec TUI-NAV-119,
            // TUI-NAV-133
            if view.field_guard {
                view.last_repeat = Some(now);
            }
            return Outcome::Handled;
        }
    }
    view.field_guard = false;
    view.last_repeat = Some(now);
    Outcome::Apply(command)
}

/// The armed-row state machine at the heart of TUI-NAV-015/051: the first
/// Enter arms, only a distinct second Enter (one arriving after the repeat
/// gate has passed) commits, and a held/auto-repeating Enter can never widen
/// that gap into a commit.
fn activate_heavy(view: &mut ViewState, key: &RowKey, command: Command) -> Outcome {
    let now = Instant::now();

    let already_armed_here = view
        .armed
        .as_ref()
        .is_some_and(|a| &a.key == key && now < a.expires_at);

    if already_armed_here {
        // The commit re-runs the command the row was armed with — not a
        // freshly rebuilt one — so a count typed into the DeepNap prompt
        // before arming survives to the commit. @spec TUI-NAV-036, TUI-NAV-043
        let armed_command = view.armed.as_ref().expect("checked Some above").command;
        let distinct = view
            .last_repeat
            .is_none_or(|last| now.duration_since(last) >= REPEAT_GATE);
        view.last_repeat = Some(now);

        if distinct {
            view.armed = None;
            return Outcome::Apply(armed_command);
        }
        // A held or auto-repeating key: still armed, no commit.
        return Outcome::Handled;
    }

    // First press on this row (or the previous arming expired, or a
    // different row was armed): arm this one, disarming any other.
    view.armed = Some(ArmedRow {
        key: key.clone(),
        command,
        expires_at: now + ARM_DURATION,
    });
    view.last_repeat = Some(now);
    Outcome::Handled
}

/// The count prompt's Enter: sets the count, then either fires/repeats
/// (ordinary row) or arms (heavy row, i.e. DeepNap). An empty or zero count
/// cancels with no effect. The prompt belongs to the row it was opened on, by
/// that row's identity, never to whatever the cursor reads by the time Enter
/// arrives; if that row has left its list since — the sim keeps ticking
/// under an open prompt — the prompt closes with no effect and says so.
/// @spec TUI-NAV-011, TUI-NAV-031, TUI-NAV-036, TUI-NAV-070, TUI-NAV-072
fn accept_count_prompt(view: &mut ViewState, state: &GameState) -> Outcome {
    let prompt = view.prompt.take().expect("checked Some above");
    let n: u32 = prompt.digits.parse().unwrap_or(0);

    let Some(row) = prompt.row.as_ref().and_then(|key| by_key(view, state, key)) else {
        set_footer(view, "that row isn't here anymore.");
        return Outcome::Handled;
    };

    if n == 0 {
        // A cancel is still an Enter the field spends on itself: the guard
        // must stand so a held or auto-repeating continuation of this same
        // press cannot fall through and act on the row beneath.
        // @spec TUI-NAV-060, TUI-NAV-061
        guard_after_field(view);
        return Outcome::Handled;
    }

    match &row.kind {
        RowKind::Act { command, semantics } => match semantics {
            CountSemantics::CountTaking => {
                let counted = counted_command(*command, n);
                if row.heavy {
                    view.armed = Some(ArmedRow {
                        key: row.key.clone(),
                        command: counted,
                        expires_at: Instant::now() + ARM_DURATION,
                    });
                    view.last_repeat = Some(Instant::now());
                    Outcome::Handled
                } else {
                    guard_after_field(view);
                    Outcome::Apply(counted)
                }
            }
            CountSemantics::RepeatN => {
                guard_after_field(view);
                Outcome::Repeat(*command, n)
            }
            CountSemantics::Single => Outcome::Handled,
        },
        RowKind::Count { increment, .. } => {
            guard_after_field(view);
            Outcome::Repeat(*increment, n)
        }
        RowKind::Display | RowKind::Group => Outcome::Handled,
    }
}

/// The palette's Enter: with a result to jump to, close it and land the
/// cursor there — the jump is navigation, so it disarms, dismisses the report
/// banner, and counts as a visit to the section it lands in, the same as any
/// other move; it never performs a row action. With nothing aimed at (no
/// query, one of only spaces, or no match), the palette stays open exactly as
/// typed. @spec TUI-NAV-038, TUI-NAV-060, TUI-NAV-064, TUI-NAV-065
fn accept_palette(view: &mut ViewState, state: &GameState) -> Outcome {
    let Some(Overlay::Palette { query, .. }) = &view.overlay else {
        return Outcome::Handled;
    };
    // A blank query (empty, or nothing but spaces) aims at nothing — it lists
    // every result without searching, so Enter must not jump the way a real
    // aimed match does. @spec TUI-NAV-064
    if query.chars().all(|c| c == ' ') {
        return Outcome::Handled;
    }
    let Some(top) = palette::results(state, query).into_iter().next() else {
        return Outcome::Handled;
    };
    let (section, key) = (top.section, top.key);

    // A jump to a row folded away in the done group opens the group, so the
    // row it focuses is drawn. @spec TUI-NAV-121
    if section == Section::Ideas
        && !drawn_rows(state, section, view)
            .iter()
            .any(|r| r.key == key)
    {
        view.done_open = true;
    }

    view.overlay = None;
    view.armed = None;
    // The landing is navigation like any other: it disarms, retires the
    // showing report banner, and runs through the same shared doorway that
    // marks the section seen. @spec TUI-NAV-065, TUI-NAV-083
    dismiss_active_report(view);
    enter_section(view, state, section);
    view.focus[section.index()] = Some(key);

    guard_after_field(view);
    Outcome::Handled
}

/// Guard the row a field just acted on: the continuation of the very Enter
/// that accepted the field must not also act on the row it returns to.
/// @spec TUI-NAV-061
pub(super) fn guard_after_field(view: &mut ViewState) {
    view.field_guard = true;
    view.last_repeat = Some(Instant::now());
}

/// Substitute a typed count into a count-taking command, keeping every other
/// field. `DeepNap` and `Pounce` are the count-taking commands rows offer.
fn counted_command(command: Command, n: u32) -> Command {
    match command {
        Command::DeepNap(_) => Command::DeepNap(n),
        Command::Pounce(_) => Command::Pounce(n),
        Command::Trade(colony, _) => Command::Trade(colony, n),
        other => other,
    }
}

// ---------------------------------------------------------------------------
// x — count prompt — TUI-NAV-011, TUI-NAV-036, TUI-NAV-041
// ---------------------------------------------------------------------------

/// Open the count prompt on a row that takes a count, with no affordability
/// check of the shell's own: the core judges the count when Enter sends it.
/// @spec TUI-NAV-011, TUI-NAV-071
fn handle_count_prompt(view: &mut ViewState, state: &GameState) -> Outcome {
    let Some(row) = current_row(view, state) else {
        return no_row_to_act_on(view);
    };

    if owned_one_shot(state, &row) {
        set_footer(view, "You already have that.");
        return Outcome::Handled;
    }

    let countable = match &row.kind {
        RowKind::Act { semantics, .. } => {
            matches!(
                semantics,
                CountSemantics::CountTaking | CountSemantics::RepeatN
            )
        }
        RowKind::Count { .. } => true,
        RowKind::Display | RowKind::Group => false,
    };

    if !countable {
        set_footer(view, "There's no count to set here.");
        return Outcome::Handled;
    }

    view.prompt = Some(CountPrompt {
        digits: String::new(),
        row: Some(row.key.clone()),
    });
    Outcome::Handled
}

// ---------------------------------------------------------------------------
// X — max affordable — TUI-NAV-012, TUI-NAV-041, TUI-NAV-051
// ---------------------------------------------------------------------------

/// Act to the maximum affordable. The entry point for the behaviour, which is
/// why the annotation is here rather than only on the action variant and the
/// shell-side cap that this reads.
///
/// Held, `X` is paced the same as any other held key — no more than ten runs
/// a second, whatever the terminal's key-repeat rate — by the same gate and
/// the same `last_repeat` clock ordinary rows use; a press that lands inside
/// the gate is dropped in full, before the row or its ownership is even
/// looked at, with no footer note of its own.
/// @spec TUI-NAV-012
fn handle_max_affordable(view: &mut ViewState, state: &GameState) -> Outcome {
    let now = Instant::now();
    if let Some(last) = view.last_repeat {
        if now.duration_since(last) < REPEAT_GATE {
            // A press dropped while the fold's fence stands renews the clock,
            // so a continuous hold never wears the fence down. @spec TUI-NAV-119
            if view.field_guard {
                view.last_repeat = Some(now);
            }
            return Outcome::Handled;
        }
    }
    view.field_guard = false;

    let Some(row) = current_row(view, state) else {
        return no_row_to_act_on(view);
    };

    if owned_one_shot(state, &row) {
        set_footer(view, "You already have that.");
        return Outcome::Handled;
    }

    if row.heavy {
        // X may arm a row but must never be the press that commits it.
        // @spec TUI-NAV-051
        set_footer(view, "That takes a deliberate Enter, not X.");
        return Outcome::Handled;
    }

    match &row.kind {
        RowKind::Act { command, semantics } => match semantics {
            CountSemantics::CountTaking | CountSemantics::RepeatN => {
                view.last_repeat = Some(now);
                Outcome::Repeat(*command, MAX_AFFORDABLE_CAP)
            }
            CountSemantics::Single => {
                set_footer(view, "There's no count to set here.");
                Outcome::Handled
            }
        },
        RowKind::Count { increment, .. } => {
            view.last_repeat = Some(now);
            Outcome::Repeat(*increment, MAX_AFFORDABLE_CAP)
        }
        RowKind::Display | RowKind::Group => {
            set_footer(view, "There's no count to set here.");
            Outcome::Handled
        }
    }
}

// ---------------------------------------------------------------------------
// +/- — count rows — TUI-NAV-013
// ---------------------------------------------------------------------------

/// One worker on or off a count row. The entry point for the behaviour, which
/// is why the annotation is here rather than on the row field that merely
/// declares a count row exists.
/// @spec TUI-NAV-013
fn handle_increment_decrement(view: &mut ViewState, state: &GameState, up: bool) -> Outcome {
    let Some(row) = current_row(view, state) else {
        return no_row_to_act_on(view);
    };
    match &row.kind {
        RowKind::Count {
            increment,
            decrement,
        } => fire_once(view, if up { *increment } else { *decrement }),
        _ => Outcome::Handled,
    }
}

// ---------------------------------------------------------------------------
// The count prompt's own field — TUI-NAV-011, TUI-NAV-037
// ---------------------------------------------------------------------------

/// The count prompt takes digits only, and only up to the same cap that
/// bounds `X`: a digit that would carry the count past it is refused, with a
/// footer note, so the prompt's count never reads higher than `X` would ever
/// run. The palette takes every printable character, digits and Space
/// included, verbatim into its query. @spec TUI-NAV-037, TUI-NAV-068
fn handle_type(view: &mut ViewState, c: char) -> Outcome {
    if let Some(prompt) = view.prompt.as_mut() {
        if c.is_ascii_digit() {
            let candidate = format!("{}{c}", prompt.digits);
            match candidate.parse::<u64>() {
                Ok(n) if n <= MAX_AFFORDABLE_CAP as u64 => prompt.digits = candidate,
                _ => {
                    set_footer(
                        view,
                        format!("that count goes no higher than {MAX_AFFORDABLE_CAP}"),
                    );
                }
            }
        }
        return Outcome::Handled;
    }
    if let Some(Overlay::Palette { query, .. }) = view.overlay.as_mut() {
        query.push(c);
    }
    Outcome::Handled
}

/// Backspace deletes the open field's last character, doing nothing on an
/// empty field and never reaching a row's own global meaning.
/// @spec TUI-NAV-058
fn handle_erase(view: &mut ViewState) -> Outcome {
    if let Some(prompt) = view.prompt.as_mut() {
        prompt.digits.pop();
        return Outcome::Handled;
    }
    if let Some(Overlay::Palette { query, .. }) = view.overlay.as_mut() {
        query.pop();
    }
    Outcome::Handled
}

// ---------------------------------------------------------------------------
// Row departure — TUI-NAV-076
// ---------------------------------------------------------------------------

/// Ends an arming whose row is no longer listed in its section — the row
/// leaving is navigation by other means (the cursor has already lost it,
/// TUI-NAV-032), so the arming ends the same as any other navigation would,
/// and the next Enter is an ordinary press on whatever is focused now. Called
/// every frame, independent of input, since a tick or a loaded command can
/// remove a row out from under an arming with no key pressed at all.
/// @spec TUI-NAV-076
pub fn expire_unlisted_arming(view: &mut ViewState, state: &GameState) {
    if let Some(armed) = &view.armed {
        if !super::rows::row_still_listed(state, &armed.key) {
            view.armed = None;
        }
    }
}

// ---------------------------------------------------------------------------
// Esc — TUI-NAV-030
// ---------------------------------------------------------------------------

/// Closes only the topmost transient: a field (the count prompt, or the
/// palette — a field under this rule, not a full-screen overlay) first, then
/// any other overlay, then an armed row, then nothing further — so Esc over a
/// field open above an armed row closes only the field, leaving the row
/// beneath it exactly as armed. Called from within `update`, which holds no
/// drawn layout to judge the narrow-mode swap by, so it never reaches past an
/// armed row; `dismiss` below is the layout-aware version the loop actually
/// runs Esc through, and reduces to exactly this over a field, overlay, or
/// armed row. @spec TUI-NAV-030, TUI-NAV-037
fn handle_dismiss(view: &mut ViewState) -> Outcome {
    dismiss(view, false)
}

/// Esc's full order of topmost transients, the report banner included: a
/// field, then any other overlay, then an armed row, then — only once none of
/// those are open — the narrow-mode inspector swap, and last the report
/// banner showing. The swap is not a field or an overlay in `ViewState`'s own
/// terms; whether it is even in effect is judged by the layout the previous
/// frame actually drew — `narrow_on_screen` — rather than the terminal's raw
/// width, so a resize sitting inside the breakpoint's dead-band (TUI-NAV-047)
/// keeps whichever layout is already on screen; only the caller holding that
/// drawn layout can say, which is why this takes `narrow_on_screen` where the
/// reducer's other actions do not. A pane at any width is not a swap and Esc
/// never touches it — there is nothing above the banner there to close.
/// @spec TUI-NAV-017, TUI-NAV-030, TUI-NAV-047, TUI-NAV-083
pub fn dismiss(view: &mut ViewState, narrow_on_screen: bool) -> Outcome {
    if view.prompt.take().is_some() {
        return Outcome::Handled;
    }
    if view.overlay.take().is_some() {
        // Whatever the Journal pinned at its top was pinned for that
        // Journal's own lifetime; closing it ends that hold, so the reports
        // it read take their ordinary place in the scrollback from here.
        // @spec TUI-NAV-027, TUI-NAV-050
        view.journal_pinned.clear();
        // And what the Achievements overlay marked new was new to that
        // opening alone. @spec TUI-NAV-175
        view.achievements_new.clear();
        return Outcome::Handled;
    }
    if view.armed.take().is_some() {
        return Outcome::Handled;
    }
    if narrow_on_screen && view.inspector_wanted {
        view.inspector_wanted = false;
        return Outcome::Handled;
    }
    dismiss_active_report(view);
    Outcome::Handled
}

/// Space and i, judged the same way Esc's swap step is judged (TUI-NAV-030):
/// by the layout the previous frame actually drew, dead-band already folded
/// in, rather than by the terminal's raw width — so a resize sitting inside
/// the breakpoint's dead-band keeps whichever layout is already on screen.
/// Only while the narrow layout is on screen does the press set or clear the
/// inspector call; a swap is what answers it, and only the narrow layout
/// draws one. While the persistent pane is drawn there is nothing for the
/// press to bring in or take out, so the call is left exactly as it stood and
/// the press answers with one footer note instead; while the too-small card
/// is drawn there is nothing on screen to answer at all, so the call is left
/// as it stood and nothing else in the view changes — no note, either.
/// @spec TUI-NAV-088
pub fn toggle_inspector(
    view: &mut ViewState,
    narrow_on_screen: bool,
    pane_on_screen: bool,
) -> Outcome {
    if narrow_on_screen {
        view.inspector_wanted = !view.inspector_wanted;
    } else if pane_on_screen {
        set_footer(view, "the inspector already stands beside the list");
    }
    Outcome::Handled
}
