//! The pointer: where each frame drew what a click or the wheel can address
//! (render leaf, The Pointer Record), and what a pointer event means against
//! that frame (navigation leaf, The Pointer).
//!
//! The mouse only moves around. tmux hands the program the very click that
//! focused its pane, so every test here holds a pointer event to two claims at
//! once: it lands on what the player saw under it, and it changes nothing but
//! where the cursor stands, which section is in focus, or how far the Journal
//! is scrolled. A pointer event never reaches the core.
//!
//! **Where things are found on screen.** Never from the record under test,
//! which would let a wrong record vouch for itself:
//!
//! * a tab's label is read off the tab bar's cells — its digit, the `★` before
//!   it where worn, and its name after it where the bar draws one;
//! * a row's line is read off the list's cells. Each selectable row is drawn
//!   once with the cursor on it, and the line wearing the cursor glyph there
//!   is that row's own text; a line of any other frame reading the same, the
//!   cursor glyph aside, is that row. A line that no row reads as is not a
//!   row: a group header, the done header, an overflow note, an armed row's
//!   summary line, a blank line below the last row.
//!
//! **The seams this file drives.** The loop does not yet hand pointer events
//! over (that is the runtime leaf's next slice), so these tests go through the
//! entry point it will use:
//!
//! * `render::frame_recorded(f, state, view, prev) -> (Layout, PointerRecord)`
//!   — the frame as `render::frame` draws it, with its pointer record as a
//!   second output;
//! * `render::waking_recorded(f, total, remaining) -> PointerRecord` — the
//!   waking screen as `render::waking` draws it, with its record;
//! * `render::pointer::PointerRecord` (`Debug`, `Clone`, `PartialEq`), asked
//!   `size()`, `tab_at(x, y)`, `row_at(x, y)` (the section and the row's
//!   identity), `list_at(x, y)`, `journal_at(x, y)`, `journal_max_scroll()`
//!   and `is_empty()`;
//! * `nav::pointer_event(event, &mut view, &state, &record, present_size)` —
//!   one crossterm mouse event resolved against the last frame's record at the
//!   terminal's present size, answering with the same `Outcome` a key does.

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use clowder_core::content::MEADOW_MOVE_MIN_KITTENS;
use clowder_core::{
    ArchId, ColonyId, Command, ContentId, CraftId, Event, GameState, IdeaId, JobId, Kitten,
    KittenTrait, Location, LockReason, OfflineReport, ResourceId, RiteId, RuleId, SiteId,
    StructureId,
};
use clowder_tui::nav::rows::{drawn_rows, lists_selectable, rows, tab_sections, Row, RowKind};
use clowder_tui::nav::{
    self, update, Action, ArmedRow, CountPrompt, Outcome, Overlay, PendingReport, RowKey, Section,
    ViewState,
};
use clowder_tui::render::layout::{Body, Layout};
use clowder_tui::render::pointer::PointerRecord;
use clowder_tui::render;
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::Terminal;

// ---------------------------------------------------------------------------
// Sizes
// ---------------------------------------------------------------------------

/// The floor: the narrow layout, its tab bar in the compact strip.
const SMALL: (u16, u16) = (80, 24);
/// Past both breakpoints: three panes, the tab bar in full.
const WIDE: (u16, u16) = (130, 40);
const SIZES: [(u16, u16); 2] = [SMALL, WIDE];

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn kitten(name: &str, job: Option<JobId>) -> Kitten {
    Kitten {
        name: name.to_string(),
        job,
        location: Location::Home,
        arrived_day: 0,
        job_seq: job.map(|_| 1),
        away_seq: None,
        skills: [0.0; 8],
        exp: 1.0e6,
        rank: 0,
        trait_id: KittenTrait::None,
        tinkering: None,
    }
}

/// Every surface open and everything affordable many times over: every
/// section lists rows, every kind of row is listed somewhere, and the lists
/// run far longer than a viewport at the floor.
fn rich() -> GameState {
    let mut s = GameState::new(7);
    for r in ResourceId::ALL {
        s.resources[r.ix()] = 1.0e12;
    }
    for b in StructureId::ALL {
        s.structures[b.ix()] = 5;
    }
    s.ideas = IdeaId::ALL.iter().copied().collect::<BTreeSet<_>>();
    s.revealed = StructureId::ALL
        .iter()
        .map(|b| ContentId::Structure(*b))
        .chain(IdeaId::ALL.iter().map(|i| ContentId::Idea(*i)))
        .collect::<BTreeSet<_>>();
    s.roster_high_water = MEADOW_MOVE_MIN_KITTENS + 9;
    let mut roster: Vec<Kitten> =
        (0..MEADOW_MOVE_MIN_KITTENS + 9).map(|i| kitten(&format!("kitten-{i}"), None)).collect();
    for k in roster.iter_mut().take(4) {
        k.job = Some(JobId::Tinkerer);
    }
    roster[0].tinkering = Some(CraftId::ALL[0]);
    s.kittens = roster;
    s.elder = Some(0);
    s.peak_devotion = 1.0e12;
    s.devotion = 1.0e6;
    s.serenity = 1.0e12;
    s.rites = [RiteId::TheReverie, RiteId::Enlightenment].into_iter().collect::<BTreeSet<_>>();
    s.arches_bought = [ArchId::EverdreamArch].into_iter().collect::<BTreeSet<_>>();
    s.discovered = ColonyId::ALL.iter().copied().collect::<BTreeSet<_>>();
    s.sites_reached = BTreeSet::new();
    s.whale_visit_years = 5;
    s.whale_contentment = 1;
    s.generation = 1;
    s.elder_wisdom = 1.0e9;
    s.warm_memories = 1.0e3;
    s
}

/// Whether the core names `row`'s item already the player's.
fn finished(state: &GameState, row: &Row) -> bool {
    match &row.key {
        RowKey::Content(id, _) => state.lock_reason(*id) == Some(LockReason::AlreadyYours),
        _ => false,
    }
}

/// A meadow mid-game: `rich`, with half the ideas no other idea needs left
/// unresearched, half the refinements on offer bought and one rule adopted
/// from every other pair — so the Ideas list holds finished rows (its done
/// group) and rows still to take — and a journal far longer than any body,
/// with one away-report the core kept, for the banner.
fn mid_game() -> GameState {
    use clowder_core::content::{idea_def, refinement_def, rule_def};
    use clowder_core::RefinementId;
    let mut s = rich();
    let leaves: Vec<IdeaId> = IdeaId::ALL
        .iter()
        .copied()
        .filter(|i| !IdeaId::ALL.iter().any(|j| idea_def(*j).prereqs.contains(i)))
        .collect();
    for (n, i) in leaves.iter().enumerate() {
        if n % 2 == 0 {
            s.ideas.remove(i);
        }
    }
    let mut n = 0;
    for r in RefinementId::ALL {
        if s.ideas.contains(&refinement_def(r).gate) {
            if n % 2 == 0 {
                s.refinements.insert(r);
            }
            n += 1;
        }
    }
    let mut paired: Vec<RuleId> = Vec::new();
    let mut pair = 0;
    for r in RuleId::ALL {
        if paired.contains(&r) {
            continue;
        }
        paired.push(r);
        paired.push(rule_def(r).blocks);
        if pair % 2 == 0 {
            s.rules.insert(r);
        }
        pair += 1;
    }
    let mut report = OfflineReport { elapsed_seconds: 3 * 86_400, ..OfflineReport::default() };
    report.resource_deltas[ResourceId::ALL[0].ix()] = 4_800.0;
    s.journal.record_away(1_000, &report);
    assert_eq!(s.journal.kept.len(), 1, "fixture: the core kept the away-report");
    for i in 0..90u64 {
        let id = StructureId::ALL[i as usize % StructureId::ALL.len()];
        s.journal.record(2_000 + i, &Event::StructureBuilt { id, count: 2 });
    }
    assert!(
        rows(&s, Section::Ideas).iter().filter(|r| r.selectable() && finished(&s, r)).count() >= 2,
        "fixture: the Ideas section lists finished rows"
    );
    s
}

/// A meadow whose Sky lists nothing a cursor can rest on.
fn sky_emptied() -> GameState {
    let mut s = GameState::new(3);
    s.ideas.insert(IdeaId::SkywardLonging);
    s.sites_reached = SiteId::ALL.into_iter().collect();
    assert!(rows(&s, Section::Sky).iter().all(|r| !r.selectable()), "fixture: the Sky lists no selectable row");
    s
}

/// A sitting opened on `state`, standing on its Meadow.
fn opened(state: &GameState) -> ViewState {
    let mut v = ViewState::default();
    nav::opening(&mut v, state);
    v
}

/// A sitting opened on `state` and `section` entered by its digit.
fn standing_in(state: &GameState, section: Section) -> ViewState {
    let mut v = opened(state);
    update(Action::SectionJump(section), &mut v, state);
    assert_eq!(v.section, Some(section), "fixture: the digit entered the {} screen", section.title());
    v
}

/// The same view with the cursor on the last row the list draws.
fn deep(state: &GameState, view: &ViewState) -> ViewState {
    let section = view.section.expect("a section is in focus");
    let mut v = view.clone();
    v.focus[section.index()] =
        drawn_rows(state, section, view).into_iter().filter(|r| r.selectable()).map(|r| r.key).next_back();
    v
}

/// The one unread report the mid-game journal kept, showing as the banner.
fn with_banner(view: &ViewState) -> ViewState {
    let mut v = view.clone();
    v.reports = vec![PendingReport { entry: 0, dismissed: false }];
    v
}

/// Two tabs badged as not yet visited since they appeared — few enough that
/// the compact strip at the floor still holds every tab.
fn badge_two(view: &mut ViewState) {
    for s in [Section::Ideas, Section::Sky] {
        view.seen[s.index()].visited = false;
    }
}

/// A footer note still on its clock.
fn with_footer(view: &ViewState) -> ViewState {
    let mut v = view.clone();
    v.footer = Some(("a note".to_string(), Instant::now() + Duration::from_secs(60)));
    v
}

/// The first row of `section` whose command satisfies `pred`.
fn find_row(state: &GameState, section: Section, pred: impl Fn(&Row) -> bool) -> Row {
    rows(state, section)
        .into_iter()
        .find(|r| r.selectable() && pred(r))
        .unwrap_or_else(|| panic!("fixture: no such row on the {} screen", section.title()))
}

/// Each of the four heavy rows, with its section.
fn heavy_rows(state: &GameState) -> Vec<(Section, Row)> {
    let pick = |section: Section, f: fn(&Command) -> bool| {
        let row = find_row(state, section, |r| matches!(&r.kind, RowKind::Act { command, .. } if f(command)));
        assert!(row.heavy, "fixture: {:?} is heavy", row.key);
        (section, row)
    };
    vec![
        pick(Section::Legacy, |c| matches!(c, Command::GreatMeadowMove)),
        pick(Section::Legacy, |c| matches!(c, Command::PassItDown)),
        pick(Section::Ideas, |c| matches!(c, Command::AdoptRule(_))),
        pick(Section::Dreams, |c| matches!(c, Command::DeepNap(_))),
    ]
}

/// `view` with `row` focused and armed, its clock running well past the test.
fn armed_on(view: &ViewState, section: Section, row: &Row) -> ViewState {
    let RowKind::Act { command, .. } = &row.kind else { panic!("fixture: a heavy row acts") };
    let mut v = view.clone();
    v.focus[section.index()] = Some(row.key.clone());
    v.armed = Some(ArmedRow {
        key: row.key.clone(),
        command: *command,
        expires_at: Instant::now() + Duration::from_secs(600),
    });
    v
}

// ---------------------------------------------------------------------------
// Drawing
// ---------------------------------------------------------------------------

/// One drawn frame: its cells, its layout, its pointer record, and the size it
/// was drawn at.
struct Shot {
    buf: Buffer,
    layout: Layout,
    record: PointerRecord,
    size: (u16, u16),
}

fn terminal((w, h): (u16, u16)) -> Terminal<TestBackend> {
    Terminal::new(TestBackend::new(w, h)).expect("TestBackend is infallible")
}

/// Draw one frame into `term`, as the loop does.
fn shoot_on(term: &mut Terminal<TestBackend>, state: &GameState, view: &ViewState, prev: Option<&Layout>) -> Shot {
    let mut out = None;
    term.draw(|f| out = Some(render::frame_recorded(f, state, view, prev))).expect("TestBackend is infallible");
    let (layout, record) = out.expect("the frame was drawn");
    let buf = term.backend().buffer().clone();
    let size = (buf.area.width, buf.area.height);
    Shot { buf, layout, record, size }
}

/// Draw one frame into a terminal that has drawn nothing before.
fn shoot(state: &GameState, view: &ViewState, size: (u16, u16)) -> Shot {
    shoot_on(&mut terminal(size), state, view, None)
}

fn cell(buf: &Buffer, x: u16, y: u16) -> String {
    buf.cell((x, y)).map(|c| c.symbol().to_string()).unwrap_or_else(|| " ".into())
}

fn text(buf: &Buffer, y: u16, x0: u16, x1: u16) -> String {
    (x0..x1).map(|x| cell(buf, x, y)).collect()
}

/// The list's space: its pane with the pane dividers set aside (TUI-RENDER-016).
fn list_space(l: &Layout) -> Rect {
    match l.body {
        Body::Narrow { main, .. } => Rect::new(main.x + 1, main.y, main.width - 1, main.height),
        Body::Wide { list, .. } => Rect::new(list.x + 1, list.y, list.width - 2, list.height),
        Body::TooSmall(r) => panic!("fixture: {r:?} is below the floor"),
    }
}

/// The body a full-screen overlay is drawn over.
fn body_rect(l: &Layout) -> Rect {
    match l.body {
        Body::Narrow { sidebar, main } => sidebar.union(main),
        Body::Wide { sidebar, inspector, .. } => sidebar.union(inspector),
        Body::TooSmall(r) => r,
    }
}

/// Every cell of a rectangle.
fn cells_of(r: Rect) -> Vec<(u16, u16)> {
    (r.y..r.bottom()).flat_map(|y| (r.x..r.right()).map(move |x| (x, y))).collect()
}

/// Every cell of the frame.
fn every_cell(shot: &Shot) -> Vec<(u16, u16)> {
    cells_of(Rect::new(0, 0, shot.size.0, shot.size.1))
}

/// A line of the list with the cursor glyph read as the blank it stands in.
fn uncursored(line: &str) -> String {
    match line.strip_prefix('\u{25B8}') {
        Some(rest) => format!(" {rest}"),
        None => line.to_string(),
    }
}

fn wears_cursor(line: &str) -> bool {
    line.starts_with('\u{25B8}')
}

/// One line of the list's space as drawn, and the rows whose own text it
/// reads as — none where the line is not a row.
#[derive(Debug, Clone)]
struct ListLine {
    y: u16,
    text: String,
    rows: Vec<RowKey>,
}

/// What the list drew on each line of its space in `shot`, read off the
/// screen alone. `view` is the view `shot` was drawn from.
///
/// Sibling rows can read alike, so a line's text alone may fit several rows;
/// the list's order settles it. The rows on screen are an unbroken run of the
/// list's selectable rows in order — a window scrolls, it does not skip — so
/// each line takes the row that every placement of that run consistent with
/// all the lines' texts puts there.
fn list_map(state: &GameState, view: &ViewState, shot: &Shot) -> Vec<ListLine> {
    let section = view.section.expect("a section is in focus");
    let space = list_space(&shot.layout);
    let order: Vec<RowKey> =
        drawn_rows(state, section, view).into_iter().filter(|r| r.selectable()).map(|r| r.key).collect();
    let mut own: Vec<(RowKey, String)> = Vec::new();
    for key in &order {
        // An arming's summary lines can push a cursor below it out of view,
        // and a row reads the same armed or not, so its own text is read
        // with any arming set aside.
        let mut v = view.clone();
        v.armed = None;
        v.focus[section.index()] = Some(key.clone());
        let s = shoot(state, &v, shot.size);
        let line = (space.y..space.bottom())
            .map(|y| text(&s.buf, y, space.x, space.right()))
            .find(|t| wears_cursor(t))
            .unwrap_or_else(|| panic!("fixture: the cursor on {key:?} is drawn"));
        own.push((key.clone(), uncursored(&line)));
    }
    let mut lines: Vec<ListLine> = (space.y..space.bottom())
        .map(|y| {
            let t = uncursored(&text(&shot.buf, y, space.x, space.right()));
            let rows = own.iter().filter(|(_, o)| *o == t).map(|(k, _)| k.clone()).collect();
            ListLine { y, text: t, rows }
        })
        .collect();
    let at: Vec<usize> = (0..lines.len()).filter(|i| !lines[*i].rows.is_empty()).collect();
    if at.is_empty() {
        return lines;
    }
    let placements: Vec<usize> = (0..=order.len().saturating_sub(at.len()))
        .filter(|s| at.iter().enumerate().all(|(n, i)| lines[*i].rows.contains(&order[s + n])))
        .collect();
    assert!(!placements.is_empty(), "fixture: the rows on screen read as an unbroken run of the list");
    for (n, i) in at.iter().enumerate() {
        let mut keys: Vec<RowKey> = Vec::new();
        for s in &placements {
            if !keys.contains(&order[s + n]) {
                keys.push(order[s + n].clone());
            }
        }
        lines[*i].rows = keys;
    }
    lines
}

/// The line that wears the cursor glyph in `shot`'s list, uncursored.
fn cursor_line(shot: &Shot) -> Option<(u16, String)> {
    let space = list_space(&shot.layout);
    (space.y..space.bottom())
        .map(|y| (y, text(&shot.buf, y, space.x, space.right())))
        .find(|(_, t)| wears_cursor(t))
        .map(|(y, t)| (y, uncursored(&t)))
}

/// Each tab label the bar drew, read off its cells: its section and the
/// columns `[from, to)` it covers — the `★` where worn, the digit, and the
/// name where drawn.
fn tab_labels(shot: &Shot) -> Vec<(Section, u16, u16)> {
    let bar = shot.layout.tabs;
    let cells: Vec<String> = (bar.x..bar.right()).map(|x| cell(&shot.buf, x, bar.y)).collect();
    let mut out = Vec::new();
    for (i, c) in cells.iter().enumerate() {
        let mut chars = c.chars();
        let (Some(d), None) = (chars.next(), chars.next()) else { continue };
        let Some(section) = Section::from_digit(d) else { continue };
        let from = if i > 0 && cells[i - 1] == "\u{2605}" { i - 1 } else { i };
        let name = format!(" {}", section.title());
        let after: String = cells.iter().skip(i + 1).take(name.chars().count()).cloned().collect();
        let to = if after == name { i + 1 + name.chars().count() } else { i + 1 };
        out.push((section, bar.x + from as u16, bar.x + to as u16));
    }
    out
}

// ---------------------------------------------------------------------------
// Pointer events
// ---------------------------------------------------------------------------

fn mouse(kind: MouseEventKind, x: u16, y: u16, modifiers: KeyModifiers) -> MouseEvent {
    MouseEvent { kind, column: x, row: y, modifiers }
}

fn click(x: u16, y: u16) -> MouseEvent {
    mouse(MouseEventKind::Down(MouseButton::Left), x, y, KeyModifiers::NONE)
}

fn wheel_down(x: u16, y: u16) -> MouseEvent {
    mouse(MouseEventKind::ScrollDown, x, y, KeyModifiers::NONE)
}

fn wheel_up(x: u16, y: u16) -> MouseEvent {
    mouse(MouseEventKind::ScrollUp, x, y, KeyModifiers::NONE)
}

/// The three events that answer at all: a left press and a wheel step either
/// way.
fn answering(x: u16, y: u16) -> [MouseEvent; 3] {
    [click(x, y), wheel_down(x, y), wheel_up(x, y)]
}

/// One pointer event, resolved against `shot` at the size it was drawn at.
fn point(e: MouseEvent, view: &mut ViewState, state: &GameState, shot: &Shot) -> Outcome {
    nav::pointer_event(e, view, state, &shot.record, shot.size)
}

/// The whole view, as text.
fn whole(view: &ViewState) -> String {
    format!("{view:?}")
}

/// The view with the clock and guard a held press is judged by set aside.
fn settled(view: &ViewState) -> String {
    let mut v = view.clone();
    v.last_repeat = None;
    v.field_guard = false;
    format!("{v:?}")
}

/// The view with everything a pointer event may change set aside: the
/// section, the cursor, the visits a section move records, an arming a move
/// ends, the Journal's scroll, and the held-press guard.
fn untouchable(view: &ViewState) -> String {
    let mut v = view.clone();
    v.section = None;
    v.focus = Default::default();
    v.seen = Default::default();
    v.armed = None;
    v.last_repeat = None;
    v.field_guard = false;
    if let Some(Overlay::Journal { anchor }) = v.overlay.as_mut() {
        *anchor = 0;
    }
    format!("{v:?}")
}

/// Assert that `e` did nothing at all to `view`.
fn assert_nothing(e: MouseEvent, view: &ViewState, state: &GameState, shot: &Shot, what: &str) {
    let mut v = view.clone();
    let out = point(e, &mut v, state, shot);
    assert_eq!(out, Outcome::Handled, "{what}: {e:?} answered {out:?}");
    assert_eq!(whole(&v), whole(view), "{what}: {e:?} changed the view");
}

/// The view a move to `key` by the keys would leave, the banner aside: the
/// cursor on `key`, remembered by identity, and any arming ended.
fn moved_to(view: &ViewState, section: Section, key: &RowKey) -> ViewState {
    let mut v = view.clone();
    v.focus[section.index()] = Some(key.clone());
    v.armed = None;
    v
}

// ===========================================================================
// The Pointer Record — TUI-RENDER-025, TUI-RENDER-026
// ===========================================================================

/// The views the record sweeps stand in: every section with a tab, its cursor
/// at the top and at the foot of its list, the Ideas done group open, and the
/// four heavy rows armed.
fn record_contexts(state: &GameState) -> Vec<(String, ViewState)> {
    let mut out = Vec::new();
    for section in tab_sections(state, Some(Section::Meadow)) {
        let v = standing_in(state, section);
        out.push((format!("the {} screen, cursor on its landing", section.title()), v.clone()));
        out.push((format!("the {} screen, cursor on its last row", section.title()), deep(state, &v)));
    }
    let mut open = standing_in(state, Section::Ideas);
    open.done_open = true;
    out.push(("the Ideas screen, done group open".into(), open.clone()));
    out.push(("the Ideas screen, done group open, cursor on its last row".into(), deep(state, &open)));
    for (section, row) in heavy_rows(state) {
        let v = standing_in(state, section);
        out.push((format!("the {} screen with {:?} armed", section.title(), row.key), armed_on(&v, section, &row)));
    }
    out
}

/// One context of the sweeps, at one size, with its list read off the screen.
struct Swept {
    what: String,
    view: ViewState,
    size: (u16, u16),
    lines: Vec<ListLine>,
}

/// The sweep contexts at both sizes, the report banner showing in each, each
/// with its list read off the screen — worked out once for the whole file,
/// since reading a list off the screen draws it once per row.
fn swept() -> &'static [Swept] {
    static SWEPT: std::sync::OnceLock<Vec<Swept>> = std::sync::OnceLock::new();
    SWEPT.get_or_init(|| {
        let state = mid_game();
        let mut out = Vec::new();
        for size in SIZES {
            for (what, view) in record_contexts(&state) {
                let view = with_banner(&view);
                let lines = list_map(&state, &view, &shoot(&state, &view, size));
                out.push(Swept { what, view, size, lines });
            }
        }
        out
    })
}

/// Each tab is recorded over its label exactly as the bar drew it — the `★`
/// where worn, the digit, and the name where drawn, in the full form and the
/// compact strip alike — naming its section; no column between labels belongs
/// to a tab; and the record holds the size the frame was drawn at.
///
/// @spec TUI-RENDER-025
#[test]
fn every_tab_is_recorded_over_its_label_as_drawn_and_no_column_between_labels_is() {
    let state = mid_game();
    let mut forms = BTreeSet::new();
    for size in SIZES {
        for badged in [true, false] {
            for focus in tab_sections(&state, Some(Section::Meadow)) {
                let mut v = standing_in(&state, focus);
                if badged {
                    badge_two(&mut v);
                }
                let shot = shoot(&state, &v, size);
                assert_eq!(shot.record.size(), size, "the record does not hold the size its frame was drawn at");
                let labels = tab_labels(&shot);
                let drawn: Vec<Section> = labels.iter().map(|(s, _, _)| *s).collect();
                assert_eq!(drawn, tab_sections(&state, Some(focus)), "fixture: every tab is read off the bar at {size:?}");
                for (section, from, to) in &labels {
                    forms.insert((size, badged, to - from > 2));
                    for x in *from..*to {
                        assert_eq!(
                            shot.record.tab_at(x, shot.layout.tabs.y),
                            Some(*section),
                            "at {size:?} the cell at column {x} of the {} tab's label is not recorded as that tab",
                            section.title()
                        );
                    }
                }
                let bar = shot.layout.tabs;
                for x in bar.x..bar.right() {
                    if !labels.iter().any(|(_, from, to)| (*from..*to).contains(&x)) {
                        assert_eq!(
                            shot.record.tab_at(x, bar.y),
                            None,
                            "at {size:?} column {x} of the tab bar lies between labels but is recorded as a tab"
                        );
                    }
                }
                for (x, y) in every_cell(&shot) {
                    if y != bar.y {
                        assert_eq!(shot.record.tab_at(x, y), None, "at {size:?} ({x},{y}) is off the tab bar but recorded as a tab");
                    }
                }
            }
        }
    }
    assert!(
        forms.contains(&(SMALL, false, false)) && forms.contains(&(WIDE, true, true)),
        "fixture: the sweep met the compact strip's bare digits and the full form's badged names; met {forms:?}"
    );
}

/// Every row drawn in the list's viewport is recorded across its whole line —
/// from the list's left edge to the width the dividers leave — by its
/// identity and its section, and over no other cell; the list is recorded over
/// every cell of its space and no other, naming the focused section's list.
/// Quantified over every section's screen at both sizes, scrolled to its top
/// and its foot, the done group open, and each heavy row armed.
///
/// @spec TUI-RENDER-025
#[test]
fn every_row_in_view_is_recorded_across_its_whole_line_by_its_identity() {
    let state = mid_game();
    let mut rows_checked = 0usize;
    for Swept { what, view, size, lines } in swept() {
        let size = *size;
        {
            let section = view.section.expect("in a section");
            let shot = shoot(&state, view, size);
            let space = list_space(&shot.layout);
            for line in lines {
                let Some(first) = line.rows.first() else { continue };
                for x in space.x..space.right() {
                    let got = shot.record.row_at(x, line.y);
                    assert!(
                        matches!(&got, Some((s, k)) if *s == section && line.rows.contains(k)),
                        "{what} at {size:?}: the cell ({x},{}) on {first:?}'s line is recorded as {got:?}",
                        line.y
                    );
                }
                rows_checked += 1;
            }
            let on_screen: Vec<&RowKey> = lines.iter().flat_map(|l| l.rows.iter()).collect();
            for (x, y) in every_cell(&shot) {
                let inside = space.x <= x && x < space.right() && space.y <= y && y < space.bottom();
                assert_eq!(
                    shot.record.list_at(x, y),
                    inside.then_some(section),
                    "{what} at {size:?}: ({x},{y}) is {} the list's space but recorded otherwise",
                    if inside { "in" } else { "outside" }
                );
                if !inside {
                    assert_eq!(shot.record.row_at(x, y), None, "{what} at {size:?}: ({x},{y}) is outside the list but recorded as a row");
                } else if let Some((_, k)) = shot.record.row_at(x, y) {
                    assert!(on_screen.contains(&&k), "{what} at {size:?}: {k:?} is recorded at ({x},{y}) but was not drawn there");
                }
            }
        }
    }
    assert!(rows_checked > 400, "fixture: too few rows swept ({rows_checked})");
}

/// While the Journal is drawn, its region is recorded, and no cell outside it;
/// the record holds the scroll at which its last line stands at the region's
/// foot — the last line drawn on the region's last line there, and one line
/// further leaving that line blank — or none where every line fits.
///
/// @spec TUI-RENDER-025
#[test]
fn the_journal_is_recorded_over_its_region_with_the_scroll_at_which_its_last_line_stands_at_the_foot() {
    let long = mid_game();
    let short = GameState::new(5);
    for size in SIZES {
        let mut view = opened(&long);
        view.overlay = Some(Overlay::Journal { anchor: 0 });
        let shot = shoot(&long, &view, size);
        let region = body_rect(&shot.layout);
        for (x, y) in every_cell(&shot) {
            let inside = region.x <= x && x < region.right() && region.y <= y && y < region.bottom();
            assert_eq!(shot.record.journal_at(x, y), inside, "at {size:?} ({x},{y}) is {} the Journal's region but recorded otherwise", if inside { "in" } else { "outside" });
        }
        let max = shot
            .record
            .journal_max_scroll()
            .unwrap_or_else(|| panic!("at {size:?} a Journal far longer than its region records no scroll"));
        let foot = |anchor: usize| {
            let mut v = view.clone();
            v.overlay = Some(Overlay::Journal { anchor });
            let s = shoot(&long, &v, size);
            text(&s.buf, region.bottom() - 1, region.x, region.right())
        };
        assert!(!foot(max).trim().is_empty(), "at {size:?} the recorded scroll {max} does not put a line at the region's foot");
        assert!(foot(max + 1).trim().is_empty(), "at {size:?} the Journal's last line stands above the foot at the recorded scroll {max}");

        let mut view = opened(&short);
        view.overlay = Some(Overlay::Journal { anchor: 0 });
        let shot = shoot(&short, &view, size);
        assert_eq!(shot.record.journal_max_scroll(), None, "at {size:?} a Journal whose every line fits records a scroll");
        let mut bare = opened(&long);
        bare.overlay = None;
        let shot = shoot(&long, &bare, size);
        assert!(
            every_cell(&shot).iter().all(|(x, y)| !shot.record.journal_at(*x, *y)),
            "at {size:?} a frame with no Journal drawn records one"
        );
    }
}

/// A frame's record names only what the frame drew: no row scrolled out of the
/// viewport or folded in the done group; no group header, done header,
/// overflow note, armed row's summary line, or blank line below the last row
/// as a row — every line of the list that reads as no row names none.
///
/// @spec TUI-RENDER-026
#[test]
fn a_line_of_the_list_that_is_not_a_row_and_a_row_not_drawn_name_no_row() {
    let state = mid_game();
    let mut kinds = BTreeSet::new();
    for Swept { what, view, size, lines } in swept() {
        let size = *size;
        {
            let section = view.section.expect("in a section");
            let shot = shoot(&state, view, size);
            let space = list_space(&shot.layout);
            let armed_line = view.armed.as_ref().and_then(|a| lines.iter().position(|l| l.rows.contains(&a.key)));
            for (i, line) in lines.iter().enumerate() {
                if !line.rows.is_empty() {
                    continue;
                }
                let kind = if line.text.trim().is_empty() {
                    "blank"
                } else if line.text.contains("more below") || line.text.contains("more above") {
                    "note"
                } else if line.text.contains("done (") {
                    "done header"
                } else if line.text.trim_start().starts_with('\u{2500}') {
                    "header"
                } else if armed_line.is_some_and(|a| i > a) {
                    "summary"
                } else {
                    panic!("{what} at {size:?}: line {} reads as no row and no known line: {:?}", line.y, line.text)
                };
                kinds.insert(kind);
                for x in space.x..space.right() {
                    assert_eq!(
                        shot.record.row_at(x, line.y),
                        None,
                        "{what} at {size:?}: the {kind} line {:?} is recorded as a row at column {x}",
                        line.text.trim()
                    );
                }
            }
            let drawn: Vec<RowKey> = lines.iter().flat_map(|l| l.rows.clone()).collect();
            for row in rows(&state, section).into_iter().filter(|r| r.selectable() && !drawn.contains(&r.key)) {
                assert!(
                    every_cell(&shot).iter().all(|(x, y)| shot.record.row_at(*x, *y).map(|(_, k)| k) != Some(row.key.clone())),
                    "{what} at {size:?}: {:?} was not drawn but the record names it",
                    row.key
                );
            }
        }
    }
    for kind in ["blank", "note", "done header", "header", "summary"] {
        assert!(kinds.contains(kind), "fixture: the sweep met no {kind} line; met {kinds:?}");
    }
}

/// Where the narrow-mode swap, the palette, the Journal, or help holds the
/// list's space, the record names no row and no list — the tab bar beside an
/// overlay still recorded; a frame that draws the too-small card, and the
/// waking screen, record no target at all.
///
/// @spec TUI-RENDER-026
#[test]
fn where_something_else_holds_the_lists_space_no_row_or_list_is_recorded_and_the_card_and_waking_screen_record_nothing() {
    let state = mid_game();
    let base = standing_in(&state, Section::Clowder);
    let mut holders: Vec<(&str, ViewState, (u16, u16))> = Vec::new();
    let mut swap = base.clone();
    swap.inspector_wanted = true;
    holders.push(("the narrow-mode swap", swap, SMALL));
    for size in SIZES {
        for (what, overlay) in [
            ("the palette", Overlay::Palette { query: "kit".into(), selected: 0 }),
            ("the Journal", Overlay::Journal { anchor: 0 }),
            ("help", Overlay::Help),
        ] {
            let mut v = base.clone();
            v.overlay = Some(overlay);
            holders.push((what, v, size));
        }
    }
    for (what, view, size) in holders {
        let shot = shoot(&state, &view, size);
        assert!(
            matches!(shot.layout.body, Body::Narrow { .. } | Body::Wide { .. }),
            "fixture: {what} at {size:?} draws a body"
        );
        for (x, y) in every_cell(&shot) {
            assert_eq!(shot.record.row_at(x, y), None, "with {what} at {size:?}, ({x},{y}) is recorded as a row");
            assert_eq!(shot.record.list_at(x, y), None, "with {what} at {size:?}, ({x},{y}) is recorded as the list");
        }
        let labels = tab_labels(&shot);
        assert!(!labels.is_empty(), "fixture: the tab bar is drawn beside {what}");
        for (section, from, _) in labels {
            assert_eq!(shot.record.tab_at(from, shot.layout.tabs.y), Some(section), "with {what} at {size:?} the tab bar beside it is not recorded");
        }
    }

    for size in [(79, 30), (100, 23), (40, 10)] {
        let shot = shoot(&state, &base, size);
        assert!(matches!(shot.layout.body, Body::TooSmall(_)), "fixture: {size:?} draws the too-small card");
        assert!(shot.record.is_empty(), "the too-small card at {size:?} records a target");
        for (x, y) in every_cell(&shot) {
            assert!(
                shot.record.tab_at(x, y).is_none()
                    && shot.record.row_at(x, y).is_none()
                    && shot.record.list_at(x, y).is_none()
                    && !shot.record.journal_at(x, y),
                "the too-small card at {size:?} records a target at ({x},{y})"
            );
        }
    }
    for size in [SMALL, WIDE, (60, 20)] {
        let mut term = terminal(size);
        let mut record = None;
        term.draw(|f| record = Some(render::waking_recorded(f, 1_000, 400))).expect("TestBackend is infallible");
        let record = record.expect("the waking screen was drawn");
        assert!(record.is_empty(), "the waking screen at {size:?} records a target");
        for (x, y) in cells_of(Rect::new(0, 0, size.0, size.1)) {
            assert!(
                record.tab_at(x, y).is_none() && record.row_at(x, y).is_none() && record.list_at(x, y).is_none() && !record.journal_at(x, y),
                "the waking screen at {size:?} records a target at ({x},{y})"
            );
        }
    }
}

/// Drawing consults no record of an earlier frame: a frame drawn after others
/// is the frame — cells and record alike — that its inputs draw cold, so the
/// frame remains the pure function TUI-RENDER-001 makes it.
///
/// @spec TUI-RENDER-026
#[test]
fn a_frame_and_its_record_depend_on_nothing_drawn_before_it() {
    let state = mid_game();
    for size in SIZES {
        let a = standing_in(&state, Section::Meadow);
        let b = deep(&state, &standing_in(&state, Section::Clowder));
        let cold = shoot(&state, &b, size);
        let mut term = terminal(size);
        let first = shoot_on(&mut term, &state, &a, None);
        let mut journal = a.clone();
        journal.overlay = Some(Overlay::Journal { anchor: 3 });
        let second = shoot_on(&mut term, &state, &journal, Some(&first.layout));
        let after = shoot_on(&mut term, &state, &b, Some(&second.layout));
        assert_eq!(after.buf, cold.buf, "at {size:?} a frame drawn after others drew different cells from the same inputs drawn cold");
        assert_eq!(after.record, cold.record, "at {size:?} a frame drawn after others recorded differently from the same inputs drawn cold");
        let again = shoot(&state, &b, size);
        assert_eq!(again.record, cold.record, "at {size:?} the same inputs recorded two different records");
    }
}

// ===========================================================================
// What a pointer event means — TUI-NAV-125..133, TUI-NAV-076
// ===========================================================================

/// Only a left press and a vertical wheel step with no modifier held answer.
/// A release, a drag, motion alone, the right and middle buttons, a sideways
/// wheel step, and every press or step with Shift, Ctrl, or Alt held do
/// nothing at all — over a row, a tab, and the list, at both sizes — where the
/// unmodified left press and wheel step at the same cells do move the cursor
/// or the section.
///
/// @spec TUI-NAV-125
#[test]
fn only_an_unmodified_left_press_or_vertical_wheel_step_answers() {
    use MouseButton::{Left, Middle, Right};
    use MouseEventKind::{Down, Drag, Moved, ScrollDown, ScrollLeft, ScrollRight, ScrollUp, Up};
    let state = mid_game();
    let mods = [
        KeyModifiers::SHIFT,
        KeyModifiers::CONTROL,
        KeyModifiers::ALT,
        KeyModifiers::META,
        KeyModifiers::SUPER,
        KeyModifiers::SHIFT | KeyModifiers::CONTROL,
        KeyModifiers::CONTROL | KeyModifiers::ALT,
    ];
    for size in SIZES {
        let view = with_banner(&standing_in(&state, Section::Clowder));
        let shot = shoot(&state, &view, size);
        let lines = list_map(&state, &view, &shot);
        let cursor = view.focus[Section::Clowder.index()].clone();
        let target = lines
            .iter()
            .find(|l| !l.rows.is_empty() && !l.rows.contains(cursor.as_ref().expect("a row is focused")))
            .expect("fixture: another row is drawn");
        let space = list_space(&shot.layout);
        let (tab, tab_x, _) = *tab_labels(&shot).iter().find(|(s, _, _)| *s != Section::Clowder).expect("fixture: another tab");
        let spots = [
            ("a row", space.x + 3, target.y),
            ("a tab", tab_x, shot.layout.tabs.y),
            ("the list", space.x + space.width / 2, target.y),
        ];
        for (what, x, y) in spots {
            let mut silent: Vec<MouseEvent> = Vec::new();
            for kind in [
                Up(Left), Up(Right), Up(Middle), Down(Right), Down(Middle), Drag(Left), Drag(Right), Drag(Middle),
                Moved, ScrollLeft, ScrollRight,
            ] {
                silent.push(mouse(kind, x, y, KeyModifiers::NONE));
                for m in mods {
                    silent.push(mouse(kind, x, y, m));
                }
            }
            for kind in [Down(Left), ScrollDown, ScrollUp] {
                for m in mods {
                    silent.push(mouse(kind, x, y, m));
                }
            }
            for e in silent {
                assert_nothing(e, &view, &state, &shot, &format!("over {what} at {size:?}"));
            }
        }

        let mut v = view.clone();
        point(click(space.x + 3, target.y), &mut v, &state, &shot);
        assert_ne!(v.focus, view.focus, "fixture: at {size:?} an unmodified left press on a row moves the cursor");
        let mut v = view.clone();
        point(click(tab_x, shot.layout.tabs.y), &mut v, &state, &shot);
        assert_eq!(v.section, Some(tab), "fixture: at {size:?} an unmodified left press on a tab moves focus");
        let mut v = view.clone();
        point(wheel_down(space.x + space.width / 2, target.y), &mut v, &state, &shot);
        assert_ne!(v.focus, view.focus, "fixture: at {size:?} an unmodified wheel step over the list moves the cursor");
    }
}

/// A click resolves against the frame the player was looking at: where keys
/// earlier in the same batch have moved the cursor far enough that the next
/// frame would scroll the list, a click lands on the row the last frame drew
/// under it — never on what a layout worked out again would put there.
///
/// @spec TUI-NAV-126
#[test]
fn a_click_after_keys_in_the_same_batch_lands_on_the_row_the_last_frame_drew_under_it() {
    let state = mid_game();
    for size in SIZES {
        for section in [Section::Meadow, Section::Clowder] {
            let view = standing_in(&state, section);
            let shot = shoot(&state, &view, size);
            let lines = list_map(&state, &view, &shot);
            let mut after_keys = view.clone();
            for _ in 0..(shot.size.1 as usize) {
                update(Action::CursorDown, &mut after_keys, &state);
            }
            let redrawn = shoot(&state, &after_keys, size);
            let aimed = lines
                .iter()
                .find(|l| !l.rows.is_empty() && !l.rows.contains(&after_keys.focus[section.index()].clone().expect("focused")))
                .expect("fixture: a row is drawn");
            assert_ne!(
                uncursored(&text(&redrawn.buf, aimed.y, list_space(&redrawn.layout).x, list_space(&redrawn.layout).right())),
                aimed.text,
                "fixture: on the {} screen at {size:?} the keys scrolled the list, so a layout worked out again puts another row there",
                section.title()
            );
            let mut v = after_keys.clone();
            let out = point(click(list_space(&shot.layout).x + 2, aimed.y), &mut v, &state, &shot);
            assert_eq!(out, Outcome::Handled);
            let landed = v.focus[section.index()].clone().expect("the cursor rests on a row");
            assert!(
                aimed.rows.contains(&landed),
                "on the {} screen at {size:?} a click after keys in the same batch landed on {landed:?}, not the row {:?} the last frame drew under it",
                section.title(),
                aimed.rows
            );
        }
    }
}

/// A click resolves against the layout the last frame drew, dead-band and
/// all: at a width inside the dead-band below the pane breakpoint, a frame
/// that followed a three-pane frame is still three panes, so a click where a
/// layout worked out cold would put a list row — in the inspector the frame
/// drew there — does nothing, and a click on a row the frame drew lands on it.
///
/// @spec TUI-NAV-126
#[test]
fn a_click_resolves_against_the_layout_the_last_frame_drew_inside_the_dead_band() {
    let state = mid_game();
    let view = standing_in(&state, Section::Clowder);
    let mut term = terminal((130, 40));
    let wide = shoot_on(&mut term, &state, &view, None);
    let mut term = terminal((97, 40));
    let held = shoot_on(&mut term, &state, &view, Some(&wide.layout));
    let Body::Wide { inspector, .. } = held.layout.body else {
        panic!("fixture: at 97 columns a frame following three panes keeps three panes (TUI-NAV-047)")
    };
    let cold = shoot(&state, &view, (97, 40));
    assert!(matches!(cold.layout.body, Body::Narrow { .. }), "fixture: at 97 columns a cold frame is narrow");
    let cold_lines = list_map(&state, &view, &cold);
    let cold_space = list_space(&cold.layout);
    let under_inspector = cold_lines
        .iter()
        .find(|l| !l.rows.is_empty() && !l.rows.contains(view.focus[Section::Clowder.index()].as_ref().expect("focused")))
        .expect("fixture: a row is drawn cold");
    let x = inspector.x + 1;
    assert!(cold_space.x <= x && x < cold_space.right(), "fixture: the inspector column lies in a cold frame's list");
    for e in answering(x, under_inspector.y) {
        assert_nothing(e, &view, &state, &held, "over the inspector a dead-band frame drew");
    }
    let lines = list_map(&state, &view, &held);
    let aimed = lines
        .iter()
        .find(|l| !l.rows.is_empty() && !l.rows.contains(view.focus[Section::Clowder.index()].as_ref().expect("focused")))
        .expect("fixture: a row is drawn");
    let mut v = view.clone();
    point(click(list_space(&held.layout).x, aimed.y), &mut v, &state, &held);
    assert!(
        aimed.rows.contains(v.focus[Section::Clowder.index()].as_ref().expect("focused")),
        "a click on a row a dead-band frame drew did not land on it"
    );
}

/// A record holds only for the size it was drawn at: every pointer event that
/// arrives after a resize no frame has yet been drawn for does nothing, over
/// every tab, row, and the list alike.
///
/// @spec TUI-NAV-126
#[test]
fn a_pointer_event_after_a_resize_no_frame_was_drawn_for_does_nothing() {
    let state = mid_game();
    for (drawn_at, now) in [(WIDE, SMALL), (SMALL, WIDE), (WIDE, (130, 41)), (WIDE, (131, 40))] {
        let view = standing_in(&state, Section::Clowder);
        let shot = shoot(&state, &view, drawn_at);
        let mut probes: Vec<(u16, u16)> = tab_labels(&shot).iter().map(|(_, x, _)| (*x, shot.layout.tabs.y)).collect();
        let space = list_space(&shot.layout);
        probes.extend((space.y..space.bottom()).map(|y| (space.x + 1, y)));
        for (x, y) in probes {
            for e in answering(x, y) {
                let mut v = view.clone();
                let out = nav::pointer_event(e, &mut v, &state, &shot.record, now);
                assert_eq!(out, Outcome::Handled);
                assert_eq!(whole(&v), whole(&view), "{e:?} drawn at {drawn_at:?} but arriving at {now:?} changed the view");
            }
        }
    }
}

/// A pointer event at cells where the record names no target does nothing:
/// every cell of the frame that holds no tab, row, list or Journal, swept
/// with a left press and a wheel step either way, in every section at both
/// sizes, the banner showing.
///
/// @spec TUI-NAV-126
#[test]
fn a_pointer_event_where_the_record_names_no_target_does_nothing() {
    let state = mid_game();
    let mut swept = 0usize;
    for size in SIZES {
        for section in tab_sections(&state, Some(Section::Meadow)) {
            let view = with_banner(&standing_in(&state, section));
            let shot = shoot(&state, &view, size);
            for (x, y) in every_cell(&shot) {
                let r = &shot.record;
                if r.tab_at(x, y).is_some() || r.row_at(x, y).is_some() || r.list_at(x, y).is_some() || r.journal_at(x, y) {
                    continue;
                }
                for e in answering(x, y) {
                    assert_nothing(e, &view, &state, &shot, &format!("on the {} screen at {size:?}", section.title()));
                }
                swept += 1;
            }
        }
    }
    assert!(swept > 10_000, "fixture: too few untargeted cells swept ({swept})");
}

/// The views the never-acts sweep stands in, each a context a click could
/// find the player in: every section, bare and with the banner, a footer note
/// and the cursor deep; the done group open; each heavy row armed; the
/// narrow-mode swap; a count prompt; the palette; the Journal; help.
fn every_context(state: &GameState) -> Vec<(String, ViewState)> {
    let mut out = Vec::new();
    for section in tab_sections(state, Some(Section::Meadow)) {
        let v = standing_in(state, section);
        out.push((format!("{} bare", section.title()), v.clone()));
        out.push((format!("{} with banner, footer, cursor deep", section.title()), with_footer(&with_banner(&deep(state, &v)))));
    }
    let mut open = with_banner(&standing_in(state, Section::Ideas));
    open.done_open = true;
    out.push(("Ideas with the done group open".into(), open));
    for (section, row) in heavy_rows(state) {
        let v = with_banner(&standing_in(state, section));
        out.push((format!("{} with {:?} armed", section.title(), row.key), armed_on(&v, section, &row)));
    }
    let mut swap = with_banner(&standing_in(state, Section::Clowder));
    swap.inspector_wanted = true;
    out.push(("Clowder with the inspector called".into(), swap));
    let mut prompt = with_banner(&standing_in(state, Section::Clowder));
    prompt.prompt = Some(CountPrompt { digits: "3".into(), row: prompt.focus[Section::Clowder.index()].clone() });
    out.push(("Clowder with a count prompt open".into(), prompt));
    for overlay in [
        Overlay::Palette { query: "e".into(), selected: 0 },
        Overlay::Journal { anchor: 2 },
        Overlay::Help,
    ] {
        let mut v = with_footer(&with_banner(&standing_in(state, Section::Meadow)));
        v.overlay = Some(overlay.clone());
        out.push((format!("Meadow under {overlay:?}"), v));
    }
    out
}

/// No pointer event ever asks the core for anything, in any context, and none
/// touches a field, an overlay, an arming, the footer, the banner, the
/// inspector call or the done group: it changes nothing but the section, the
/// cursor (an arming ending with a move), and the Journal's scroll. Swept over
/// every context a click could find the player in, at both sizes, over every
/// tab label, every line of the list, and a grid of every other cell.
///
/// @spec TUI-NAV-127
#[test]
fn no_pointer_event_asks_the_core_for_anything_or_touches_anything_but_the_cursor() {
    let state = mid_game();
    let mut moved = 0usize;
    let mut calls = 0usize;
    for size in SIZES {
        for (what, view) in every_context(&state) {
            let shot = shoot(&state, &view, size);
            let mut probes: Vec<(u16, u16)> = Vec::new();
            for (_, from, to) in tab_labels(&shot) {
                probes.extend((from..to).map(|x| (x, shot.layout.tabs.y)));
            }
            if let Body::Narrow { main, .. } | Body::Wide { list: main, .. } = shot.layout.body {
                for y in main.y..main.bottom() {
                    probes.push((main.x + 2, y));
                    probes.push((main.x + main.width / 2, y));
                }
            }
            probes.extend(every_cell(&shot).into_iter().filter(|(x, y)| x % 7 == 3 && y % 3 == 1));
            for (x, y) in probes {
                for e in answering(x, y) {
                    let mut v = view.clone();
                    let out = point(e, &mut v, &state, &shot);
                    calls += 1;
                    assert_eq!(out, Outcome::Handled, "{what} at {size:?}: {e:?} asked the core for {out:?}");
                    assert_eq!(
                        untouchable(&v),
                        untouchable(&view),
                        "{what} at {size:?}: {e:?} changed something besides the cursor, the section and the Journal's scroll"
                    );
                    let moved_now = v.section != view.section || v.focus != view.focus;
                    if moved_now {
                        moved += 1;
                    } else {
                        assert_eq!(v.seen, view.seen, "{what} at {size:?}: {e:?} moved nothing but recorded a visit");
                        assert_eq!(v.armed, view.armed, "{what} at {size:?}: {e:?} moved nothing but changed the arming");
                    }
                    assert!(
                        v.armed.is_none() || v.armed == view.armed,
                        "{what} at {size:?}: {e:?} armed a row"
                    );
                    assert_eq!(v.footer, view.footer, "{what} at {size:?}: {e:?} set or cleared a footer note");
                    assert_eq!(v.reports, view.reports, "{what} at {size:?}: {e:?} retired or read a report");
                }
            }
        }
    }
    assert!(moved > 500 && calls > 40_000, "fixture: the sweep moved the cursor {moved} times in {calls} events");
}

/// A left press on a tab's label — on any cell of it, in the full form and the
/// compact strip — goes to that tab's section exactly as its digit would: a
/// visit, the cursor landing on the remembered row or by the landing rule; but
/// it leaves the report banner showing. A press on the focused section's own
/// tab does nothing at all.
///
/// @spec TUI-NAV-128
#[test]
fn a_left_press_on_a_tab_label_goes_to_its_section_as_its_digit_would() {
    let state = mid_game();
    let mut went = 0usize;
    for size in SIZES {
        for badged in [true, false] {
            for focus in [Section::Meadow, Section::Ideas] {
                let mut view = with_banner(&standing_in(&state, focus));
                if badged {
                    badge_two(&mut view);
                }
                let mut remembered = view.clone();
                remembered.focus[Section::Clowder.index()] =
                    drawn_rows(&state, Section::Clowder, &view).into_iter().filter(|r| r.selectable()).map(|r| r.key).nth(5);
                for view in [view, remembered] {
                    let shot = shoot(&state, &view, size);
                    for (section, from, to) in tab_labels(&shot) {
                        for x in from..to {
                            let e = click(x, shot.layout.tabs.y);
                            if section == focus {
                                assert_nothing(e, &view, &state, &shot, "the focused section's own tab");
                                continue;
                            }
                            let mut v = view.clone();
                            assert_eq!(point(e, &mut v, &state, &shot), Outcome::Handled);
                            let mut by_digit = view.clone();
                            update(Action::SectionJump(section), &mut by_digit, &state);
                            by_digit.reports = view.reports.clone();
                            assert_eq!(
                                settled(&v),
                                settled(&by_digit),
                                "at {size:?} a press at column {x} on the {} tab did not go there as its digit does (banner aside)",
                                section.title()
                            );
                            assert!(v.seen[section.index()].visited, "a press on the {} tab was no visit", section.title());
                            assert_eq!(v.reports, view.reports, "a press on the {} tab retired the banner", section.title());
                            went += 1;
                        }
                    }
                }
            }
        }
    }
    assert!(went > 500, "fixture: too few tab presses ({went})");
}

/// With the narrow-mode swap in, a press on a tab moves focus there and the
/// swap stays in, showing the row the cursor landed on.
///
/// @spec TUI-NAV-128
#[test]
fn a_tab_press_under_the_narrow_swap_keeps_the_swap_in() {
    let state = mid_game();
    let mut view = standing_in(&state, Section::Meadow);
    view.inspector_wanted = true;
    let shot = shoot(&state, &view, SMALL);
    assert!(matches!(shot.layout.body, Body::Narrow { .. }), "fixture: the floor is narrow");
    for (section, from, _) in tab_labels(&shot).into_iter().filter(|(s, _, _)| *s != Section::Meadow) {
        let mut v = view.clone();
        point(click(from, shot.layout.tabs.y), &mut v, &state, &shot);
        assert_eq!(v.section, Some(section), "a press on the {} tab under the swap did not go there", section.title());
        assert!(v.inspector_wanted, "a press on the {} tab took the swap out", section.title());
        let after = shoot(&state, &v, SMALL);
        assert!(
            every_cell(&after).iter().all(|(x, y)| after.record.list_at(*x, *y).is_none()),
            "after a press on the {} tab the list is drawn in the swap's place",
            section.title()
        );
    }
}

/// A press on the label of a tab whose section has lost its last row since the
/// frame was drawn does nothing: the frame showed a tab the bar no longer has.
///
/// @spec TUI-NAV-128
#[test]
fn a_press_on_a_tab_whose_section_has_since_lost_its_tab_does_nothing() {
    let now = sky_emptied();
    let mut drawn = now.clone();
    drawn.sites_reached = BTreeSet::new();
    assert!(lists_selectable(&drawn, Section::Sky), "fixture: the Sky has a tab when the frame is drawn");
    assert!(!lists_selectable(&now, Section::Sky), "fixture: the Sky has since lost its last row");
    for size in SIZES {
        let view = with_banner(&standing_in(&drawn, Section::Meadow));
        let shot = shoot(&drawn, &view, size);
        let (_, from, to) = tab_labels(&shot).into_iter().find(|(s, _, _)| *s == Section::Sky).expect("fixture: the Sky's tab is drawn");
        for x in from..to {
            assert_nothing(click(x, shot.layout.tabs.y), &view, &now, &shot, "the Sky's tab after its section emptied");
        }
    }
}

/// A left press anywhere along a row's line — the cursor's column, the marker,
/// the name and whatever follows it (a label, the elder hint, the Elder's
/// mark), the fields — puts the cursor on that row exactly as moving there
/// with the keys would, by identity, and does nothing else: the banner stays,
/// and an arming the cursor left ends. Quantified over every row of every
/// section's list at both sizes, scrolled to the top and the foot — content
/// rows, heavy rows, count rows, kittens, stores rows, the open done group's
/// rows — and checked as the player would: the next frame draws the cursor on
/// the very line that was under the click.
///
/// @spec TUI-NAV-129
#[test]
fn a_left_press_on_a_rows_line_puts_the_cursor_on_it_and_does_nothing_else() {
    let state = mid_game();
    let mut kinds = BTreeSet::new();
    for Swept { what, view, size, lines } in swept() {
        let size = *size;
        {
            let section = view.section.expect("in a section");
            let shot = shoot(&state, view, size);
            let space = list_space(&shot.layout);
            let cursor = view.focus[section.index()].clone();
            for line in lines {
                if line.rows.is_empty() || cursor.as_ref().is_some_and(|c| line.rows.contains(c)) {
                    continue;
                }
                let mut landed: Option<RowKey> = None;
                for x in space.x..space.right() {
                    let mut v = view.clone();
                    let out = point(click(x, line.y), &mut v, &state, &shot);
                    assert_eq!(out, Outcome::Handled, "{what} at {size:?}: a press on a row asked the core for {out:?}");
                    let key = v.focus[section.index()].clone().expect("the cursor rests on a row");
                    assert!(
                        line.rows.contains(&key),
                        "{what} at {size:?}: a press at ({x},{}) on {:?}'s line put the cursor on {key:?}",
                        line.y,
                        line.rows
                    );
                    assert_eq!(
                        settled(&v),
                        settled(&moved_to(view, section, &key)),
                        "{what} at {size:?}: a press on {key:?} did something besides putting the cursor there"
                    );
                    if let Some(first) = &landed {
                        assert_eq!(&key, first, "{what} at {size:?}: presses along one line landed on two rows");
                    } else {
                        let mut seen = v.clone();
                        seen.last_repeat = None;
                        let next = shoot(&state, &seen, size);
                        let (_, now) = cursor_line(&next).expect("the next frame draws the cursor");
                        assert_eq!(now, line.text, "{what} at {size:?}: after a press on {key:?} the cursor stands on another line's text");
                        landed = Some(key.clone());
                    }
                }
                let key = landed.expect("the line was pressed");
                let row = rows(&state, section).into_iter().find(|r| r.key == key).expect("listed");
                kinds.insert(match (&row.key, &row.kind) {
                    (RowKey::Kitten(..), _) if line.text.contains("(n: Elder)") => "kitten with hint",
                    (RowKey::Kitten(..), _) => "kitten",
                    (RowKey::Resource(_), _) => "stores",
                    (_, RowKind::Count { .. }) => "count",
                    _ if row.heavy => "heavy",
                    _ if section == Section::Ideas && finished(&state, &row) => "done",
                    (RowKey::Content(..), RowKind::Act { .. }) => "content",
                    _ => "other",
                });
            }
        }
    }
    for kind in ["kitten with hint", "kitten", "stores", "count", "heavy", "done", "content"] {
        assert!(kinds.contains(kind), "fixture: no {kind} row was pressed; pressed {kinds:?}");
    }
}

/// A press on the row the cursor already rests on does nothing — on every
/// cell of its line, every kind of row, an armed row among them — so a
/// double click is a move and then nothing, never an act.
///
/// @spec TUI-NAV-129
#[test]
fn a_press_on_the_cursors_own_row_does_nothing_and_a_double_click_never_acts() {
    let state = mid_game();
    let mut pressed = 0usize;
    for Swept { what, view, size, lines } in swept() {
        let size = *size;
        {
            let section = view.section.expect("in a section");
            let shot = shoot(&state, view, size);
            let space = list_space(&shot.layout);
            let cursor = view.focus[section.index()].clone().expect("a row is focused");
            let own = lines.iter().find(|l| l.rows.contains(&cursor)).expect("the cursor's row is drawn");
            for x in space.x..space.right() {
                assert_nothing(click(x, own.y), view, &state, &shot, &format!("{what} at {size:?}, the cursor's own row"));
                pressed += 1;
            }
            if let Some(other) = lines.iter().find(|l| !l.rows.is_empty() && !l.rows.contains(&cursor)) {
                let mut v = view.clone();
                point(click(space.x + 4, other.y), &mut v, &state, &shot);
                let after_one = v.clone();
                let out = point(click(space.x + 4, other.y), &mut v, &state, &shot);
                assert_eq!(out, Outcome::Handled, "{what} at {size:?}: a double click asked the core for {out:?}");
                assert_eq!(whole(&v), whole(&after_one), "{what} at {size:?}: the second press of a double click did something");
            }
        }
    }
    assert!(pressed > 1_000, "fixture: too few presses on the cursor's own row ({pressed})");
}

/// A press on a row the frame drew that is no longer drawn does nothing: a row
/// of a section a key earlier in the batch has left, a row folded away since
/// by `d`, and a row that has left its list.
///
/// @spec TUI-NAV-129
#[test]
fn a_press_on_a_row_no_longer_drawn_does_nothing() {
    let state = mid_game();
    for size in SIZES {
        // A section left by a key earlier in the batch.
        let view = standing_in(&state, Section::Meadow);
        let shot = shoot(&state, &view, size);
        let mut left = view.clone();
        update(Action::SectionJump(Section::Clowder), &mut left, &state);
        for line in list_map(&state, &view, &shot).iter().filter(|l| !l.rows.is_empty()) {
            assert_nothing(click(list_space(&shot.layout).x + 2, line.y), &left, &state, &shot, "a row of a section since left");
        }

        // A row folded away since.
        let mut open = standing_in(&state, Section::Ideas);
        open.done_open = true;
        let open = deep(&state, &open);
        let shot = shoot(&state, &open, size);
        let mut folded = open.clone();
        folded.done_open = false;
        folded.focus[Section::Ideas.index()] =
            drawn_rows(&state, Section::Ideas, &folded).into_iter().find(|r| r.selectable()).map(|r| r.key);
        let mut met = 0;
        for line in list_map(&state, &open, &shot) {
            let Some(key) = line.rows.first() else { continue };
            let row = rows(&state, Section::Ideas).into_iter().find(|r| &r.key == key).expect("listed");
            if !finished(&state, &row) {
                continue;
            }
            assert_nothing(click(list_space(&shot.layout).x + 2, line.y), &folded, &state, &shot, "a row folded away since");
            met += 1;
        }
        assert!(met > 0, "fixture: at {size:?} a finished row is drawn in the open done group");

        // A row that has left its list.
        let view = standing_in(&state, Section::Meadow);
        let shot = shoot(&state, &view, size);
        let lines = list_map(&state, &view, &shot);
        let gone = lines
            .iter()
            .find(|l| l.rows.iter().any(|k| matches!(k, RowKey::Content(ContentId::Structure(_), _))) && !l.rows.contains(view.focus[0].as_ref().expect("focused")))
            .expect("fixture: a structure's row is drawn");
        let Some(RowKey::Content(id, _)) = gone.rows.first().cloned() else { unreachable!() };
        let mut now = state.clone();
        now.revealed.remove(&id);
        if let ContentId::Structure(b) = id {
            now.structures[b.ix()] = 0;
        }
        assert!(!rows(&now, Section::Meadow).iter().any(|r| gone.rows.contains(&r.key)), "fixture: the row has left its list");
        assert_nothing(click(list_space(&shot.layout).x + 2, gone.y), &view, &now, &shot, "a row that has left its list");
    }
}

/// Each wheel step over the focused section's list — on a row, a header, a
/// note or a blank line alike — moves the cursor one row exactly as `↓` and
/// `↑` do, passing over headers and folded rows, the banner aside; the next
/// frame keeps the cursor in view; and a step at either end, where the cursor
/// has nowhere to go, does nothing at all. Walked the whole length of long
/// lists both ways at both sizes.
///
/// @spec TUI-NAV-130
#[test]
fn each_wheel_step_over_the_list_moves_the_cursor_one_row_as_the_arrows_do() {
    let state = mid_game();
    let mut stepped = 0usize;
    for size in SIZES {
        for section in [Section::Meadow, Section::Clowder, Section::Ideas] {
            let mut view = with_banner(&standing_in(&state, section));
            let first = drawn_rows(&state, section, &view).into_iter().find(|r| r.selectable()).map(|r| r.key);
            view.focus[section.index()] = first;
            let total = drawn_rows(&state, section, &view).iter().filter(|r| r.selectable()).count();
            for (down, steps) in [(true, total + 2), (false, total + 2)] {
                for i in 0..steps {
                    let shot = shoot(&state, &view, size);
                    let space = list_space(&shot.layout);
                    let (x, y) = (space.x + (i as u16 * 7) % space.width, space.y + (i as u16 * 5) % space.height);
                    let e = if down { wheel_down(x, y) } else { wheel_up(x, y) };
                    let mut by_key = view.clone();
                    update(if down { Action::CursorDown } else { Action::CursorUp }, &mut by_key, &state);
                    by_key.reports = view.reports.clone();
                    let mut v = view.clone();
                    assert_eq!(point(e, &mut v, &state, &shot), Outcome::Handled);
                    if by_key.focus == view.focus {
                        assert_eq!(whole(&v), whole(&view), "on the {} screen at {size:?} a step at the end of the list did something", section.title());
                    } else {
                        assert_eq!(
                            settled(&v),
                            settled(&by_key),
                            "on the {} screen at {size:?} a wheel step {} did not move the cursor as the arrow does",
                            section.title(),
                            if down { "down" } else { "up" }
                        );
                        stepped += 1;
                    }
                    view = v;
                    view.last_repeat = None;
                    view.field_guard = false;
                    assert!(cursor_line(&shoot(&state, &view, size)).is_some(), "the view did not follow the cursor at {size:?}");
                }
            }
        }
    }
    assert!(stepped > 200, "fixture: too few wheel steps moved the cursor ({stepped})");
}

/// Each step the terminal reports is one row: several steps delivered in one
/// batch, against one frame's record, move the cursor that many rows.
///
/// @spec TUI-NAV-130
#[test]
fn several_reported_steps_move_the_cursor_as_many_rows() {
    let state = mid_game();
    for size in SIZES {
        let view = standing_in(&state, Section::Clowder);
        let shot = shoot(&state, &view, size);
        let space = list_space(&shot.layout);
        for n in [2usize, 3, 5] {
            let mut v = view.clone();
            let mut by_keys = view.clone();
            for _ in 0..n {
                point(wheel_down(space.x + 1, space.y + 1), &mut v, &state, &shot);
                update(Action::CursorDown, &mut by_keys, &state);
            }
            assert_eq!(v.focus, by_keys.focus, "at {size:?} {n} reported steps did not move the cursor {n} rows");
        }
    }
}

/// A wheel step over a list where the cursor rests on no row does nothing.
///
/// @spec TUI-NAV-130
#[test]
fn a_wheel_step_over_a_list_with_no_row_to_rest_on_does_nothing() {
    let state = sky_emptied();
    for size in SIZES {
        for memory in [Some(RowKey::Site(SiteId::ALL[0])), None] {
            let mut view = ViewState { section: Some(Section::Sky), ..Default::default() };
            view.focus[Section::Sky.index()] = memory.clone();
            let shot = shoot(&state, &view, size);
            for (x, y) in cells_of(list_space(&shot.layout)) {
                for e in answering(x, y) {
                    assert_nothing(e, &view, &state, &shot, "an emptied Sky");
                }
            }
        }
    }
}

/// A press or a step over the sidebar, the header, the report banner, the
/// ticker, the hint bar, a pane divider, or the inspector — as a pane, or as
/// the narrow-mode swap in the list's place — does nothing: swept over every
/// cell of each, at both sizes.
///
/// @spec TUI-NAV-130
#[test]
fn a_press_or_step_over_the_chrome_the_dividers_or_the_inspector_does_nothing() {
    let state = mid_game();
    let mut regions = BTreeSet::new();
    for size in SIZES {
        for swap in [false, true] {
            let mut view = with_footer(&with_banner(&standing_in(&state, Section::Clowder)));
            view.inspector_wanted = swap;
            let shot = shoot(&state, &view, size);
            let l = shot.layout;
            let mut inert: Vec<(&str, Rect)> = vec![("header", l.header), ("ticker", l.ticker), ("hint bar", l.hints)];
            inert.push(("banner", l.banner.expect("fixture: the banner shows")));
            match l.body {
                Body::Narrow { sidebar, main } => {
                    inert.push(("sidebar", sidebar));
                    inert.push(("divider", Rect::new(main.x, main.y, 1, main.height)));
                    if swap {
                        inert.push(("swap", main));
                    }
                }
                Body::Wide { sidebar, list, inspector } => {
                    inert.push(("sidebar", sidebar));
                    inert.push(("divider", Rect::new(list.x, list.y, 1, list.height)));
                    inert.push(("divider", Rect::new(list.right() - 1, list.y, 1, list.height)));
                    inert.push(("inspector", inspector));
                }
                Body::TooSmall(_) => unreachable!("above the floor"),
            }
            for (what, rect) in inert {
                regions.insert(what);
                for (x, y) in cells_of(rect) {
                    for e in answering(x, y) {
                        assert_nothing(e, &view, &state, &shot, &format!("the {what} at {size:?}"));
                    }
                }
            }
        }
    }
    assert!(regions.len() == 8, "fixture: swept {regions:?}");
}

/// While a count prompt or the palette is open, every pointer event does
/// nothing — on every cell of the frame, the palette's results and the
/// prompt's row among them — so no click jumps to a result, closes, or
/// accepts the field, and the cursor and section stand where they were.
///
/// @spec TUI-NAV-131
#[test]
fn under_a_count_prompt_or_the_palette_every_pointer_event_does_nothing() {
    let state = mid_game();
    let mut views = Vec::new();
    for section in [Section::Meadow, Section::Clowder] {
        let mut v = with_banner(&standing_in(&state, section));
        v.prompt = Some(CountPrompt { digits: "12".into(), row: v.focus[section.index()].clone() });
        views.push((format!("a count prompt on the {} screen", section.title()), v));
    }
    for query in ["", "kit", "e"] {
        let mut v = with_banner(&standing_in(&state, Section::Meadow));
        v.overlay = Some(Overlay::Palette { query: query.into(), selected: 0 });
        views.push((format!("the palette with {query:?}"), v));
    }
    for size in SIZES {
        for (what, view) in &views {
            let shot = shoot(&state, view, size);
            for (x, y) in every_cell(&shot) {
                for e in answering(x, y) {
                    assert_nothing(e, view, &state, &shot, &format!("{what} at {size:?}"));
                }
            }
        }
    }
}

/// While the Journal is open, each wheel step over it scrolls it one line —
/// down toward older entries, up toward newer — never above its top nor past
/// the scroll its record holds, a step at either bound doing nothing; and no
/// step ends the arming beneath or retires the banner.
///
/// @spec TUI-NAV-132, TUI-NAV-076
#[test]
fn each_wheel_step_over_the_journal_scrolls_it_one_line_within_its_bounds() {
    let state = mid_game();
    let (section, heavy) = heavy_rows(&state).remove(0);
    for size in SIZES {
        let mut view = armed_on(&with_banner(&standing_in(&state, section)), section, &heavy);
        view.overlay = Some(Overlay::Journal { anchor: 0 });
        let armed = view.armed.clone();
        let max = shoot(&state, &view, size).record.journal_max_scroll().expect("fixture: the Journal overflows");
        assert!(max >= 3, "fixture: the Journal scrolls a few lines");
        let anchor = |v: &ViewState| match v.overlay {
            Some(Overlay::Journal { anchor }) => anchor,
            ref other => panic!("the Journal closed under the wheel: {other:?}"),
        };
        for (down, steps) in [(true, max + 3), (false, max + 3)] {
            for i in 0..steps {
                let shot = shoot(&state, &view, size);
                let region = body_rect(&shot.layout);
                let (x, y) = (region.x + (i as u16 * 11) % region.width, region.y + (i as u16 * 3) % region.height);
                let before = anchor(&view);
                let mut v = view.clone();
                let out = point(if down { wheel_down(x, y) } else { wheel_up(x, y) }, &mut v, &state, &shot);
                assert_eq!(out, Outcome::Handled);
                let want = if down { (before + 1).min(max) } else { before.saturating_sub(1) };
                assert_eq!(anchor(&v), want, "at {size:?} a step {} over the Journal from {before} scrolled to {}", if down { "down" } else { "up" }, anchor(&v));
                if want == before {
                    assert_eq!(whole(&v), whole(&view), "at {size:?} a step at the Journal's bound did something");
                }
                let mut expect = view.clone();
                expect.overlay = Some(Overlay::Journal { anchor: want });
                assert_eq!(settled(&v), settled(&expect), "at {size:?} a step over the Journal did something besides scrolling it");
                assert_eq!(v.armed, armed, "at {size:?} a wheel step over the Journal ended the arming beneath");
                assert_eq!(v.reports, view.reports, "at {size:?} a wheel step over the Journal retired the banner");
                view = v;
            }
            assert_eq!(anchor(&view), if down { max } else { 0 }, "at {size:?} the wheel did not reach the Journal's bound");
        }
    }
}

/// While the Journal is open, ↓/j and ↑/k move it exactly as a wheel step
/// down and up does, from every scroll and at both bounds, and [ and ] take it
/// to its top and its foot; no key ends the arming beneath or retires the
/// banner. Quantified over every supported size.
///
/// @spec TUI-NAV-134
#[test]
fn each_journal_key_scrolls_it_as_the_wheel_does() {
    use clowder_tui::nav::{journal_key, Action};
    let state = mid_game();
    let (section, heavy) = heavy_rows(&state).remove(0);
    for size in SIZES {
        let mut base = armed_on(&with_banner(&standing_in(&state, section)), section, &heavy);
        base.overlay = Some(Overlay::Journal { anchor: 0 });
        let shot = shoot(&state, &base, size);
        let max = shot.record.journal_max_scroll().expect("fixture: the Journal overflows");
        let region = body_rect(&shot.layout);
        let anchor = |v: &ViewState| match v.overlay {
            Some(Overlay::Journal { anchor }) => anchor,
            ref other => panic!("the Journal closed under a key: {other:?}"),
        };
        for at in 0..=max {
            let mut view = base.clone();
            view.overlay = Some(Overlay::Journal { anchor: at });
            let shot = shoot(&state, &view, size);
            for (action, wheel) in [(Action::CursorDown, true), (Action::CursorUp, false)] {
                let mut by_key = view.clone();
                assert!(journal_key(&action, &mut by_key, Some(&shot.record)), "{action:?} was not the Journal's");
                let mut by_wheel = view.clone();
                let (x, y) = (region.x, region.y);
                point(if wheel { wheel_down(x, y) } else { wheel_up(x, y) }, &mut by_wheel, &state, &shot);
                assert_eq!(anchor(&by_key), anchor(&by_wheel), "at {size:?} from {at}, {action:?} and the wheel disagree");
                assert_eq!(by_key.armed, view.armed, "at {size:?} {action:?} in the Journal ended the arming");
                assert_eq!(by_key.reports, view.reports, "at {size:?} {action:?} in the Journal retired the banner");
            }
            for (action, want) in [(Action::GroupPrev, 0), (Action::GroupNext, max)] {
                let mut v = view.clone();
                assert!(journal_key(&action, &mut v, Some(&shot.record)));
                assert_eq!(anchor(&v), want, "at {size:?} from {at}, {action:?} went to {} rather than {want}", anchor(&v));
                assert_eq!(v.armed, view.armed);
                assert_eq!(v.reports, view.reports);
            }
        }
    }
}

/// The Journal's keys belong to it alone: with help open, or no overlay, they
/// are not taken here and the view is left as it stood.
///
/// @spec TUI-NAV-134
#[test]
fn journal_keys_are_not_taken_without_an_open_journal() {
    use clowder_tui::nav::{journal_key, Action};
    let state = mid_game();
    for overlay in [None, Some(Overlay::Help)] {
        let mut view = with_banner(&standing_in(&state, Section::Meadow));
        view.overlay = overlay.clone();
        let shot = shoot(&state, &view, (100, 30));
        for action in [Action::CursorDown, Action::CursorUp, Action::GroupPrev, Action::GroupNext] {
            let mut v = view.clone();
            assert!(!journal_key(&action, &mut v, Some(&shot.record)), "{action:?} was taken under {overlay:?}");
            assert_eq!(whole(&v), whole(&view), "{action:?} changed the view under {overlay:?}");
        }
    }
}

/// While the Journal or help is open, every press does nothing — on the
/// overlay and on the tab bar beside it alike — and so does every step over
/// anything but the Journal, and every step under help.
///
/// @spec TUI-NAV-132
#[test]
fn under_the_journal_or_help_every_press_and_every_other_step_does_nothing() {
    let state = mid_game();
    for size in SIZES {
        for overlay in [Overlay::Journal { anchor: 4 }, Overlay::Help] {
            let mut view = with_banner(&standing_in(&state, Section::Meadow));
            view.overlay = Some(overlay.clone());
            let shot = shoot(&state, &view, size);
            let region = body_rect(&shot.layout);
            for (x, y) in every_cell(&shot) {
                assert_nothing(click(x, y), &view, &state, &shot, &format!("a press under {overlay:?} at {size:?}"));
                let over = region.x <= x && x < region.right() && region.y <= y && y < region.bottom();
                if over && matches!(overlay, Overlay::Journal { .. }) {
                    continue;
                }
                assert_nothing(wheel_down(x, y), &view, &state, &shot, &format!("a step under {overlay:?} at {size:?}"));
                assert_nothing(wheel_up(x, y), &view, &state, &shot, &format!("a step under {overlay:?} at {size:?}"));
            }
        }
    }
}

/// A pointer event that moves the cursor off an armed row — a press on another
/// row, a press on another section's tab, a wheel step that moves — ends the
/// arming, as navigation does; one that moves neither cursor nor section — a
/// press on the armed row itself, on its summary lines, on a header, on the
/// sidebar, a step at the end of the list, every event that does not answer —
/// leaves it armed with its clock unchanged. Quantified over the four heavy
/// rows at both sizes.
///
/// @spec TUI-NAV-133, TUI-NAV-076
#[test]
fn a_pointer_move_ends_an_arming_and_an_event_that_moves_nothing_leaves_it() {
    let state = mid_game();
    for size in SIZES {
        for (section, row) in heavy_rows(&state) {
            let view = with_banner(&armed_on(&standing_in(&state, section), section, &row));
            let shot = shoot(&state, &view, size);
            let space = list_space(&shot.layout);
            let lines = list_map(&state, &view, &shot);
            let what = format!("{:?} armed at {size:?}", row.key);

            let other = lines.iter().find(|l| !l.rows.is_empty() && !l.rows.contains(&row.key)).expect("fixture: another row");
            let mut v = view.clone();
            point(click(space.x + 3, other.y), &mut v, &state, &shot);
            assert!(v.armed.is_none(), "{what}: a press on another row left it armed");

            let (_, tab_x, _) = *tab_labels(&shot).iter().find(|(s, _, _)| *s != section).expect("another tab");
            let mut v = view.clone();
            point(click(tab_x, shot.layout.tabs.y), &mut v, &state, &shot);
            assert!(v.armed.is_none(), "{what}: a press on another tab left it armed");

            let selectable: Vec<RowKey> =
                drawn_rows(&state, section, &view).into_iter().filter(|r| r.selectable()).map(|r| r.key).collect();
            let at = selectable.iter().position(|k| *k == row.key).expect("drawn");
            let moving = if at + 1 < selectable.len() { wheel_down(space.x, space.y) } else { wheel_up(space.x, space.y) };
            let mut v = view.clone();
            point(moving, &mut v, &state, &shot);
            assert!(v.armed.is_none() && v.focus != view.focus, "{what}: a wheel step that moved the cursor left it armed");

            let own = lines.iter().position(|l| l.rows.contains(&row.key)).expect("the armed row is drawn");
            let mut still: Vec<(u16, u16)> = (space.x..space.right()).map(|x| (x, lines[own].y)).collect();
            still.extend(lines.iter().filter(|l| l.rows.is_empty()).map(|l| (space.x + 2, l.y)));
            if let Body::Narrow { sidebar, .. } | Body::Wide { sidebar, .. } = shot.layout.body {
                still.push((sidebar.x + 2, sidebar.y + 2));
            }
            still.push((tab_labels(&shot).iter().find(|(s, _, _)| *s == section).expect("own tab").1, shot.layout.tabs.y));
            assert!(lines[own + 1].rows.is_empty(), "fixture: the armed row's summary is drawn beneath it");
            for (x, y) in still {
                for e in [
                    click(x, y),
                    mouse(MouseEventKind::Down(MouseButton::Right), x, y, KeyModifiers::NONE),
                    mouse(MouseEventKind::Moved, x, y, KeyModifiers::NONE),
                    mouse(MouseEventKind::Down(MouseButton::Left), x, y, KeyModifiers::SHIFT),
                ] {
                    assert_nothing(e, &view, &state, &shot, &format!("{what}, an event that moves nothing"));
                }
            }

            // The same row armed at the very end of its list, where a step
            // past the end moves nothing.
            for (end, step) in [(0, wheel_up(space.x, space.y)), (selectable.len() - 1, wheel_down(space.x, space.y))] {
                let mut at_end = view.clone();
                at_end.focus[section.index()] = Some(selectable[end].clone());
                if let Some(a) = at_end.armed.as_mut() {
                    a.key = selectable[end].clone();
                }
                let end_shot = shoot(&state, &at_end, size);
                assert_nothing(step, &at_end, &state, &end_shot, &format!("{what}, a step past the end of the list"));
            }
        }
    }
}

/// A distinct press acts on the row a pointer moved the cursor to; no held or
/// auto-repeating continuation of a press already down does. Held `Enter`,
/// `X`, `+`, `-` and `n`, each pressed (acting on the row it was aimed at),
/// then a click moving the cursor — onto another row or another section's
/// landing — then the key's repeats a half-second long: none of them acts on
/// the row the click moved the cursor to; the next distinct press does.
///
/// @spec TUI-NAV-133
#[test]
fn no_held_press_acts_on_the_row_a_pointer_moved_the_cursor_to() {
    let state = mid_game();
    let acts = |out: &Outcome| matches!(out, Outcome::Apply(_) | Outcome::Repeat(..));

    // (key, section, a row the key acts on, a row the click moves to)
    let content = |r: &Row| matches!(r.kind, RowKind::Act { .. }) && !r.heavy && matches!(r.key, RowKey::Content(ContentId::Structure(_), _));
    let count = |r: &Row| matches!(r.kind, RowKind::Count { .. });
    let idea = |r: &Row| matches!(r.key, RowKey::Content(ContentId::Idea(_) | ContentId::Refinement(_), _)) && !r.heavy && !finished(&state, r);
    let rule = |r: &Row| r.heavy && matches!(r.key, RowKey::Content(ContentId::Rule(_), _)) && !finished(&state, r);
    let other_kitten = |r: &Row| matches!(r.key, RowKey::Kitten(ix, _) if Some(ix) != state.elder);
    type Pick<'a> = &'a dyn Fn(&Row) -> bool;
    let cases: Vec<(&str, Action, Section, Pick, Pick)> = vec![
        ("Enter", Action::Activate, Section::Meadow, &content, &content),
        ("Enter onto a heavy row", Action::Activate, Section::Ideas, &idea, &rule),
        ("X", Action::MaxAffordable, Section::Clowder, &count, &count),
        ("+", Action::Increment, Section::Clowder, &count, &count),
        ("-", Action::Decrement, Section::Clowder, &count, &count),
        ("n", Action::AppointElder, Section::Clowder, &other_kitten, &other_kitten),
    ];
    for size in SIZES {
        for (name, action, section, first, second) in &cases {
            // The row to click, and a row to hold the key on drawn in the
            // same frame as it, the cursor on the held row.
            let base = standing_in(&state, *section);
            let all = rows(&state, *section);
            let (a, b, v, shot, b_line) = all
                .iter()
                .filter(|r| second(r))
                .flat_map(|b| all.iter().filter(|a| first(a) && a.key != b.key).map(move |a| (a, b)))
                .find_map(|(a, b)| {
                    let mut v = base.clone();
                    v.focus[section.index()] = Some(a.key.clone());
                    let shot = shoot(&state, &v, size);
                    let y = list_map(&state, &v, &shot).into_iter().find(|l| l.rows.contains(&b.key))?.y;
                    Some((a.clone(), b.clone(), v, shot, y))
                })
                .unwrap_or_else(|| panic!("fixture: at {size:?} a row to hold {name} on is drawn beside the row to click"));
            let mut v = v;

            let out = update(action.clone(), &mut v, &state);
            assert!(acts(&out), "fixture: {name} at {size:?} acts on {:?}: {out:?}", a.key);
            point(click(list_space(&shot.layout).x + 2, b_line), &mut v, &state, &shot);
            assert_eq!(v.focus[section.index()].as_ref(), Some(&b.key), "fixture: the click moved the cursor to {:?}", b.key);
            for _ in 0..16 {
                std::thread::sleep(Duration::from_millis(30));
                let out = update(action.clone(), &mut v, &state);
                assert!(!acts(&out), "at {size:?} a held {name} acted on {:?}, the row a click moved the cursor to: {out:?}", b.key);
                assert!(v.armed.is_none(), "at {size:?} a held {name} armed {:?}, the row a click moved the cursor to", b.key);
            }
            std::thread::sleep(Duration::from_millis(300));
            let out = update(action.clone(), &mut v, &state);
            let armed_b = v.armed.as_ref().is_some_and(|x| x.key == b.key);
            assert!(acts(&out) || armed_b, "at {size:?} a distinct {name} after the hold did not act on {:?}: {out:?}", b.key);
        }

        // Onto another section's landing, by its tab.
        let mut v = standing_in(&state, Section::Meadow);
        v.focus[0] = Some(find_row(&state, Section::Meadow, content).key);
        let shot = shoot(&state, &v, size);
        let out = update(Action::Activate, &mut v, &state);
        assert!(acts(&out), "fixture: Enter acts on a Meadow row");
        let (_, x, _) = *tab_labels(&shot).iter().find(|(s, _, _)| *s == Section::Clowder).expect("the Clowder tab");
        point(click(x, shot.layout.tabs.y), &mut v, &state, &shot);
        assert_eq!(v.section, Some(Section::Clowder), "fixture: the tab press went to the Clowder");
        for _ in 0..16 {
            std::thread::sleep(Duration::from_millis(30));
            let out = update(Action::Activate, &mut v, &state);
            assert!(!acts(&out) && v.armed.is_none(), "at {size:?} a held Enter acted on the row a tab press landed on: {out:?}");
        }
        std::thread::sleep(Duration::from_millis(300));
        let out = update(Action::Activate, &mut v, &state);
        assert!(acts(&out) || v.armed.is_some(), "at {size:?} a distinct Enter after the hold did not act on the landed row");
    }
}

/// The player's eye: a mid-game meadow, opened as a sitting is, driven only
/// by the pointer through the entry point the loop will use — a tab pressed,
/// the wheel run down the list until it scrolls, a row near the foot pressed,
/// the inspector called and a tab pressed beneath it — at the floor and wide.
/// After every event the cursor stands on the line that was under the click,
/// the game is the game it was, and nothing but the cursor and the section
/// has moved.
///
/// @spec TUI-NAV-128, TUI-NAV-129, TUI-NAV-130, TUI-NAV-127
#[test]
fn players_eye_a_mid_game_meadow_driven_by_the_pointer_alone() {
    let state = mid_game();
    let saved = clowder_core::save_string(&state, 0);
    for size in SIZES {
        let mut view = with_banner(&opened(&state));
        let mut term = terminal(size);
        let mut shot = shoot_on(&mut term, &state, &view, None);

        // A tab.
        let (_, x, _) = *tab_labels(&shot).iter().find(|(s, _, _)| *s == Section::Clowder).expect("the Clowder tab");
        assert_eq!(point(click(x, shot.layout.tabs.y), &mut view, &state, &shot), Outcome::Handled);
        assert_eq!(view.section, Some(Section::Clowder), "at {size:?} the tab press did not go to the Clowder");
        shot = shoot_on(&mut term, &state, &view, Some(&shot.layout));

        // The wheel, down until the list scrolls.
        let space = list_space(&shot.layout);
        let top_before = text(&shot.buf, space.y, space.x, space.right());
        let mut scrolled = false;
        for _ in 0..(space.height as usize * 2) {
            point(wheel_down(space.x + space.width / 2, space.y + space.height / 2), &mut view, &state, &shot);
            shot = shoot_on(&mut term, &state, &view, Some(&shot.layout));
            assert!(cursor_line(&shot).is_some(), "at {size:?} the wheel left the cursor off screen");
            if text(&shot.buf, space.y, space.x, space.right()) != top_before {
                scrolled = true;
            }
        }
        assert!(scrolled, "at {size:?} the wheel never scrolled the list");

        // A row near the foot, then one near the top, of the scrolled list.
        for pick_last in [true, false] {
            let lines = list_map(&state, &view, &shot);
            let cursor = view.focus[Section::Clowder.index()].clone().expect("focused");
            let mut candidates = lines.iter().filter(|l| !l.rows.is_empty() && !l.rows.contains(&cursor));
            let aimed = if pick_last { candidates.next_back() } else { candidates.next() }.expect("another row is drawn").clone();
            point(click(space.x + space.width - 2, aimed.y), &mut view, &state, &shot);
            shot = shoot_on(&mut term, &state, &view, Some(&shot.layout));
            let (_, now) = cursor_line(&shot).expect("the cursor is drawn");
            assert_eq!(now, aimed.text, "at {size:?} the cursor does not stand on the line that was under the click");
        }

        // The inspector called; then a tab pressed beneath it.
        view.inspector_wanted = true;
        shot = shoot_on(&mut term, &state, &view, Some(&shot.layout));
        let before = view.clone();
        for (x, y) in cells_of(list_space(&shot.layout)) {
            if size == SMALL {
                let mut v = view.clone();
                point(click(x, y), &mut v, &state, &shot);
                assert_eq!(whole(&v), whole(&view), "at {size:?} a click on the inspector swap did something");
            }
        }
        let (_, x, _) = *tab_labels(&shot).iter().find(|(s, _, _)| *s == Section::Ideas).expect("the Ideas tab");
        point(click(x, shot.layout.tabs.y), &mut view, &state, &shot);
        assert_eq!(view.section, Some(Section::Ideas), "at {size:?} a tab press with the inspector called did not go there");
        assert!(view.inspector_wanted, "at {size:?} a tab press took the inspector away");
        assert_eq!(view.reports, before.reports, "at {size:?} the pointer retired the banner");
        assert_eq!(clowder_core::save_string(&state, 0), saved, "at {size:?} the game changed");
    }
}
