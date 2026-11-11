//! New rows: how a reveal is found. A reveal's line names the item and the
//! tab that lists it; its row wears `★` in its cursor column until the cursor
//! rests on it; a tab other than the focused one is badged until visited; and
//! a list's overflow note counts the new rows past its ends.
//!
//! @spec TUI-NAV-209, TUI-NAV-210, TUI-NAV-211, TUI-NAV-212, TUI-NAV-213,
//! TUI-NAV-214, TUI-NAV-215, TUI-RENDER-037, TUI-RENDER-038, TUI-THEME-032
//!
//! **Why through the loop.** A row is new from the tick whose reveal scan
//! reveals its item, and only for the ticks the game runs in play. So the
//! marking claims are driven through `App::step` with a clock reading worth
//! exactly one tick, against a meadow whose next tick reveals one structure
//! and one idea (`eve`). Everything else is read from what the frame draws.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use clowder_core::content::{idea_def, structure_def};
use clowder_core::{
    begin_load, content, load_string, save_string, ColonyId, ContentId, Event, GameState, IdeaId,
    JobId, Kitten, KittenTrait, Location, ResourceId, StructureId,
};
use clowder_tui::nav::rows::{drawn_rows, rows, RowKind};
use clowder_tui::nav::{opening, Overlay, RowKey, Section, ViewState};
use clowder_tui::render::widgets::{chrome, overlays};
use clowder_tui::runtime::clock::Reading;
use clowder_tui::runtime::{App, TickAccumulator, Wake};
use clowder_tui::theme::Role;
use crossterm::event::{Event as TermEvent, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::Terminal;

/// Wide and tall enough that the tab bar names every tab and a long list
/// shows a good stretch of rows.
const WIDE_W: u16 = 200;
const WIDE_H: u16 = 120;

// ---------------------------------------------------------------------------
// The eve of a reveal
// ---------------------------------------------------------------------------

fn kitten(name: &str) -> Kitten {
    Kitten {
        name: name.to_string(),
        job: Some(JobId::ALL[0]),
        location: Location::Home,
        arrived_day: 0,
        job_seq: None,
        away_seq: None,
        skills: [0.0; 8],
        exp: 0.0,
        rank: 0,
        trait_id: KittenTrait::None,
        tinkering: None,
        profile: None,
    }
}

/// A meadow far along, with every structure and every idea revealed and
/// every idea known.
fn far_along() -> GameState {
    let mut s = GameState::new(7);
    s.ticks = 362 * content::TICKS_PER_DAY;
    for r in ResourceId::ALL {
        s.resources[r.ix()] = 1.0e9;
    }
    for b in StructureId::ALL {
        s.structures[b.ix()] = 2;
    }
    s.ideas = IdeaId::ALL.iter().copied().collect::<BTreeSet<_>>();
    s.revealed = StructureId::ALL
        .iter()
        .map(|b| ContentId::Structure(*b))
        .chain(IdeaId::ALL.iter().map(|i| ContentId::Idea(*i)))
        .collect::<BTreeSet<_>>();
    s.peak_devotion = 1.0e9;
    s.discovered = ColonyId::ALL.iter().copied().collect::<BTreeSet<_>>();
    s.generation = 1;
    s.kittens = (0..6).map(|i| kitten(&format!("resident-{i}"))).collect();
    s.roster_high_water = 100;
    // One tick records what the meadow holds, so its stores are listed from
    // the start.
    s.tick();
    s
}

/// Through a real save and load, so the core rebuilds what it derives from
/// the saved state (its reveal worklists among it).
fn reloaded(state: &GameState) -> GameState {
    let text = save_string(state, 1_000_000);
    load_string(&text, 1_000_000)
        .expect("fixture: a save this binary wrote loads")
        .state
}

/// The reveals the next tick of `state` emits, in the order emitted.
fn next_reveals(state: &GameState) -> Vec<ContentId> {
    let mut probe = state.clone();
    probe
        .tick()
        .into_iter()
        .filter_map(|e| match e {
            Event::Revealed { id } => Some(id),
            _ => None,
        })
        .collect()
}

/// A meadow whose next tick reveals exactly one structure and one idea, and
/// the two. The idea is left unknown, so its row is an unfinished one; the
/// structure is the first whose reveal the next tick makes.
struct Eve {
    state: GameState,
    structure: StructureId,
    idea: IdeaId,
}

fn eve() -> Eve {
    let idea = *IdeaId::ALL.last().expect("ideas exist");
    for structure in StructureId::ALL {
        let mut s = far_along();
        s.ideas.remove(&idea);
        s.revealed.remove(&ContentId::Idea(idea));
        s.revealed.remove(&ContentId::Structure(structure));
        let s = reloaded(&s);
        let reveals = next_reveals(&s);
        if reveals.contains(&ContentId::Structure(structure))
            && reveals.contains(&ContentId::Idea(idea))
        {
            assert_eq!(
                reveals.len(),
                2,
                "fixture: only the two items left unrevealed can reveal"
            );
            return Eve {
                state: s,
                structure,
                idea,
            };
        }
    }
    panic!("fixture: no structure reveals beside the last idea on the next tick");
}

fn structure_key(s: StructureId) -> RowKey {
    RowKey::Content(ContentId::Structure(s), "")
}

fn idea_key(i: IdeaId) -> RowKey {
    RowKey::Content(ContentId::Idea(i), "")
}

// ---------------------------------------------------------------------------
// The loop
// ---------------------------------------------------------------------------

fn save_path(name: &str) -> PathBuf {
    std::env::temp_dir()
        .join(format!("clowder-new-rows-{}-{name}", std::process::id()))
        .join("save.json")
}

/// A sitting opened on `state` exactly as start-up opens one.
fn opened(state: GameState, name: &str) -> App {
    let mut view = ViewState::default();
    opening(&mut view, &state);
    App {
        state,
        view,
        clock: TickAccumulator::default(),
        save_path: save_path(name),
        last_save: Instant::now(),
        prev_layout: None,
        pointer_record: None,
        quitting: false,
        save_failed: false,
    }
}

fn terminal(w: u16, h: u16) -> Terminal<TestBackend> {
    Terminal::new(TestBackend::new(w, h)).expect("TestBackend is infallible")
}

fn still() -> Reading {
    Reading {
        monotonic: Duration::ZERO,
        wall: Duration::ZERO,
    }
}

/// A frame with the clock held still: whatever changed, the test changed.
fn frame(app: &mut App, term: &mut Terminal<TestBackend>, input: &[TermEvent]) {
    app.step(term, still(), input)
        .expect("a step against a TestBackend does not fail");
}

/// A frame whose clock reading earns exactly one tick at normal speed.
fn tick_frame(app: &mut App, term: &mut Terminal<TestBackend>) {
    assert_eq!(app.state.speed, 1.0, "fixture: normal speed");
    let wait = app.poll_timeout();
    let before = app.state.ticks;
    app.step(
        term,
        Reading {
            monotonic: wait,
            wall: wait,
        },
        &[],
    )
    .expect("a step against a TestBackend does not fail");
    assert_eq!(
        app.state.ticks,
        before + 1,
        "fixture: the frame ran one tick"
    );
}

fn press(code: KeyCode) -> TermEvent {
    TermEvent::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn press_char(c: char) -> TermEvent {
    TermEvent::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE))
}

fn digit(section: Section) -> TermEvent {
    press_char(section.digit())
}

/// The sitting on the eve, opened on the Meadow, with the reveal's tick run.
fn revealed_in_play(name: &str, w: u16, h: u16) -> (App, Terminal<TestBackend>, Eve) {
    let e = eve();
    let mut app = opened(e.state.clone(), name);
    let mut term = terminal(w, h);
    frame(&mut app, &mut term, &[]);
    tick_frame(&mut app, &mut term);
    (app, term, e)
}

// ---------------------------------------------------------------------------
// Reading the frame
// ---------------------------------------------------------------------------

fn line_cells(buf: &Buffer, y: u16, w: u16) -> Vec<String> {
    (0..w)
        .map(|x| {
            buf.cell((x, y))
                .map(|c| c.symbol().to_string())
                .unwrap_or_else(|| " ".into())
        })
        .collect()
}

fn line_text(buf: &Buffer, y: u16, w: u16) -> String {
    line_cells(buf, y, w).concat()
}

/// The frame's line drawing the row named `name`, with the column the name
/// begins at: the first line on which `name` follows a marker glyph and a
/// space, so a ticker or inspector line naming it is passed over.
fn row_at(buf: &Buffer, w: u16, h: u16, name: &str) -> Option<(u16, u16)> {
    for y in 0..h {
        let cells = line_cells(buf, y, w);
        let text = cells.concat();
        let Some(byte) = text.find(name) else {
            continue;
        };
        let col = text[..byte].chars().count();
        if col >= 4 {
            let marker = &cells[col - 2];
            if ["●", "◐", "○", "✓"].contains(&marker.as_str()) && cells[col - 1] == " " {
                return Some((col as u16, y));
            }
        }
    }
    None
}

/// What the row named `name` draws in its cursor column: the column four
/// before its name.
fn cursor_column(buf: &Buffer, w: u16, h: u16, name: &str) -> String {
    let (col, y) =
        row_at(buf, w, h, name).unwrap_or_else(|| panic!("no row named {name:?} is drawn"));
    buf.cell((col - 4, y)).unwrap().symbol().to_string()
}

fn structure_name(s: StructureId) -> &'static str {
    structure_def(s).name
}

fn idea_name(i: IdeaId) -> &'static str {
    idea_def(i).name
}

/// The tabs the bar draws, each as (section, badged), read from what
/// `chrome::tabs` draws at a width where every tab is named.
fn tab_bar(state: &GameState, view: &ViewState) -> Vec<(Section, bool)> {
    let mut term = terminal(WIDE_W, 1);
    term.draw(|f| {
        let area = f.area();
        chrome::tabs(f, area, state, view);
    })
    .expect("TestBackend is infallible");
    let line = line_text(term.backend().buffer(), 0, WIDE_W);
    let mut out = Vec::new();
    for token in line.split_whitespace() {
        if let Some(d) = token.chars().find(|c| c.is_ascii_digit()) {
            let section = Section::from_digit(d).expect("a tab's digit");
            out.push((section, token.contains('★')));
        }
    }
    out
}

fn badged(state: &GameState, view: &ViewState) -> Vec<Section> {
    tab_bar(state, view)
        .into_iter()
        .filter(|(_, b)| *b)
        .map(|(s, _)| s)
        .collect()
}

// ---------------------------------------------------------------------------
// TUI-NAV-209: a reveal's line
// ---------------------------------------------------------------------------

fn ticker_text(state: &GameState) -> String {
    let w = 160;
    let mut term = terminal(w, 1);
    term.draw(|f| overlays::ticker(f, Rect::new(0, 0, w, 1), state))
        .expect("TestBackend is infallible");
    line_text(term.backend().buffer(), 0, w)
}

fn journal_text(state: &GameState, view: &ViewState) -> String {
    let (w, h) = (160u16, 40u16);
    let mut term = terminal(w, h);
    term.draw(|f| overlays::journal(f, Rect::new(0, 0, w, h), state, view))
        .expect("TestBackend is infallible");
    let buf = term.backend().buffer().clone();
    (0..h)
        .map(|y| line_text(&buf, y, w))
        .collect::<Vec<_>>()
        .join("\n")
}

fn tab_words(section: Section) -> String {
    format!("{} {}", section.digit(), section.title())
}

/// A reveal's line names the item, says it looks possible, and names the tab
/// of the section that lists its kind, as digit and name — a structure's the
/// home section and an idea's the Ideas section — on the ticker and in the
/// Journal alike, carrying no badge even where that tab wears one.
/// @spec TUI-NAV-209
#[test]
fn a_reveals_line_names_the_item_and_the_tab_that_lists_it() {
    let s0 = far_along();
    let structure = StructureId::ALL[0];
    let idea = IdeaId::ALL[0];
    let cases = [
        (
            ContentId::Structure(structure),
            structure_name(structure),
            Section::Meadow,
        ),
        (ContentId::Idea(idea), idea_name(idea), Section::Ideas),
    ];
    for (id, name, section) in cases {
        let mut s = s0.clone();
        s.journal.record(s.ticks, &Event::Revealed { id });
        let want = format!("{name} looks possible · {}", tab_words(section));

        let ticker = ticker_text(&s);
        assert!(
            ticker.contains(&want),
            "the ticker's line for a reveal of {id:?} should read {want:?}; it drew {:?}",
            ticker.trim_end()
        );

        // The tab badged, to show the line takes no badge from it.
        let mut view = ViewState {
            section: Some(Section::Clowder),
            overlay: Some(Overlay::Journal { anchor: 0 }),
            ..ViewState::default()
        };
        view.seen[section.index()].visited = false;
        let journal = journal_text(&s, &view);
        let line = journal
            .lines()
            .find(|l| l.contains(name))
            .unwrap_or_else(|| panic!("the Journal drew no line naming {name:?}:\n{journal}"));
        assert!(
            line.contains(&want),
            "the Journal's line for a reveal of {id:?} should read {want:?}; it drew {:?}",
            line.trim_end()
        );
        assert!(
            !line.contains(&format!("★{}", section.digit())),
            "a reveal's line names the tab with no badge; it drew {:?}",
            line.trim_end()
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-210: every reveal has a row
// ---------------------------------------------------------------------------

/// The words of the group headers the Meadow's structures stand under.
const FAMILY_HEADERS: [&str; 10] = [
    "homes",
    "gardens & diggings",
    "storerooms",
    "studies",
    "workshops",
    "comforts",
    "power sources",
    "tidiers",
    "marvels",
    "outposts",
];

/// The header of the group `key`'s row stands in, in `list`.
fn group_of<'a>(list: &'a [clowder_tui::nav::Row], key: &RowKey) -> Option<&'a RowKey> {
    let at = list.iter().position(|r| &r.key == key)?;
    list[..at]
        .iter()
        .rev()
        .find(|r| matches!(r.kind, RowKind::Group))
        .map(|r| &r.key)
}

/// Every structure the core reports revealed is listed in the home section,
/// in one of the structure families' groups, and every idea in the Ideas
/// section — in a fresh meadow and in one far along alike, so a reveal's
/// line never names a tab without the row.
/// @spec TUI-NAV-210
#[test]
fn every_structure_and_idea_the_core_reveals_is_listed_in_its_section() {
    let fresh = GameState::new(11);
    let far = far_along();
    for (label, base) in [("a fresh meadow", fresh), ("a meadow far along", far)] {
        for s in StructureId::ALL {
            let mut state = base.clone();
            state.revealed.insert(ContentId::Structure(s));
            let list = rows(&state, Section::Meadow);
            let key = structure_key(s);
            assert!(
                list.iter().any(|r| r.key == key),
                "in {label}, revealed structure {s:?} has no row in the home section"
            );
            match group_of(&list, &key) {
                Some(RowKey::Fixed(words)) if FAMILY_HEADERS.contains(words) => {}
                other => panic!(
                    "in {label}, structure {s:?} stands under {other:?}, not a family's header"
                ),
            }
        }
        for i in IdeaId::ALL {
            let mut state = base.clone();
            state.revealed.insert(ContentId::Idea(i));
            assert!(
                rows(&state, Section::Ideas)
                    .iter()
                    .any(|r| r.key == idea_key(i)),
                "in {label}, revealed idea {i:?} has no row in the Ideas section"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-211, TUI-RENDER-037, TUI-THEME-032: the mark
// ---------------------------------------------------------------------------

/// A reveal from a tick the game runs in play marks its row: the row draws
/// `★` in its cursor column, in `milestone`, the glyph alone; before the tick
/// the same row (in the same state, unrevealed) is not drawn at all.
/// @spec TUI-NAV-211, TUI-RENDER-037, TUI-THEME-032
#[test]
fn a_reveal_in_play_marks_its_row_with_a_star_in_its_cursor_column() {
    let e = eve();
    let name = structure_name(e.structure);
    let mut app = opened(e.state.clone(), "marks");
    let mut term = terminal(WIDE_W, WIDE_H);
    frame(&mut app, &mut term, &[]);
    assert!(
        row_at(term.backend().buffer(), WIDE_W, WIDE_H, name).is_none(),
        "fixture: the structure is not listed before its reveal"
    );

    tick_frame(&mut app, &mut term);
    let buf = term.backend().buffer().clone();
    let (col, y) = row_at(&buf, WIDE_W, WIDE_H, name)
        .unwrap_or_else(|| panic!("the revealed structure's row is not drawn"));
    let star = buf.cell((col - 4, y)).unwrap();
    assert_eq!(
        star.symbol(),
        "★",
        "a row revealed in play draws ★ in its cursor column; the line reads {:?}",
        line_text(&buf, y, WIDE_W).trim_end()
    );
    assert_eq!(
        star.fg,
        Role::Milestone.color().expect("milestone has a colour"),
        "a new row's ★ wears milestone"
    );
    assert_eq!(
        buf.cell((col - 3, y)).unwrap().symbol(),
        " ",
        "the mark takes the cursor's column alone, the blank after it standing"
    );
}

/// The mark moves no other character of its row: the row drawn with the mark
/// and the same row drawn with it cleared differ in the cursor column alone.
/// @spec TUI-RENDER-037
#[test]
fn the_mark_moves_no_other_character_of_its_row() {
    let (mut app, mut term, e) = revealed_in_play("moves-nothing", WIDE_W, WIDE_H);
    let name = structure_name(e.structure);
    let buf = term.backend().buffer().clone();
    let (col, y) = row_at(&buf, WIDE_W, WIDE_H, name).expect("the new row is drawn");
    let marked = line_cells(&buf, y, WIDE_W);

    app.view.new_rows.clear();
    frame(&mut app, &mut term, &[]);
    let buf = term.backend().buffer().clone();
    let (col2, y2) = row_at(&buf, WIDE_W, WIDE_H, name).expect("the row is still drawn");
    assert_eq!((col, y), (col2, y2), "the row stands where it stood");
    let plain = line_cells(&buf, y2, WIDE_W);
    for x in 0..WIDE_W as usize {
        if x == col as usize - 4 {
            assert_eq!(marked[x], "★");
            assert_eq!(plain[x], " ", "an unmarked row's cursor column is blank");
        } else {
            assert_eq!(
                marked[x], plain[x],
                "column {x} differs between the marked and unmarked row"
            );
        }
    }
}

/// A reveal inside a load's catch-up is marked once the game opens, from the
/// load's report: the row draws `★`, and the tab of a section other than the
/// one the opening focuses is badged, while the focused one is not.
/// @spec TUI-NAV-214, TUI-NAV-024
#[test]
fn a_reveal_inside_a_catch_up_is_marked_once_the_game_opens() {
    let e = eve();
    let saved_at = 1_000_000u64;
    let text = save_string(&e.state, saved_at);
    let catch_up = begin_load(&text, saved_at + 30).expect("the save loads");
    let mut term = terminal(WIDE_W, WIDE_H);
    let mut wake = Wake::begin(catch_up, save_path("catch-up"), &mut term).expect("wakes");
    let mut frames = 0;
    let mut app = loop {
        match wake {
            Wake::Awake(app) => break app,
            Wake::Waking(w) => {
                frames += 1;
                assert!(frames < 1000, "the wake never finished");
                wake = w.step(&mut term, still(), &[]).expect("a waking frame");
            }
        }
    };
    assert!(
        app.state
            .revealed
            .contains(&ContentId::Structure(e.structure)),
        "fixture: the catch-up revealed the structure"
    );
    frame(&mut app, &mut term, &[]);
    assert_eq!(
        app.view.section,
        Some(Section::Meadow),
        "the opening's focus"
    );
    assert_eq!(
        cursor_column(
            term.backend().buffer(),
            WIDE_W,
            WIDE_H,
            structure_name(e.structure)
        ),
        "★",
        "a reveal inside a catch-up is marked once the game opens"
    );
    assert_eq!(
        badged(&app.state, &app.view),
        vec![Section::Ideas],
        "the tab of the section the absence revealed a row in is badged, the focused one not"
    );
    assert!(
        app.view.new_rows.contains(&ContentId::Idea(e.idea)),
        "the idea's row is marked too"
    );
}

/// A nap's reveals are marked from the report its command hands back, and
/// the tab of each section they stand in, other than the focused one, is
/// badged.
/// @spec TUI-NAV-214
#[test]
fn a_naps_reveals_are_marked_from_its_report() {
    let e = eve();
    assert!(
        e.state.ideas.contains(&IdeaId::DeepNapTheory),
        "fixture: naps are open"
    );
    let mut state = e.state.clone();
    let mut view = ViewState::default();
    opening(&mut view, &state);
    let events = state
        .apply(clowder_core::Command::DeepNap(1))
        .expect("the nap is affordable");
    clowder_tui::nav::queue_command_reports(&mut view, &state, &events);
    assert!(
        view.new_rows.contains(&ContentId::Structure(e.structure))
            && view.new_rows.contains(&ContentId::Idea(e.idea)),
        "a nap's reveals are marked: {:?}",
        view.new_rows
    );
    assert_eq!(
        badged(&state, &view),
        vec![Section::Ideas],
        "the Ideas tab is badged; the focused Meadow's is not"
    );
}

/// A report's pinned entry names each structure and idea it revealed, a line
/// each, in the report's order, after the kittens and before the seasons, in
/// the words of a reveal's line.
/// @spec TUI-NAV-215, TUI-NAV-027
#[test]
fn a_reports_pinned_entry_names_what_it_revealed() {
    let mut s = far_along();
    let structure = StructureId::ALL[0];
    let idea = IdeaId::ALL[0];
    let report = clowder_core::OfflineReport {
        elapsed_seconds: 3 * 86_400,
        arrived: vec![("Biscuit".to_string(), 1)],
        seasons_passed: 2,
        revealed: vec![ContentId::Structure(structure), ContentId::Idea(idea)],
        ..clowder_core::OfflineReport::default()
    };
    s.journal.record_away(s.ticks, &report);
    let view = ViewState {
        section: Some(Section::Meadow),
        overlay: Some(Overlay::Journal { anchor: 0 }),
        journal_pinned: vec![s.journal.kept.len() - 1],
        ..ViewState::default()
    };
    let text = journal_text(&s, &view);
    let at = |needle: &str| {
        text.find(needle)
            .unwrap_or_else(|| panic!("the entry has no {needle:?}:\n{text}"))
    };
    let kitten = at("Biscuit arrived");
    let first = at(&format!(
        "{} looks possible · {}",
        structure_name(structure),
        tab_words(Section::Meadow)
    ));
    let second = at(&format!(
        "{} looks possible · {}",
        idea_name(idea),
        tab_words(Section::Ideas)
    ));
    let seasons = at("2 seasons passed");
    assert!(
        kitten < first && first < second && second < seasons,
        "the reveals stand after the kittens and before the seasons, in the report's order:\n{text}"
    );
    let lines_with = |needle: &str| text.lines().filter(|l| l.contains(needle)).count();
    assert_eq!(lines_with("looks possible"), 2, "a line each:\n{text}");
}

// ---------------------------------------------------------------------------
// TUI-NAV-212: when the mark clears
// ---------------------------------------------------------------------------

/// The cursor resting on the row at the end of a frame clears its mark, and
/// the mark stays cleared once the cursor moves on.
/// @spec TUI-NAV-212, TUI-RENDER-037
#[test]
fn the_cursor_resting_on_a_new_row_clears_its_mark() {
    let (mut app, mut term, e) = revealed_in_play("rest-clears", WIDE_W, WIDE_H);
    let name = structure_name(e.structure);
    let key = structure_key(e.structure);
    let list = drawn_rows(&app.state, Section::Meadow, &app.view);
    let at = list.iter().position(|r| r.key == key).expect("listed");
    let before = list[..at]
        .iter()
        .rev()
        .find(|r| r.kind != RowKind::Group)
        .expect("a row stands above the structure's")
        .key
        .clone();

    // Cursor onto the row above, then one press down onto the new row.
    app.view.focus[Section::Meadow.index()] = Some(before);
    frame(&mut app, &mut term, &[]);
    assert_eq!(
        cursor_column(term.backend().buffer(), WIDE_W, WIDE_H, name),
        "★",
        "fixture: still marked with the cursor on the row above"
    );
    frame(&mut app, &mut term, &[press(KeyCode::Down)]);
    assert_eq!(
        cursor_column(term.backend().buffer(), WIDE_W, WIDE_H, name),
        "▸",
        "the cursor stands on the new row, and the cursor's glyph, not the mark, is drawn"
    );
    frame(&mut app, &mut term, &[press(KeyCode::Up)]);
    assert_eq!(
        cursor_column(term.backend().buffer(), WIDE_W, WIDE_H, name),
        " ",
        "the cursor rested on the row, so the mark is cleared once it moves on"
    );
}

/// A row the cursor passes over within one frame, coming to rest beyond it,
/// keeps its mark: resting is where the cursor stands at a frame's end.
/// @spec TUI-NAV-212
#[test]
fn a_new_row_passed_over_within_one_frame_keeps_its_mark() {
    let (mut app, mut term, e) = revealed_in_play("pass-over", WIDE_W, WIDE_H);
    let name = structure_name(e.structure);
    let key = structure_key(e.structure);
    let list = drawn_rows(&app.state, Section::Meadow, &app.view);
    let at = list.iter().position(|r| r.key == key).expect("listed");
    let before = list[..at]
        .iter()
        .rev()
        .find(|r| r.kind != RowKind::Group)
        .expect("a row stands above")
        .key
        .clone();
    assert!(
        list[at + 1..].iter().any(|r| r.kind != RowKind::Group),
        "fixture: a row stands below the structure's"
    );
    app.view.focus[Section::Meadow.index()] = Some(before);
    frame(&mut app, &mut term, &[]);
    frame(
        &mut app,
        &mut term,
        &[press(KeyCode::Down), press(KeyCode::Down)],
    );
    assert_eq!(
        cursor_column(term.backend().buffer(), WIDE_W, WIDE_H, name),
        "★",
        "a row the cursor only passed over within a frame keeps its mark"
    );
}

/// Nothing but the cursor's resting clears the mark: leaving the section and
/// coming back, opening and closing the Journal, a resize, and the
/// simulation advancing all leave it standing.
/// @spec TUI-NAV-212
#[test]
fn nothing_but_the_cursor_resting_clears_the_mark() {
    let (mut app, mut term, e) = revealed_in_play("only-rest", WIDE_W, WIDE_H);
    let name = structure_name(e.structure);
    let marked = |term: &Terminal<TestBackend>, w: u16, h: u16, what: &str| {
        assert_eq!(
            cursor_column(term.backend().buffer(), w, h, name),
            "★",
            "{what} cleared the new row's mark"
        );
    };
    marked(&term, WIDE_W, WIDE_H, "the reveal itself");

    frame(&mut app, &mut term, &[digit(Section::Clowder)]);
    frame(&mut app, &mut term, &[digit(Section::Meadow)]);
    marked(
        &term,
        WIDE_W,
        WIDE_H,
        "leaving the section and visiting it again",
    );

    frame(&mut app, &mut term, &[press_char('e')]);
    frame(&mut app, &mut term, &[press(KeyCode::Esc)]);
    marked(&term, WIDE_W, WIDE_H, "opening and closing the Journal");

    let mut small = terminal(120, 100);
    frame(&mut app, &mut small, &[]);
    frame(&mut app, &mut term, &[]);
    marked(&term, WIDE_W, WIDE_H, "a resize and back");

    for _ in 0..20 {
        tick_frame(&mut app, &mut term);
    }
    marked(&term, WIDE_W, WIDE_H, "the simulation advancing");
}

// ---------------------------------------------------------------------------
// TUI-NAV-213: the tab
// ---------------------------------------------------------------------------

/// A reveal in a section other than the one in focus badges that section's
/// tab, already in the bar, until it is next visited; the visit clears the
/// badge and leaves the row's mark.
/// @spec TUI-NAV-213, TUI-NAV-212
#[test]
fn a_reveal_elsewhere_badges_that_sections_tab_until_its_visit() {
    let e = eve();
    let mut app = opened(e.state.clone(), "tab");
    let mut term = terminal(WIDE_W, WIDE_H);
    frame(&mut app, &mut term, &[digit(Section::Clowder)]);
    assert!(
        badged(&app.state, &app.view).is_empty(),
        "fixture: no tab is badged before the reveal"
    );
    tick_frame(&mut app, &mut term);
    let b = badged(&app.state, &app.view);
    assert!(
        b.contains(&Section::Meadow) && b.contains(&Section::Ideas),
        "a reveal badges the tab of each section it lists a row in, other than the focused one; \
         badged: {b:?}"
    );

    frame(&mut app, &mut term, &[digit(Section::Meadow)]);
    assert!(
        !badged(&app.state, &app.view).contains(&Section::Meadow),
        "a visit clears the badge"
    );
    assert_eq!(
        cursor_column(
            term.backend().buffer(),
            WIDE_W,
            WIDE_H,
            structure_name(e.structure)
        ),
        "★",
        "a visit leaves the row's mark; only the cursor reaching it clears that"
    );
    assert!(
        badged(&app.state, &app.view).contains(&Section::Ideas),
        "a visit elsewhere leaves another section's badge"
    );

    // Entering the Ideas section lands its cursor as TUI-NAV-032 has it. A
    // landing is a resting place like any other: the new idea's mark clears
    // exactly where the landing chose its row, and stands otherwise.
    frame(&mut app, &mut term, &[digit(Section::Ideas)]);
    assert!(
        !badged(&app.state, &app.view).contains(&Section::Ideas),
        "a visit clears the Ideas tab's badge"
    );
    let landed_on_it = app.view.focus[Section::Ideas.index()] == Some(idea_key(e.idea));
    assert_eq!(
        app.view.new_rows.contains(&ContentId::Idea(e.idea)),
        !landed_on_it,
        "the new idea's mark should clear exactly where the landing rested on its row"
    );
    if !landed_on_it {
        app.view.focus[Section::Ideas.index()] = Some(idea_key(e.idea));
        frame(&mut app, &mut term, &[]);
        assert!(
            !app.view.new_rows.contains(&ContentId::Idea(e.idea)),
            "the cursor resting on the new idea's row clears its mark"
        );
    }
}

/// A reveal in the section in focus badges no tab: moving away afterwards
/// leaves that section's tab unbadged, though its row stays marked.
/// @spec TUI-NAV-213
#[test]
fn a_reveal_in_the_focused_section_badges_no_tab() {
    let (mut app, mut term, _e) = revealed_in_play("focused", WIDE_W, WIDE_H);
    assert_eq!(
        app.view.section,
        Some(Section::Meadow),
        "fixture: on the Meadow"
    );
    frame(&mut app, &mut term, &[digit(Section::Clowder)]);
    assert!(
        !badged(&app.state, &app.view).contains(&Section::Meadow),
        "the Meadow was in focus at the reveal, so its tab wears no badge after"
    );
}

// ---------------------------------------------------------------------------
// TUI-RENDER-038: past the list's ends
// ---------------------------------------------------------------------------

/// The text of the frame's lines that hold an overflow note.
fn notes(buf: &Buffer, w: u16, h: u16) -> Vec<(u16, String)> {
    (0..h)
        .map(|y| (y, line_text(buf, y, w)))
        .filter(|(_, l)| l.contains(" more above") || l.contains(" more below"))
        .collect()
}

/// Where a new row lies past an end of a scrolled list, that end's note ends
/// ` · ★N new`, the `★` in `milestone`; before the reveal it carries no such
/// ending.
/// @spec TUI-RENDER-038, TUI-THEME-032
#[test]
fn a_lists_note_counts_the_new_rows_past_its_end() {
    let e = eve();
    let (w, h) = (100u16, 24u16);
    let mut app = opened(e.state.clone(), "note");
    let mut term = terminal(w, h);
    // The cursor on the Meadow's last row, so the structures lie above.
    let last = drawn_rows(&app.state, Section::Meadow, &app.view)
        .into_iter()
        .rev()
        .find(|r| r.kind != RowKind::Group)
        .expect("the Meadow lists rows")
        .key;
    app.view.focus[Section::Meadow.index()] = Some(last);
    frame(&mut app, &mut term, &[]);
    let before = notes(term.backend().buffer(), w, h);
    assert!(
        before.iter().any(|(_, l)| l.contains(" more above")),
        "fixture: the Meadow's list runs past its top; notes: {before:?}"
    );
    assert!(
        before.iter().all(|(_, l)| !l.contains('★')),
        "no note counts a new row before any reveal; notes: {before:?}"
    );

    tick_frame(&mut app, &mut term);
    let buf = term.backend().buffer().clone();
    let after = notes(&buf, w, h);
    let (y, above) = after
        .iter()
        .find(|(_, l)| l.contains(" more above"))
        .expect("the note above is still drawn");
    assert!(
        above.contains(" more above · ★1 new"),
        "the note above should count the one new row past it; it reads {:?}",
        above.trim_end()
    );
    let x = above.chars().position(|c| c == '★').expect("a star") as u16;
    assert_eq!(
        buf.cell((x, *y)).unwrap().fg,
        Role::Milestone.color().expect("milestone has a colour"),
        "the note's ★ wears milestone"
    );
}
