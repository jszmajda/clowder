//! The key map — `nav::keys::resolve` and the admissible set behind it.
//!
//! Pure logic: a key plus the shell's own view state goes in, an `Action` comes
//! out. Resolution deliberately reads no game state — it answers "what did the
//! player ask for", not "what happens" — which is exactly why these tests can
//! name the one right answer for every key in every context the view can be in.
//! The half of a spec that needs game state (whether that section is revealed,
//! whether a sky window is open, whether this row is heavy) is `update`'s to
//! answer and is not asserted here.
//!
//! Three properties carry most of the weight.
//!
//! * **The admissible set is closed** (TUI-NAV-001). tmux owns the keyboard
//!   this game is played on, so the negative direction — *nothing* outside the
//!   safe set is ever bound — matters as much as the positive one, and is
//!   tested by sweeping a corpus of hostile keys through every context rather
//!   than by naming three examples.
//! * **Context suspends bindings, and the two kinds of context suspend
//!   different things** (TUI-NAV-037, TUI-NAV-052). A field captures every
//!   printable, which is the entire reason a typed digit does not jump
//!   sections; a full-screen overlay instead keeps the speed keys, `q`, Ctrl-C
//!   and Esc live, so the player can always retime, leave, or close from
//!   anywhere. The palette is both at once, and the field wins.
//! * **Silence and a whisper are different answers** (TUI-NAV-042). `Ignored`
//!   carries no message; `NotNow` carries words. Getting the two backwards
//!   makes the game either noisy or mute.
//!
//! Every assertion is written against the design's stated *reason*, so a
//! failure message says what the player would suffer, not merely which variant
//! differed.

use std::time::{Duration, Instant};

use clowder_core::Command;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use clowder_tui::nav::keys::{is_tmux_safe, resolve};
use clowder_tui::nav::{
    Action, ArmedRow, CountPrompt, Overlay, PendingReport, RowKey, Section, ViewState,
};

// ---------------------------------------------------------------------------
// Keys
// ---------------------------------------------------------------------------

fn k(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ch(c: char) -> KeyEvent {
    k(KeyCode::Char(c))
}

/// The same character as the terminal reports it when Shift produced it. A
/// terminal may or may not set SHIFT alongside a shifted printable, so both
/// spellings of `X`, `<`, `>`, `?` and `+` reach the shell in the wild.
fn shifted(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::SHIFT)
}

fn with(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, mods)
}

/// The one chord in the game. @spec TUI-NAV-001
fn ctrl_c() -> KeyEvent {
    with(KeyCode::Char('c'), KeyModifiers::CONTROL)
}

/// Every printable ASCII character, which is what "plain printable characters"
/// means for a key map that must survive tmux.
fn printables() -> impl Iterator<Item = char> {
    ' '..='~'
}

/// Keys tmux may swallow, rewrite, or that no terminal agrees on: every Ctrl
/// chord but Ctrl-C, every Alt/Meta/Super combination, the function keys, and
/// the navigation cluster the design's enumeration deliberately leaves out.
fn outside_the_safe_set() -> Vec<KeyEvent> {
    let mut out = Vec::new();

    for c in ['a', 'b', 'd', 'e', 'n', 'p', 'q', 'x', 'z', '1', '0', '/'] {
        out.push(with(KeyCode::Char(c), KeyModifiers::CONTROL));
        out.push(with(KeyCode::Char(c), KeyModifiers::ALT));
        out.push(with(KeyCode::Char(c), KeyModifiers::SUPER));
        out.push(with(KeyCode::Char(c), KeyModifiers::META));
        out.push(with(
            KeyCode::Char(c),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        ));
        out.push(with(KeyCode::Char(c), KeyModifiers::ALT | KeyModifiers::SHIFT));
    }
    for code in [
        KeyCode::Enter,
        KeyCode::Tab,
        KeyCode::Esc,
        KeyCode::Up,
        KeyCode::Down,
        KeyCode::Backspace,
    ] {
        out.push(with(code, KeyModifiers::CONTROL));
        out.push(with(code, KeyModifiers::ALT));
    }
    for n in 1..=12u8 {
        out.push(k(KeyCode::F(n)));
    }
    for code in [
        KeyCode::Home,
        KeyCode::End,
        KeyCode::PageUp,
        KeyCode::PageDown,
        KeyCode::Insert,
        KeyCode::Delete,
    ] {
        out.push(k(code));
    }
    out
}

/// Everything above plus the whole admissible set — the corpus the "nothing
/// outside the set is bound" sweep quantifies over.
fn key_corpus() -> Vec<KeyEvent> {
    let mut out = outside_the_safe_set();
    for c in printables() {
        out.push(ch(c));
        out.push(shifted(c));
    }
    for code in [
        KeyCode::Tab,
        KeyCode::BackTab,
        KeyCode::Enter,
        KeyCode::Esc,
        KeyCode::Backspace,
        KeyCode::Up,
        KeyCode::Down,
        KeyCode::Left,
        KeyCode::Right,
    ] {
        out.push(k(code));
    }
    out.push(with(KeyCode::BackTab, KeyModifiers::SHIFT));
    out.push(ctrl_c());
    out
}

// ---------------------------------------------------------------------------
// Contexts
// ---------------------------------------------------------------------------

/// A screen focused with nothing transient open — the context the global key
/// table speaks for.
fn screen(section: Section) -> ViewState {
    ViewState {
        section: Some(section),
        ..ViewState::default()
    }
}

fn bare() -> ViewState {
    screen(Section::Meadow)
}

/// A count prompt open: a field, capturing printables. @spec TUI-NAV-037
fn counting() -> ViewState {
    ViewState {
        prompt: Some(CountPrompt {
            digits: "1".to_string(),
        }),
        ..bare()
    }
}

fn overlay(o: Overlay) -> ViewState {
    ViewState {
        overlay: Some(o),
        ..bare()
    }
}

fn journal() -> ViewState {
    overlay(Overlay::Journal { anchor: 0 })
}

fn help() -> ViewState {
    overlay(Overlay::Help)
}

/// The palette is a full-screen overlay *and* a field. Where the two disagree,
/// the field wins — that is what stops a typed `q` quitting mid-search.
/// @spec TUI-NAV-037, TUI-NAV-052
fn palette() -> ViewState {
    overlay(Overlay::Palette {
        query: "ca".to_string(),
        selected: 0,
    })
}

/// A heavy row awaiting its second Enter — one of the transients Esc closes.
/// @spec TUI-NAV-030
fn armed() -> ViewState {
    ViewState {
        armed: Some(ArmedRow {
            key: RowKey::Fixed("GreatMeadowMove"),
            command: Command::GreatMeadowMove,
            expires_at: Instant::now() + Duration::from_secs(8),
        }),
        ..bare()
    }
}

/// An unread offline report's banner is a transient too. @spec TUI-NAV-030
fn banner() -> ViewState {
    ViewState {
        reports: vec![PendingReport {
            entry: 0,
            dismissed: false,
        }],
        ..bare()
    }
}

fn every_context() -> Vec<(&'static str, ViewState)> {
    vec![
        ("a bare screen", bare()),
        ("the Legacy screen", screen(Section::Legacy)),
        ("a count prompt open", counting()),
        ("the Journal overlay open", journal()),
        ("the help overlay open", help()),
        ("the palette open", palette()),
        ("a heavy row armed", armed()),
        ("an unread report banner", banner()),
    ]
}

// ---------------------------------------------------------------------------
// The global key table — the design's map, as data
// ---------------------------------------------------------------------------

/// Every global binding, with the sentence from the design that justifies it.
/// This is the whole map: a key absent from this table has no global meaning.
fn global_bindings() -> Vec<(KeyEvent, Action, &'static str)> {
    let mut out: Vec<(KeyEvent, Action, &'static str)> = vec![
        (k(KeyCode::Up), Action::CursorUp, "↑ moves the list cursor"),
        (ch('k'), Action::CursorUp, "k is the cursor-up synonym (full hjkl)"),
        (k(KeyCode::Down), Action::CursorDown, "↓ moves the list cursor"),
        (ch('j'), Action::CursorDown, "j is the cursor-down synonym (full hjkl)"),
        (k(KeyCode::Tab), Action::SectionNext, "Tab moves to the next section"),
        (k(KeyCode::BackTab), Action::SectionPrev, "BackTab moves to the previous section"),
        (ch('l'), Action::SectionNext, "l is the next-section synonym"),
        (ch('h'), Action::SectionPrev, "h is the previous-section synonym"),
        (k(KeyCode::Right), Action::SectionNext, "→ moves to the next section"),
        (k(KeyCode::Left), Action::SectionPrev, "← moves to the previous section"),
        (ch('['), Action::GroupPrev, "[ jumps to the previous group header"),
        (ch(']'), Action::GroupNext, "] jumps to the next group header"),
        (k(KeyCode::Enter), Action::Activate, "Enter performs the focused row's primary action"),
        (ch(' '), Action::ToggleInspector, "Space opens the inspector on the focused row"),
        (ch('i'), Action::ToggleInspector, "i is the inspector synonym"),
        (ch('x'), Action::CountPrompt, "x opens the count prompt"),
        (ch('X'), Action::MaxAffordable, "X acts to the maximum affordable"),
        (ch('+'), Action::Increment, "+ assigns one on a count row"),
        (ch('='), Action::Increment, "= is the unshifted increment synonym"),
        (ch('-'), Action::Decrement, "- unassigns one on a count row"),
        (k(KeyCode::Backspace), Action::Decrement, "Backspace is the decrement synonym"),
        (ch('/'), Action::OpenPalette, "/ opens the find-anything palette"),
        (ch('e'), Action::OpenJournal, "e opens the Journal"),
        (ch('o'), Action::ObserveSky, "o observes the sky"),
        (ch('<'), Action::SpeedDown, "< steps the speed ladder down"),
        (ch('>'), Action::SpeedUp, "> steps the speed ladder up"),
        (ch('?'), Action::OpenHelp, "? opens the help overlay"),
        (ch('q'), Action::Quit, "q quits immediately; the shell saves first"),
        (ctrl_c(), Action::Quit, "Ctrl-C is the emergency quit"),
    ];
    for s in Section::ALL {
        out.push((
            ch(s.digit()),
            Action::SectionJump(s),
            "a digit jumps to the section it was assigned at design time",
        ));
    }
    out
}

/// The printable half of the map, keyed by character: exactly the single-key
/// bindings a field suspends. @spec TUI-NAV-037
fn printable_globals() -> Vec<(char, Action)> {
    global_bindings()
        .into_iter()
        .filter_map(|(key, action, _)| match (key.code, key.modifiers) {
            (KeyCode::Char(c), KeyModifiers::NONE) => Some((c, action)),
            _ => None,
        })
        .collect()
}

fn global_meaning(c: char) -> Option<Action> {
    printable_globals()
        .into_iter()
        .find(|(bound, _)| *bound == c)
        .map(|(_, a)| a)
}

/// The sections the design gives no letter mnemonics — "others: none — rows +
/// Enter suffice". On these screens the printable map is *complete*: a key is
/// either a global binding or unbound, with nothing in between.
const NO_MNEMONIC_SECTIONS: [Section; 7] = [
    Section::Ideas,
    Section::Crafts,
    Section::Sunbeams,
    Section::Wonders,
    Section::Sky,
    Section::Dreams,
    Section::Legacy,
];

/// The letters the design spends on per-screen mnemonics, on the screens that
/// spend them. Not asserted as bindings here — they are `update`'s to aim at a
/// row — but excluded from the "unbound is silent" sweep so that this file does
/// not forbid them.
const MNEMONIC_LETTERS: [char; 5] = ['g', 'f', 'r', 'p', 'n'];

// ---------------------------------------------------------------------------
// TUI-NAV-001 — the admissible set, and its closure
// ---------------------------------------------------------------------------

/// The whole point of the constraint, stated negatively. Under tmux a Ctrl
/// chord, an Alt combination or a function key may never arrive at all — the
/// multiplexer eats it, rewrites it, or the terminal never sends it. A binding
/// on one of those is a command the player cannot reliably issue, so the map
/// must refuse them everywhere, not merely on a bare screen.
///
/// @spec TUI-NAV-001
#[test]
fn no_chord_alt_or_function_key_is_ever_bound_in_any_context() {
    for (label, view) in every_context() {
        for key in outside_the_safe_set() {
            assert_eq!(
                resolve(key, &view),
                Action::Ignored,
                "with {label}, {key:?} resolves to something; tmux owns that key, so a player \
                 pressing it may see nothing happen — or see it happen in another pane"
            );
        }
    }
}

/// Ctrl-C is the single exception, and it is an exception because a player must
/// be able to leave from anywhere, including mid-type in a field where every
/// other printable binding is suspended.
///
/// @spec TUI-NAV-001, TUI-NAV-052
#[test]
fn ctrl_c_is_the_one_chord_and_it_quits_from_everywhere() {
    for (label, view) in every_context() {
        assert_eq!(
            resolve(ctrl_c(), &view),
            Action::Quit,
            "with {label}, Ctrl-C does not quit; the emergency exit must work from every context"
        );
    }
}

/// The closure property, swept rather than sampled: every key that resolves to
/// *anything at all* is inside the admissible set (or is Ctrl-C). This is the
/// assertion that keeps the map from quietly growing a Home key or an Alt
/// binding later, in a context nobody thought to re-check.
///
/// @spec TUI-NAV-001
#[test]
fn nothing_outside_the_safe_set_resolves_to_anything() {
    for (label, view) in every_context() {
        for key in key_corpus() {
            let action = resolve(key, &view);
            if action == Action::Ignored {
                continue;
            }
            assert!(
                is_tmux_safe(key) || key == ctrl_c(),
                "with {label}, {key:?} resolves to {action:?} but is outside the tmux-safe set; \
                 the set is closed — plain printables, Tab/BackTab, Enter, Esc, Space, Backspace, \
                 the four arrows, and Ctrl-C"
            );
        }
    }
}

/// The set itself, enumerated exactly as the spec enumerates it. It is closed
/// on both sides: a printable character is in it however Shift was involved,
/// and the navigation cluster and function keys are out of it even though tmux
/// would not intercept them — the map's vocabulary is fixed by design, not by
/// what happens to survive the multiplexer.
///
/// @spec TUI-NAV-001
#[test]
fn the_safe_set_is_exactly_the_designs_enumeration() {
    for c in printables() {
        assert!(is_tmux_safe(ch(c)), "the printable character {c:?} is admissible");
        assert!(
            is_tmux_safe(shifted(c)),
            "{c:?} reported with SHIFT is still a plain printable character — Shift is the \
             modifier tier precisely because tmux never intercepts what it emits"
        );
    }
    for code in [
        KeyCode::Tab,
        KeyCode::BackTab,
        KeyCode::Enter,
        KeyCode::Esc,
        KeyCode::Backspace,
        KeyCode::Up,
        KeyCode::Down,
        KeyCode::Left,
        KeyCode::Right,
    ] {
        assert!(is_tmux_safe(k(code)), "{code:?} is named in the admissible set");
    }
    assert!(
        is_tmux_safe(with(KeyCode::BackTab, KeyModifiers::SHIFT)),
        "terminals report Shift-Tab as BackTab carrying SHIFT; the same key with its own \
         modifier attached must not fall out of the set"
    );

    for key in outside_the_safe_set() {
        if key == ctrl_c() {
            continue;
        }
        assert!(
            !is_tmux_safe(key),
            "{key:?} is reported as admissible; nothing outside the enumerated set may be, \
             or the map has room to grow a binding tmux can eat"
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-002 / 003 / 004 / 007 — the map on a focused screen
// ---------------------------------------------------------------------------

/// The whole global table at once, on the context it speaks for. Layer 1 alone
/// plays the game, so every binding in it is load-bearing on minute one.
///
/// @spec TUI-NAV-002, TUI-NAV-003, TUI-NAV-004, TUI-NAV-007
#[test]
fn every_global_binding_resolves_on_a_focused_screen() {
    let view = bare();
    for (key, expected, why) in global_bindings() {
        assert_eq!(
            resolve(key, &view),
            expected,
            "{key:?} should resolve to {expected:?} — {why}"
        );
    }
}

/// The backbone a cold player learns in minute one: the cursor moves, Enter
/// does the thing. `j`/`k` are synonyms, not a separate mode — they must resolve
/// to the identical action, or hjkl becomes a second scheme to keep in step.
///
/// @spec TUI-NAV-002
#[test]
fn the_cursor_moves_on_arrows_and_jk_and_enter_acts() {
    let view = bare();

    assert_eq!(resolve(k(KeyCode::Down), &view), Action::CursorDown);
    assert_eq!(resolve(k(KeyCode::Up), &view), Action::CursorUp);
    assert_eq!(
        resolve(ch('j'), &view),
        resolve(k(KeyCode::Down), &view),
        "j and ↓ must mean the same thing; a synonym that drifts is a second key map"
    );
    assert_eq!(
        resolve(ch('k'), &view),
        resolve(k(KeyCode::Up), &view),
        "k and ↑ must mean the same thing"
    );
    assert_eq!(
        resolve(k(KeyCode::Enter), &view),
        Action::Activate,
        "Enter performs the focused row's primary action — the one verb the backbone has"
    );
}

/// Truth is one keypress away, and the keypress is Space. `i` exists for the
/// hand already on the home row; both are the same toggle.
///
/// @spec TUI-NAV-003
#[test]
fn space_and_i_both_toggle_the_inspector() {
    for section in Section::ALL {
        let view = screen(section);
        assert_eq!(
            resolve(ch(' '), &view),
            Action::ToggleInspector,
            "Space must open the inspector on the {} screen too — truth is never more than one \
             keypress away, on any screen",
            section.title()
        );
        assert_eq!(
            resolve(ch('i'), &view),
            Action::ToggleInspector,
            "i is Space's synonym on the {} screen",
            section.title()
        );
    }
}

/// Uppercase is the modifier tier: `x` and `X` are two different commands, and
/// the difference is the only "modifier" this game has. A map that folded case
/// would give the count prompt and the max-affordable sweep the same key.
///
/// @spec TUI-NAV-004
#[test]
fn uppercase_is_the_modifier_tier_and_case_is_never_folded() {
    let view = bare();

    assert_eq!(resolve(ch('x'), &view), Action::CountPrompt);
    assert_eq!(resolve(ch('X'), &view), Action::MaxAffordable);
    assert_ne!(
        resolve(ch('x'), &view),
        resolve(ch('X'), &view),
        "x and X resolve to the same action; uppercase is the shift tier, and folding case \
         collapses the only modifier the tmux constraint leaves us"
    );
}

/// Shift emits a plain printable character — that is the whole reason uppercase
/// could stand in for a modifier. A terminal may report the shifted character
/// with or without the SHIFT flag attached (kitty protocol and Windows do,
/// most Unix terminals do not), so both spellings must land on one action.
///
/// @spec TUI-NAV-004, TUI-NAV-001
#[test]
fn a_shifted_character_resolves_the_same_with_or_without_the_shift_flag() {
    let view = bare();

    for c in ['X', '<', '>', '?', '+'] {
        let plain = resolve(ch(c), &view);
        assert_ne!(
            plain,
            Action::Ignored,
            "fixture: {c:?} is a bound shifted character, or this proves nothing"
        );
        assert_eq!(
            resolve(shifted(c), &view),
            plain,
            "{c:?} resolves to {plain:?} bare but differently when the terminal attaches SHIFT; \
             the same keystroke must not mean two things depending on the terminal"
        );
    }

    assert_eq!(
        resolve(with(KeyCode::BackTab, KeyModifiers::SHIFT), &view),
        Action::SectionPrev,
        "Shift-Tab arrives as BackTab carrying SHIFT; it must still walk the sections backwards"
    );
}

/// Three axes, three key groups, no overlap: sections move sideways, the cursor
/// moves down a list, and `[`/`]` skip by group header inside that list. Uniform
/// spatial movement is what lets a player navigate without thinking, so each
/// axis keeps every one of its synonyms.
///
/// @spec TUI-NAV-007
#[test]
fn sections_cursor_and_groups_are_three_distinct_axes() {
    let view = bare();

    for key in [k(KeyCode::Tab), ch('l'), k(KeyCode::Right)] {
        assert_eq!(
            resolve(key, &view),
            Action::SectionNext,
            "{key:?} moves to the next section"
        );
    }
    for key in [k(KeyCode::BackTab), ch('h'), k(KeyCode::Left)] {
        assert_eq!(
            resolve(key, &view),
            Action::SectionPrev,
            "{key:?} moves to the previous section"
        );
    }
    for key in [k(KeyCode::Down), ch('j')] {
        assert_eq!(resolve(key, &view), Action::CursorDown, "{key:?} moves within the list");
    }
    for key in [k(KeyCode::Up), ch('k')] {
        assert_eq!(resolve(key, &view), Action::CursorUp, "{key:?} moves within the list");
    }
    assert_eq!(
        resolve(ch('['), &view),
        Action::GroupPrev,
        "[ jumps to the previous group header — a long list needs a coarse axis too"
    );
    assert_eq!(resolve(ch(']'), &view), Action::GroupNext, "] jumps to the next group header");
}

// ---------------------------------------------------------------------------
// TUI-NAV-005 / 006 / 033 — sections and their digits
// ---------------------------------------------------------------------------

/// The static map, in the direction the player uses it. Digits are assigned at
/// design time and never renumbered: this is the assertion that a key the
/// player already knows never changes meaning.
///
/// @spec TUI-NAV-005
#[test]
fn every_digit_jumps_to_the_section_it_was_assigned() {
    let view = bare();

    for section in Section::ALL {
        assert_eq!(
            resolve(ch(section.digit()), &view),
            Action::SectionJump(section),
            "the digit {:?} must address the {} section",
            section.digit(),
            section.title()
        );
    }

    // The map is total and one-to-one over the ten digits: every digit the
    // player can press addresses a section, and no two address the same one.
    let mut jumped: Vec<Section> = Vec::new();
    for c in ['1', '2', '3', '4', '5', '6', '7', '8', '9', '0'] {
        match resolve(ch(c), &view) {
            Action::SectionJump(s) => {
                assert!(
                    !jumped.contains(&s),
                    "two digits address the {} section; ten digits for ten sections is the \
                     whole reason the map can be static",
                    s.title()
                );
                jumped.push(s);
            }
            other => panic!("the digit {c:?} resolves to {other:?} rather than a section jump"),
        }
    }
    assert_eq!(jumped.len(), Section::ALL.len(), "all ten sections are addressable by digit");
}

/// A section that unlocks late slots into its reserved digit, so the key map a
/// player learned on day one still holds on day thirty. At this layer that is a
/// statement about *independence*: the digit resolves to the same section
/// whatever the view is doing — which section is focused, what has been seen,
/// what is armed. Whether that section is revealed yet is game state, so the
/// footer note for an unrevealed digit is `update`'s answer, not this layer's.
///
/// @spec TUI-NAV-005, TUI-NAV-006
#[test]
fn a_late_unlocking_sections_digit_never_changes_meaning() {
    let mut visited = bare();
    visited.seen[Section::Legacy.index()].visited = true;
    visited.seen[Section::Legacy.index()].revealed_at_last_visit = 4;

    let contexts = [bare(), screen(Section::Legacy), screen(Section::Sky), visited, armed(), banner()];

    for section in Section::ALL {
        for view in &contexts {
            assert_eq!(
                resolve(ch(section.digit()), view),
                Action::SectionJump(section),
                "the digit {:?} stops addressing the {} section once the view changes; digits are \
                 assigned at design time and must never renumber under the player",
                section.digit(),
                section.title()
            );
        }
    }
}

/// The ring has no dead end. Cycling keys resolve to the same step at both ends
/// of the revealed ring — the last section's Tab is a step forward, not a
/// silence — because the wrap belongs to the reducer walking the revealed ring,
/// not to a key that refuses to fire.
///
/// @spec TUI-NAV-033, TUI-NAV-007
#[test]
fn the_cycling_keys_stay_live_at_both_ends_of_the_ring() {
    for section in Section::ALL {
        let view = screen(section);
        for key in [k(KeyCode::Tab), ch('l'), k(KeyCode::Right)] {
            assert_eq!(
                resolve(key, &view),
                Action::SectionNext,
                "on the {} screen, {key:?} must still step forward; cycling wraps around the \
                 revealed ring, so no screen is the end of it",
                section.title()
            );
        }
        for key in [k(KeyCode::BackTab), ch('h'), k(KeyCode::Left)] {
            assert_eq!(
                resolve(key, &view),
                Action::SectionPrev,
                "on the {} screen, {key:?} must still step backwards; the ring wraps the other \
                 way too",
                section.title()
            );
        }
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-028 / 040 — the two global commands that belong to no section
// ---------------------------------------------------------------------------

/// Speed is reachable from anywhere, always — it is how a player who must leave
/// slows the game down, and it is never behind a settings row. One notch per
/// press in each direction; where that lands on the ladder is the reducer's.
///
/// @spec TUI-NAV-028
#[test]
fn the_speed_keys_step_the_ladder_from_every_screen() {
    for section in Section::ALL {
        let view = screen(section);
        assert_eq!(
            resolve(ch('<'), &view),
            Action::SpeedDown,
            "< must step the speed down from the {} screen",
            section.title()
        );
        assert_eq!(
            resolve(ch('>'), &view),
            Action::SpeedUp,
            "> must step the speed up from the {} screen",
            section.title()
        );
    }

    // Both characters are Shift-produced on a US layout, so both spellings must
    // land. @spec TUI-NAV-004
    assert_eq!(resolve(shifted('<'), &bare()), Action::SpeedDown);
    assert_eq!(resolve(shifted('>'), &bare()), Action::SpeedUp);
}

/// Sky-watching belongs to no section, so `o` is the only way to reach it and
/// it must answer from every screen. Whether a window is open is game state:
/// this layer always resolves the ask, and the footer whisper for "not right
/// now" is the reducer's to add.
///
/// @spec TUI-NAV-040
#[test]
fn observing_the_sky_answers_from_every_screen() {
    for section in Section::ALL {
        assert_eq!(
            resolve(ch('o'), &screen(section)),
            Action::ObserveSky,
            "o must reach the sky from the {} screen; it is a global command with no row of its \
             own, so a screen where it does not answer is a command the player cannot issue",
            section.title()
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-042 / 046 — silence, whispers, and the reserved tier
// ---------------------------------------------------------------------------

/// The complete printable map, on the screens the design gives no mnemonics:
/// every printable character is either a global binding or silent, and there is
/// nothing in between. A cat walking across the keyboard must produce no
/// footer noise at all — quiet is the whole point — while the keys that *are*
/// bound all still answer.
///
/// @spec TUI-NAV-042, TUI-NAV-046
#[test]
fn on_a_screen_with_no_mnemonics_a_printable_key_is_a_global_binding_or_silence() {
    for section in NO_MNEMONIC_SECTIONS {
        let view = screen(section);
        for c in printables() {
            let got = resolve(ch(c), &view);
            match global_meaning(c) {
                Some(expected) => assert_eq!(
                    got,
                    expected,
                    "on the {} screen, {c:?} resolves to {got:?} rather than its global meaning",
                    section.title()
                ),
                None => assert_eq!(
                    got,
                    Action::Ignored,
                    "on the {} screen, {c:?} has no binding yet resolves to {got:?}; an unbound \
                     printable is ignored in silence, with no footer note — the design spends a \
                     whisper only on a key that is bound but not meaningful right now",
                    section.title()
                ),
            }
        }
    }
}

/// The same, restricted to letters the design never spends, on every screen
/// including the three that do carry mnemonics. These are silent everywhere.
///
/// @spec TUI-NAV-042
#[test]
fn an_unbound_printable_is_ignored_with_no_message_on_every_screen() {
    let never_bound: Vec<char> = printables()
        .filter(|c| global_meaning(*c).is_none() && !MNEMONIC_LETTERS.contains(c))
        .collect();
    assert!(
        never_bound.len() > 20,
        "fixture: most of the printable range is unbound, or this sweep is trivial"
    );

    for section in Section::ALL {
        let view = screen(section);
        for c in &never_bound {
            assert_eq!(
                resolve(ch(*c), &view),
                Action::Ignored,
                "on the {} screen, the unbound key {c:?} says something; a key with no binding \
                 in this context is ignored with no footer message",
                section.title()
            );
        }
    }
}

/// The other half of the distinction: a whisper is a whisper. Wherever the map
/// answers "bound, but not meaningful right now", it must carry words for the
/// footer to say — an empty `NotNow` is the worst of both, a message slot with
/// nothing in it.
///
/// @spec TUI-NAV-042
#[test]
fn a_not_now_answer_always_carries_words_and_ignored_never_does() {
    for (label, view) in every_context() {
        for key in key_corpus() {
            if let Action::NotNow(message) = resolve(key, &view) {
                assert!(
                    !message.trim().is_empty(),
                    "with {label}, {key:?} answers NotNow with an empty message; the whisper \
                     exists to say why the key did nothing"
                );
            }
        }
    }
}

/// The reserved tier, enforced from the other end. A mnemonic may only ever be
/// spent on a letter the global map has not claimed, so every globally bound
/// key means the same thing on every screen. If a screen ever took `x` or `e`
/// for a local verb, the player's hands would have to learn where they are
/// standing before pressing anything.
///
/// @spec TUI-NAV-046, TUI-NAV-001
#[test]
fn no_screen_may_take_a_reserved_key_for_a_mnemonic() {
    let reference = bare();
    for section in Section::ALL {
        let view = screen(section);
        for (key, expected, why) in global_bindings() {
            assert_eq!(
                resolve(key, &view),
                expected,
                "on the {} screen, {key:?} no longer means what it means everywhere else — {why}",
                section.title()
            );
        }
        // And nothing at all differs between screens: the global map is a
        // property of the key, not of where the player is standing.
        for c in printables() {
            let here = resolve(ch(c), &view);
            if let Some(expected) = global_meaning(c) {
                assert_eq!(
                    here,
                    expected,
                    "on the {} screen, the reserved key {c:?} resolves to {here:?}",
                    section.title()
                );
                assert_eq!(here, resolve(ch(c), &reference));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-037 — a field captures the keyboard
// ---------------------------------------------------------------------------

/// The spec's own worked example, and the reason the whole rule exists: a digit
/// typed into a count prompt must be a digit, not a jump to section three. Get
/// this wrong and typing "3" while buying three of something walks the player
/// out of the screen they were buying on.
///
/// @spec TUI-NAV-037
#[test]
fn a_digit_typed_into_a_field_types_instead_of_jumping() {
    for (label, view) in [("a count prompt", counting()), ("the palette", palette())] {
        for c in ['1', '2', '3', '4', '5', '6', '7', '8', '9', '0'] {
            let got = resolve(ch(c), &view);
            assert_eq!(
                got,
                Action::Type(c),
                "with {label} open, {c:?} resolves to {got:?}; digits must reach the field, or \
                 typing a count walks the player out of the screen they were counting on"
            );
        }
    }
}

/// Every global single-key binding is suspended, not merely the digits. `q` is
/// the sharpest case: a player typing a search for something with a `q` in it
/// must not have the game quit out from under them.
///
/// @spec TUI-NAV-037
#[test]
fn a_field_suspends_every_global_single_key_binding() {
    for (label, view) in [("a count prompt", counting()), ("the palette", palette())] {
        for (c, global) in printable_globals() {
            let got = resolve(ch(c), &view);
            assert_ne!(
                got,
                global,
                "with {label} open, {c:?} still resolves to its global meaning {global:?}; every \
                 global single-key binding is suspended while a field has the keyboard"
            );
        }
    }
}

/// The palette narrows by substring as characters arrive, so in the palette the
/// suspension is not merely a silence — the letters have to land in the query.
///
/// @spec TUI-NAV-037
#[test]
fn printable_characters_reach_the_palette_field() {
    let view = palette();
    for c in ['a', 'q', 'x', 'e', 'o', 'z', 'X', '<', '>', '/', '?', '-'] {
        let got = resolve(ch(c), &view);
        assert_eq!(
            got,
            Action::Type(c),
            "with the palette open, {c:?} resolves to {got:?}; the palette narrows by substring \
             as characters are typed, so every printable belongs to the query"
        );
    }
}

/// A field's three structural keys. Backspace is the interesting one: its
/// global meaning is "unassign one", and inside a field it must edit instead —
/// a decrement fired by a typo would be a silent, invisible change to state
/// the player thought they were only typing into.
///
/// @spec TUI-NAV-037, TUI-NAV-030
#[test]
fn enter_accepts_backspace_edits_and_esc_cancels_a_field() {
    for (label, view) in [("a count prompt", counting()), ("the palette", palette())] {
        assert_eq!(
            resolve(k(KeyCode::Enter), &view),
            Action::Activate,
            "with {label} open, Enter must accept the field"
        );
        assert_eq!(
            resolve(k(KeyCode::Backspace), &view),
            Action::Erase,
            "with {label} open, Backspace must edit the field rather than fire its global \
             decrement"
        );
        assert_eq!(
            resolve(k(KeyCode::Esc), &view),
            Action::Dismiss,
            "with {label} open, Esc must cancel the field — nothing happened"
        );
    }
}

/// The keys that move the cursor or the section mean nothing inside a field —
/// not motion, and not a footer whisper either: the field holds the keyboard,
/// so the row beneath it cannot be walked out from under the player's typing.
///
/// @spec TUI-NAV-059
#[test]
fn arrows_tab_and_backtab_resolve_to_nothing_inside_a_field() {
    for (label, view) in [("a count prompt", counting()), ("the palette", palette())] {
        for code in [
            KeyCode::Up,
            KeyCode::Down,
            KeyCode::Left,
            KeyCode::Right,
            KeyCode::Tab,
            KeyCode::BackTab,
        ] {
            let got = resolve(k(code), &view);
            assert_eq!(
                got,
                Action::Ignored,
                "with {label} open, {code:?} resolves to {got:?}; the arrows, Tab and BackTab do \
                 nothing while a field holds the keyboard"
            );
        }
        let got = resolve(with(KeyCode::BackTab, KeyModifiers::SHIFT), &view);
        assert_eq!(got, Action::Ignored, "with {label} open, Shift-BackTab resolves to {got:?}");
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-052 — a full-screen overlay
// ---------------------------------------------------------------------------

/// The four keys that stay live wherever the player is: retime, leave, leave in
/// a hurry, close. All four are safe wherever they land, which is exactly why
/// they are the ones that survive an overlay.
///
/// @spec TUI-NAV-052
#[test]
fn an_overlay_keeps_the_speed_keys_quit_and_esc_live() {
    for (label, view) in [("the Journal", journal()), ("the help overlay", help())] {
        assert_eq!(
            resolve(ch('<'), &view),
            Action::SpeedDown,
            "with {label} open, < must still slow the game — the player can always retime"
        );
        assert_eq!(
            resolve(ch('>'), &view),
            Action::SpeedUp,
            "with {label} open, > must still speed the game up"
        );
        assert_eq!(
            resolve(ch('q'), &view),
            Action::Quit,
            "with {label} open, q must still leave — quitting is always safe, from anywhere"
        );
        assert_eq!(
            resolve(ctrl_c(), &view),
            Action::Quit,
            "with {label} open, Ctrl-C must still leave"
        );
        assert_eq!(
            resolve(k(KeyCode::Esc), &view),
            Action::Dismiss,
            "with {label} open, Esc must close it — the way out of an overlay is never blocked"
        );
    }
}

/// The other side of the same bargain: while an overlay is up, the section and
/// action keys stop meaning what they mean on a screen. A digit pressed in the
/// Journal must not tear the Journal away to a section, and `x` must not open a
/// count prompt on a row the player cannot even see.
///
/// @spec TUI-NAV-052
#[test]
fn an_overlay_suspends_the_section_and_action_keys() {
    for (label, view) in [("the Journal", journal()), ("the help overlay", help())] {
        for section in Section::ALL {
            let got = resolve(ch(section.digit()), &view);
            assert_ne!(
                got,
                Action::SectionJump(section),
                "with {label} open, the digit {:?} still jumps to a section; the overlay holds \
                 the section keys while it is up",
                section.digit()
            );
        }
        for key in [k(KeyCode::Tab), ch('l'), k(KeyCode::Right)] {
            assert_ne!(
                resolve(key, &view),
                Action::SectionNext,
                "with {label} open, {key:?} still walks the sections"
            );
        }
        for key in [k(KeyCode::BackTab), ch('h'), k(KeyCode::Left)] {
            assert_ne!(
                resolve(key, &view),
                Action::SectionPrev,
                "with {label} open, {key:?} still walks the sections"
            );
        }
        assert_ne!(
            resolve(ch('x'), &view),
            Action::CountPrompt,
            "with {label} open, x still opens a count prompt on a row the player cannot see"
        );
        assert_ne!(
            resolve(ch('X'), &view),
            Action::MaxAffordable,
            "with {label} open, X still acts on the hidden row — to the maximum affordable"
        );
        assert_ne!(
            resolve(ch('+'), &view),
            Action::Increment,
            "with {label} open, + still assigns a worker behind the overlay"
        );
        assert_ne!(
            resolve(ch('-'), &view),
            Action::Decrement,
            "with {label} open, - still unassigns a worker behind the overlay"
        );
    }
}

/// The overlay takes the cursor keys *for its own navigation* — the Journal
/// scrolls with ↑↓/j/k and moves by day with [/] — so those keys keep resolving
/// to cursor motion and the overlay is what the motion lands on. An overlay
/// that resolved them to nothing would be a scrollback the player cannot scroll.
///
/// @spec TUI-NAV-052, TUI-NAV-026
#[test]
fn the_journal_takes_the_cursor_and_group_keys_for_its_own_navigation() {
    let view = journal();

    for key in [k(KeyCode::Down), ch('j')] {
        assert_eq!(
            resolve(key, &view),
            Action::CursorDown,
            "{key:?} must still resolve to downward motion with the Journal open; the Journal \
             scrolls with the cursor keys, so a silence here is a scrollback that cannot scroll"
        );
    }
    for key in [k(KeyCode::Up), ch('k')] {
        assert_eq!(resolve(key, &view), Action::CursorUp, "{key:?} must still move upward");
    }
    assert_eq!(
        resolve(ch('['), &view),
        Action::GroupPrev,
        "[ moves the Journal back by day — the same coarse axis it has in a list"
    );
    assert_eq!(resolve(ch(']'), &view), Action::GroupNext, "] moves the Journal on by day");
}

/// The palette is an overlay *and* a field, and the field wins. The design says
/// so in as many words: the count prompt and the palette differ from the other
/// overlays because they capture all printable input. So the speed keys and `q`
/// — live in the Journal — type in the palette, while Esc and Ctrl-C, which are
/// not printable, still get the player out.
///
/// @spec TUI-NAV-037, TUI-NAV-052
#[test]
fn the_palette_field_captures_even_the_keys_an_overlay_keeps_live() {
    let field = palette();
    let overlay = journal();

    for c in ['<', '>', 'q'] {
        let in_overlay = resolve(ch(c), &overlay);
        assert_ne!(
            in_overlay,
            Action::Ignored,
            "fixture: {c:?} stays live inside a plain overlay, or the contrast below is empty"
        );
        assert_eq!(
            resolve(ch(c), &field),
            Action::Type(c),
            "{c:?} resolves to {in_overlay:?} in the Journal and must type in the palette; a \
             printable key belongs to the field whenever a field is open"
        );
    }

    assert_eq!(
        resolve(k(KeyCode::Esc), &field),
        Action::Dismiss,
        "Esc is not printable, so it still closes the palette"
    );
    assert_eq!(
        resolve(ctrl_c(), &field),
        Action::Quit,
        "Ctrl-C is not printable either; the emergency exit survives even a field"
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-030 — Esc, and the escape-time hazard
// ---------------------------------------------------------------------------

/// Esc closes the topmost transient — and every kind of transient counts, which
/// is what makes the key predictable enough to press without looking.
///
/// @spec TUI-NAV-030
#[test]
fn esc_closes_a_transient_of_every_kind() {
    for (label, view) in [
        ("a count prompt", counting()),
        ("the Journal", journal()),
        ("the help overlay", help()),
        ("the palette", palette()),
        ("an armed heavy row", armed()),
        ("an unread report banner", banner()),
    ] {
        assert_eq!(
            resolve(k(KeyCode::Esc), &view),
            Action::Dismiss,
            "with {label} open, Esc must close it"
        );
    }
}

/// The escape-time hazard, stated as the property that makes it harmless. Under
/// tmux a stray or late Esc can arrive on a bare screen at any moment; there it
/// must do nothing. Nothing means nothing: it may not act, may not quit, may
/// not move, may not open anything. At worst it closes something that was open.
///
/// @spec TUI-NAV-030
#[test]
fn esc_on_a_bare_screen_can_neither_act_nor_quit_nor_move() {
    for section in Section::ALL {
        let view = screen(section);
        let got = resolve(k(KeyCode::Esc), &view);
        assert!(
            matches!(got, Action::Ignored | Action::Dismiss),
            "on the bare {} screen, Esc resolves to {got:?}; with nothing open there is nothing \
             for it to close, and a late Esc arriving under tmux escape-time must never act, \
             quit, or lose anything",
            section.title()
        );
        assert_ne!(got, Action::Quit, "a stray Esc must never quit the game");
    }
}

/// No Esc-prefixed sequence exists anywhere, so escape-time coalescing cannot
/// form a phantom binding: an Esc followed by any key resolves that key exactly
/// as it would have on its own. Resolution holds no state of its own to build a
/// prefix out of — this test states the property the signature currently grants
/// for free, so that a later prefix table cannot be added without breaking it.
///
/// @spec TUI-NAV-030
#[test]
fn an_esc_leaves_no_prefix_behind_for_the_next_key_to_join() {
    for (label, view) in every_context() {
        for key in key_corpus() {
            let alone = resolve(key, &view);
            let _ = resolve(k(KeyCode::Esc), &view);
            let after_an_esc = resolve(key, &view);
            assert_eq!(
                alone, after_an_esc,
                "with {label}, {key:?} means {alone:?} on its own but {after_an_esc:?} after an \
                 Esc; a coalesced Esc under tmux escape-time would then form a binding nobody \
                 designed"
            );
        }
    }
}
