//! The declare list: which adventure is chosen, and what each one is.
//!
//! The core holds two Adventures, either of which may be none — the one armed
//! for the next Move and the one active in this meadow (SIM-ADV-001,
//! SIM-ADV-002). The list says both on the rows they name (TUI-NAV-178), the
//! inspector says both in sentences beneath the name (TUI-NAV-179), and an
//! adventure's inspector states its blurb as its mechanical statement
//! (TUI-NAV-018, SIM-ADV-006).

use clowder_core::content::{adventure_def, blurb_text};
use clowder_core::{AdventureId, GameState, TraditionId};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::Terminal;

use clowder_tui::nav::rows::rows;
use clowder_tui::nav::{RowKey, Section, ViewState};
use clowder_tui::render::layout::Body;

/// The terminal floor's width, and widths either side of the pane breakpoint
/// up to the widest the tests draw at.
const WIDTHS: [u16; 7] = [80, 90, 99, 100, 120, 200, 240];
/// Tall enough that the whole Legacy list is drawn unscrolled.
const TALL: u16 = 60;

/// What the inspector says of each slot.
const UNDER_WAY: &str = "Under way in this meadow.";
const CHOSEN: &str = "Chosen for the next meadow.";

/// A meadow with the Adventures surface open, holding `active` and `armed`:
/// one Move made, as the tradition that opens the surface is bought with what
/// a Move grants, so the Legacy rows' gates are open.
fn meadow(active: Option<AdventureId>, armed: Option<AdventureId>) -> GameState {
    let mut s = GameState::new(178);
    s.generation = 1;
    s.traditions.insert(TraditionId::AdventurousSpirit);
    s.active_adventure = active;
    s.armed_adventure = armed;
    s
}

/// Every pair of slots worth telling apart: both empty, each alone, the two
/// naming different adventures, and the two naming one.
fn slot_pairs() -> Vec<(Option<AdventureId>, Option<AdventureId>)> {
    let mut out = vec![(None, None)];
    for a in AdventureId::ALL {
        out.push((Some(a), None));
        out.push((None, Some(a)));
        out.push((Some(a), Some(a)));
    }
    let all = AdventureId::ALL;
    for (i, a) in all.iter().enumerate() {
        out.push((Some(*a), Some(all[(i + 1) % all.len()])));
    }
    out
}

/// An adventure's name as its row and the palette spell it.
fn name(a: AdventureId) -> String {
    let tag = format!("{a:?}");
    let mut out = String::new();
    for (i, c) in tag.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

/// The words the list should draw after an adventure's name, if any.
fn words(state: &GameState, a: AdventureId) -> Option<&'static str> {
    match (
        state.active_adventure == Some(a),
        state.armed_adventure == Some(a),
    ) {
        (true, true) => Some("this meadow and next"),
        (true, false) => Some("this meadow"),
        (false, true) => Some("next meadow"),
        (false, false) => None,
    }
}

fn looking_at(key: RowKey) -> ViewState {
    let mut v = ViewState {
        section: Some(Section::Legacy),
        inspector_wanted: true,
        ..Default::default()
    };
    v.focus[Section::Legacy.index()] = Some(key);
    v
}

fn region_lines(buf: &Buffer, rect: Rect) -> Vec<String> {
    (rect.y..rect.bottom())
        .map(|y| {
            (rect.x..rect.right())
                .map(|x| buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "))
                .collect::<String>()
        })
        .collect()
}

/// One frame: the buffer, the list's rectangle where the list is drawn, and
/// the inspector's rectangle — the pane, or the narrow-mode swap.
fn frame(state: &GameState, view: &ViewState, w: u16) -> (Buffer, Option<Rect>, Rect) {
    let mut term = Terminal::new(TestBackend::new(w, TALL)).expect("TestBackend is infallible");
    let mut layout = None;
    term.draw(|f| layout = Some(clowder_tui::render::frame(f, state, view, None)))
        .expect("TestBackend is infallible");
    let (list, inspector) = match layout.expect("frame returns its layout").body {
        Body::Wide {
            list, inspector, ..
        } => (
            Some(Rect::new(list.x + 1, list.y, list.width - 2, list.height)),
            inspector,
        ),
        Body::Narrow { main, .. } => (
            None,
            Rect::new(
                main.x + 1,
                main.y,
                main.width.saturating_sub(1),
                main.height,
            ),
        ),
        other => panic!("fixture: {w}x{TALL} lays out a list, not {other:?}"),
    };
    (term.backend().buffer().clone(), list, inspector)
}

/// The list drawn at `w` with the inspector not called for, so the narrow
/// layout draws the list rather than the swap.
fn list_frame(state: &GameState, key: RowKey, w: u16) -> (Buffer, Rect) {
    let view = ViewState {
        inspector_wanted: false,
        ..looking_at(key)
    };
    let mut term = Terminal::new(TestBackend::new(w, TALL)).expect("TestBackend is infallible");
    let mut layout = None;
    term.draw(|f| layout = Some(clowder_tui::render::frame(f, state, &view, None)))
        .expect("TestBackend is infallible");
    let rect = match layout.expect("frame returns its layout").body {
        Body::Wide { list, .. } => Rect::new(list.x + 1, list.y, list.width - 2, list.height),
        Body::Narrow { main, .. } => Rect::new(
            main.x + 1,
            main.y,
            main.width.saturating_sub(1),
            main.height,
        ),
        other => panic!("fixture: {w}x{TALL} lays out a list, not {other:?}"),
    };
    (term.backend().buffer().clone(), rect)
}

/// The line of `lines` drawing the row named `name` — its name standing at
/// the name's column, four cells in, and followed by a blank or nothing.
fn row_line<'a>(lines: &'a [String], name: &str) -> Option<(usize, &'a String)> {
    lines.iter().enumerate().find(|(_, l)| {
        let rest: String = l.chars().skip(4).collect();
        rest.strip_prefix(name)
            .is_some_and(|after| after.is_empty() || after.starts_with(' '))
    })
}

/// While the core holds an adventure active, armed, or both, that adventure's
/// row in the declare list carries `this meadow`, `next meadow`, or `this
/// meadow and next` after its name and one blank column, whole, in the row's
/// own style — the cursor's row reversed with it — and nothing else follows
/// the name. No other adventure's row carries words, and the no-adventure row
/// never does. Quantified over every pair of slots worth telling apart, with
/// the cursor on each marked row and off it, at widths from the floor through
/// the pane breakpoint to the widest.
///
/// @spec TUI-NAV-178
#[test]
fn the_declare_list_says_which_meadow_each_chosen_adventure_is_for() {
    let mut marked = 0u32;
    for (active, armed) in slot_pairs() {
        let state = meadow(active, armed);
        let mut cursors = vec![RowKey::Adventure(None)];
        cursors.extend(
            active
                .into_iter()
                .chain(armed)
                .map(|a| RowKey::Adventure(Some(a))),
        );
        for cursor in cursors {
            for w in WIDTHS {
                let (buf, rect) = list_frame(&state, cursor.clone(), w);
                let lines = region_lines(&buf, rect);
                let whr = format!(
                    "at {w} columns with {active:?} active and {armed:?} armed, the cursor on \
                     {cursor:?}"
                );

                let (_, none_line) = row_line(&lines, "No Adventure").unwrap_or_else(|| {
                    panic!(
                        "fixture: {whr}, the no-adventure row is drawn:\n{}",
                        lines.join("\n")
                    )
                });
                assert_eq!(
                    none_line.chars().skip(4).collect::<String>().trim_end(),
                    "No Adventure",
                    "{whr}: the no-adventure row carries no words (TUI-NAV-178)"
                );

                for a in AdventureId::ALL {
                    let n = name(a);
                    let (y, line) = row_line(&lines, &n).unwrap_or_else(|| {
                        panic!("fixture: {whr}, {n}'s row is drawn:\n{}", lines.join("\n"))
                    });
                    let after_marker: String = line.chars().skip(4).collect();
                    let expected = match words(&state, a) {
                        Some(wd) => format!("{n} {wd}"),
                        None => n.clone(),
                    };
                    assert_eq!(
                        after_marker.trim_end(),
                        expected,
                        "{whr}: {n}'s row reads {line:?} (TUI-NAV-178)"
                    );

                    // The words wear the row's own style, as the name does.
                    if words(&state, a).is_some() {
                        marked += 1;
                        let name_x = rect.x + 4;
                        let words_x = name_x + n.chars().count() as u16 + 1;
                        let at = |x: u16| {
                            buf.cell((x, rect.y + y as u16))
                                .expect("in the buffer")
                                .style()
                        };
                        assert_eq!(
                            at(words_x),
                            at(name_x),
                            "{whr}: {n}'s words are drawn in the row's own style (TUI-NAV-178)"
                        );
                    }
                }
            }
        }
    }
    assert!(marked > 0, "fixture: some row carried words");
}

/// The words are the row's, never the adventure's name: the palette finds and
/// shows an adventure by its name alone, chosen or not.
///
/// @spec TUI-NAV-178
#[test]
fn an_adventures_words_are_no_part_of_its_name() {
    let a = AdventureId::ALL[0];
    let state = meadow(Some(a), Some(a));
    let row = rows(&state, Section::Legacy)
        .into_iter()
        .find(|r| r.key == RowKey::Adventure(Some(a)))
        .expect("fixture: the adventure's row is listed");
    let results = clowder_tui::nav::palette::results(&state, &name(a));
    let found: Vec<String> = results
        .iter()
        .filter(|r| r.key == row.key)
        .map(|r| r.name.clone())
        .collect();
    assert_eq!(
        found,
        vec![name(a)],
        "the palette names the chosen adventure by its name alone (TUI-NAV-178)"
    );
}

/// The inspector's lines on an adventure's row: everything it draws, trimmed,
/// blank lines kept so the gap before the statement can be read.
fn inspector_lines(state: &GameState, a: Option<AdventureId>, w: u16) -> Vec<String> {
    let (buf, _, rect) = frame(state, &looking_at(RowKey::Adventure(a)), w);
    let mut lines: Vec<String> = region_lines(&buf, rect)
        .into_iter()
        .map(|l| l.trim().to_string())
        .collect();
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    while lines.first().is_some_and(|l| l.is_empty()) {
        lines.remove(0);
    }
    lines
}

/// While the cursor is on an adventure's row, the inspector states
/// `Under way in this meadow.` where the core holds it active and then
/// `Chosen for the next meadow.` where it holds it armed, each a line of its
/// own directly beneath the name, at every width — the persistent pane and the
/// narrow-mode swap alike. The no-adventure row, and an adventure's row the
/// core holds in neither slot, state no such line.
///
/// @spec TUI-NAV-179
#[test]
fn the_inspector_says_which_meadow_an_adventure_is_for_beneath_its_name() {
    for (active, armed) in slot_pairs() {
        let state = meadow(active, armed);
        for w in WIDTHS {
            let whr = format!("at {w} columns with {active:?} active and {armed:?} armed");

            let none = inspector_lines(&state, None, w);
            assert_eq!(
                none.first().map(String::as_str),
                Some("No Adventure"),
                "fixture: {whr}, the inspector names the no-adventure row:\n{}",
                none.join("\n")
            );
            assert!(
                !none
                    .iter()
                    .any(|l| l.contains(UNDER_WAY) || l.contains(CHOSEN)),
                "{whr}: the no-adventure row states no slot (TUI-NAV-179):\n{}",
                none.join("\n")
            );

            for a in AdventureId::ALL {
                let lines = inspector_lines(&state, Some(a), w);
                let mut expected = vec![name(a)];
                if active == Some(a) {
                    expected.push(UNDER_WAY.to_string());
                }
                if armed == Some(a) {
                    expected.push(CHOSEN.to_string());
                }
                // The inspector wraps a line its pane cannot hold, so the head
                // is read as the drawn lines before the gap above the
                // statement, rejoined: the name, then each sentence, each
                // beginning a drawn line of its own.
                let gap = lines
                    .iter()
                    .position(|l| l.is_empty())
                    .unwrap_or(lines.len());
                let head = &lines[..gap];
                assert_eq!(
                    head.join(" "),
                    expected.join(" "),
                    "{whr}, on {a:?}'s row: the name, then a line for each slot the core holds it \
                     in (TUI-NAV-179):\n{}",
                    lines.join("\n")
                );
                for sentence in &expected[1..] {
                    let first_word = sentence.split(' ').next().expect("a sentence has a word");
                    assert!(
                        head.iter().any(|l| l.starts_with(first_word)),
                        "{whr}, on {a:?}'s row: {sentence:?} begins a line of its own \
                         (TUI-NAV-179):\n{}",
                        lines.join("\n")
                    );
                }
                let rest = lines[gap..].join(" ");
                assert!(
                    !rest.contains("Under way") && !rest.contains("Chosen for"),
                    "{whr}, on {a:?}'s row: no slot line stands anywhere else (TUI-NAV-179):\n{}",
                    lines.join("\n")
                );
            }
        }
    }
}

/// An adventure's definition carries a blurb, so its row's inspector states
/// the blurb's words as the row's mechanical statement — set apart from the
/// lines above it by a blank line — at every width, whether or not the
/// adventure is chosen. The no-adventure row has no definition, and states no
/// statement and no stand-in for one.
///
/// @spec TUI-NAV-018
#[test]
fn an_adventures_inspector_states_its_blurb() {
    let squash = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
    for (active, armed) in [
        (None, None),
        (Some(AdventureId::ALL[1]), Some(AdventureId::ALL[2])),
    ] {
        let state = meadow(active, armed);
        for w in WIDTHS {
            for a in AdventureId::ALL {
                let lines = inspector_lines(&state, Some(a), w);
                let blurb = squash(&blurb_text(adventure_def(a).blurb));
                let gap = lines.iter().position(|l| l.is_empty()).unwrap_or_else(|| {
                    panic!(
                        "at {w} columns, {a:?}'s inspector sets its statement apart:\n{}",
                        lines.join("\n")
                    )
                });
                let after = squash(&lines[gap + 1..].join(" "));
                assert!(
                    after.starts_with(&blurb),
                    "at {w} columns, {a:?}'s inspector states its blurb {blurb:?} as its \
                     statement (TUI-NAV-018):\n{}",
                    lines.join("\n")
                );
            }
            let none = inspector_lines(&state, None, w);
            assert_eq!(
                none,
                vec!["No Adventure".to_string()],
                "at {w} columns, the no-adventure row states its name and nothing else \
                 (TUI-NAV-018)"
            );
        }
    }
}
