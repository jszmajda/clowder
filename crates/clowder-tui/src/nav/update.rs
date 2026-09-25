//! The reducer: an action plus current state becomes a view change, a core
//! command, or both.
//!
//! Key resolution is deliberately state-light — it answers "what did the
//! player ask for", not "what happens". This is where the two meet, because
//! the answer depends on the focused row, and rows come from game state.
//! @spec TUI-NAV-002, TUI-NAV-011, TUI-NAV-015, TUI-NAV-037, TUI-NAV-038,
//! TUI-NAV-041, TUI-NAV-051, TUI-NAV-058, TUI-NAV-060, TUI-NAV-061,
//! TUI-NAV-064, TUI-NAV-065

use std::time::{Duration, Instant};

use clowder_core::{Command, GameState, LockReason};

use super::action::{Action, CountSemantics};
use super::palette;
use super::rows::{revealed_sections, rows, Row, RowKey, RowKind};
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

    match action {
        Action::CursorUp
        | Action::CursorDown
        | Action::SectionNext
        | Action::SectionPrev
        | Action::SectionJump(_)
        | Action::GroupNext
        | Action::GroupPrev => {
            // Any navigation silently disarms whatever was armed, and ends any
            // field-accept guard still standing over the row it left.
            // @spec TUI-NAV-015, TUI-NAV-061
            view.armed = None;
            view.field_guard = false;
            handle_navigation(action, view, state);
            Outcome::Handled
        }
        Action::Activate => handle_activate(view, state),
        Action::CountPrompt => handle_count_prompt(view, state),
        Action::MaxAffordable => handle_max_affordable(view, state),
        Action::Increment => handle_increment_decrement(view, state, true),
        Action::Decrement => handle_increment_decrement(view, state, false),
        Action::Type(c) => handle_type(view, c),
        Action::Erase => handle_erase(view),
        Action::Dismiss => handle_dismiss(view),
        Action::Quit => unreachable!("handled above"),

        // @spec TUI-NAV-003
        Action::ToggleInspector => {
            view.inspector_wanted = !view.inspector_wanted;
            Outcome::Handled
        }
        // @spec TUI-NAV-026, TUI-NAV-025, TUI-NAV-038
        Action::OpenJournal => open_overlay(view, Overlay::Journal { anchor: 0 }),
        Action::OpenHelp => open_overlay(view, Overlay::Help),
        Action::OpenPalette => {
            open_overlay(view, Overlay::Palette { query: String::new(), selected: 0 })
        }
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
// Navigation
// ---------------------------------------------------------------------------

fn handle_navigation(action: Action, view: &mut ViewState, state: &GameState) {
    match action {
        // @spec TUI-NAV-005: the digit of a section with nothing in it yet is
        // a soft no-op, not a doorway to an empty room.
        Action::SectionJump(section) => {
            if revealed_sections(state).contains(&section) {
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
fn move_cursor(view: &mut ViewState, state: &GameState, delta: isize) {
    let Some(section) = view.section else { return };
    let list = rows(state, section);
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
    let ring = revealed_sections(state);
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
    let list = rows(state, section);
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
        .filter_map(|(i, _)| list.iter().enumerate().skip(i + 1).find(|(_, r)| r.selectable()).map(|(j, _)| j))
        .collect();
    if heads.is_empty() {
        return;
    }
    let target = if delta > 0 {
        heads.iter().copied().find(|&h| h > here).or_else(|| heads.first().copied())
    } else {
        heads.iter().copied().rev().find(|&h| h < here).or_else(|| heads.last().copied())
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
            set_footer(view, if up { "already at the fastest" } else { "already at the slowest" });
            Outcome::Handled
        }
    }
}

/// Enter `section`, restoring the remembered item by identity, or landing on
/// the first actionable row, or the first row at all. This is navigation's one
/// shared doorway — every path that changes `view.section` runs through it —
/// so it is also where a visit marks the section seen and dismisses whatever
/// report banner is currently showing, the same for a section cycle, a digit
/// jump, or a palette landing. @spec TUI-NAV-008, TUI-NAV-032
fn enter_section(view: &mut ViewState, state: &GameState, section: Section) {
    view.section = Some(section);
    let list = rows(state, section);
    let idx = section.index();

    // A visit marks the section seen and clears its new-content watermark,
    // and dismisses the one report banner currently showing — the first
    // still-undismissed entry in the newest-first queue; the rest stays
    // pending for its own turn.
    view.seen[idx] = SeenMark { visited: true, revealed_at_last_visit: list.len() };
    if let Some(r) = view.reports.iter_mut().find(|r| !r.dismissed) {
        r.dismissed = true;
    }

    let remembered_still_present = view.focus[idx]
        .as_ref()
        .is_some_and(|k| list.iter().any(|r| r.selectable() && &r.key == k));

    if remembered_still_present {
        return;
    }

    let landing = list
        .iter()
        .find(|r| r.selectable() && is_available(state, r))
        .or_else(|| list.iter().find(|r| r.selectable()))
        .map(|r| r.key.clone());

    view.focus[idx] = landing;
}

fn is_available(state: &GameState, row: &Row) -> bool {
    match &row.key {
        RowKey::Content(id, "") => state.availability(*id) == clowder_core::Availability::Available,
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
    rows(state, section).into_iter().find(|r| r.key == key)
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
        let distinct =
            view.last_repeat.is_none_or(|last| now.duration_since(last) >= REPEAT_GATE);
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
        return Outcome::Handled;
    };

    if owned_one_shot(state, &row) {
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

/// Fire an ordinary row's command once, rate-limited to ~10/s so a held key
/// cannot outrun the shell's own pace. The cursor never moves on action.
/// @spec TUI-NAV-010
fn fire_once(view: &mut ViewState, command: Command) -> Outcome {
    let now = Instant::now();
    if let Some(last) = view.last_repeat {
        if now.duration_since(last) < REPEAT_GATE {
            return Outcome::Handled;
        }
    }
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
    view.armed = Some(ArmedRow { key: key.clone(), command, expires_at: now + ARM_DURATION });
    view.last_repeat = Some(now);
    Outcome::Handled
}

/// The count prompt's Enter: sets the count, then either fires/repeats
/// (ordinary row) or arms (heavy row, i.e. DeepNap). An empty or zero count
/// cancels with no effect. The prompt belongs to the row it was opened on, by
/// that row's identity, never to whatever the cursor reads by the time Enter
/// arrives; if that row has left its list since — the sim keeps ticking
/// under an open prompt — the prompt closes with no effect and says so.
/// @spec TUI-NAV-011, TUI-NAV-031, TUI-NAV-036, TUI-NAV-072
fn accept_count_prompt(view: &mut ViewState, state: &GameState) -> Outcome {
    let prompt = view.prompt.take().expect("checked Some above");
    let n: u32 = prompt.digits.parse().unwrap_or(0);

    let Some(row) = prompt
        .row
        .as_ref()
        .and_then(|key| by_key(view, state, key))
    else {
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

    view.overlay = None;
    view.armed = None;
    // The landing is navigation like any other, so it runs through the same
    // shared doorway that marks the section seen and dismisses the showing
    // report banner. @spec TUI-NAV-065
    enter_section(view, state, section);
    view.focus[section.index()] = Some(key);

    guard_after_field(view);
    Outcome::Handled
}

/// Guard the row a field just acted on: the continuation of the very Enter
/// that accepted the field must not also act on the row it returns to.
/// @spec TUI-NAV-061
fn guard_after_field(view: &mut ViewState) {
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

fn handle_count_prompt(view: &mut ViewState, state: &GameState) -> Outcome {
    let Some(row) = current_row(view, state) else {
        return Outcome::Handled;
    };

    if owned_one_shot(state, &row) {
        set_footer(view, "You already have that.");
        return Outcome::Handled;
    }

    let countable = match &row.kind {
        RowKind::Act { semantics, .. } => {
            matches!(semantics, CountSemantics::CountTaking | CountSemantics::RepeatN)
        }
        RowKind::Count { .. } => true,
        RowKind::Display | RowKind::Group => false,
    };

    if !countable {
        set_footer(view, "There's no count to set here.");
        return Outcome::Handled;
    }

    view.prompt = Some(CountPrompt { digits: String::new(), row: Some(row.key.clone()) });
    Outcome::Handled
}

// ---------------------------------------------------------------------------
// X — max affordable — TUI-NAV-012, TUI-NAV-041, TUI-NAV-051
// ---------------------------------------------------------------------------

/// Act to the maximum affordable. The entry point for the behaviour, which is
/// why the annotation is here rather than only on the action variant and the
/// shell-side cap that this reads.
/// @spec TUI-NAV-012
fn handle_max_affordable(view: &mut ViewState, state: &GameState) -> Outcome {
    let Some(row) = current_row(view, state) else {
        return Outcome::Handled;
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
                Outcome::Repeat(*command, MAX_AFFORDABLE_CAP)
            }
            CountSemantics::Single => {
                set_footer(view, "There's no count to set here.");
                Outcome::Handled
            }
        },
        RowKind::Count { increment, .. } => Outcome::Repeat(*increment, MAX_AFFORDABLE_CAP),
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
        return Outcome::Handled;
    };
    match &row.kind {
        RowKind::Count { increment, decrement } => {
            fire_once(view, if up { *increment } else { *decrement })
        }
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
                    set_footer(view, format!("that count goes no higher than {MAX_AFFORDABLE_CAP}"));
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
// Esc — TUI-NAV-030
// ---------------------------------------------------------------------------

/// Closes only the topmost transient: a field (the count prompt, or the
/// palette — a field under this rule, not a full-screen overlay) first, then
/// any other overlay, then an armed row, then nothing further — so Esc over a
/// field open above an armed row closes only the field, leaving the row
/// beneath it exactly as armed. @spec TUI-NAV-030, TUI-NAV-037
fn handle_dismiss(view: &mut ViewState) -> Outcome {
    if view.prompt.take().is_some() {
        return Outcome::Handled;
    }
    if view.overlay.take().is_some() {
        return Outcome::Handled;
    }
    if view.armed.take().is_some() {
        return Outcome::Handled;
    }
    Outcome::Handled
}
