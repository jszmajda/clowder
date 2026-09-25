//! The find-anything palette — what it finds, in what order, and where its
//! Enter lands.
//!
//! @spec TUI-NAV-038, TUI-NAV-062, TUI-NAV-063
//!
//! **The seam.** The palette's result list is a pure function of the game state
//! and the query, re-derived at every look — which is exactly what makes it
//! testable apart from any drawing, and exactly what these tests call:
//!
//! ```ignore
//! clowder_tui::nav::palette::results(state: &GameState, query: &str) -> Vec<PaletteResult>
//!
//! pub struct PaletteResult {
//!     pub section: Section, // the section the item is listed in
//!     pub key: RowKey,      // the item's first row in that section
//!     pub name: String,     // the name that item's rows display
//! }
//! ```
//!
//! The list is returned in the order the player sees it, top first — so the
//! first entry is the one the palette's Enter takes. The drawing and the Enter
//! are then tested against that list: the palette widget must draw it (every
//! result, beside its section's digit, top result first), and the reducer must
//! jump to its head. Three surfaces reading one derivation cannot disagree
//! about what the player is aiming at.
//!
//! **Where the truth comes from.** "The items listed in revealed sections" is
//! read straight off `nav::rows::rows` — the one builder both input and drawing
//! already use — and "the name its rows display" is read off a drawn list
//! frame. Nothing here restates a content table.
//!
//! **Why this is its own file.** The reducer-side field keys (typing,
//! Backspace, arrows, Esc, the held Enter) are asserted beside the other
//! reducer tests in `tests/update.rs`, and the widget's existing drawing tests
//! live in `tests/overlays.rs`. The tests here all go through the result list,
//! a seam neither of those files has.

use std::collections::BTreeSet;

use clowder_core::content::{craft_output, structure_def, MEADOW_MOVE_MIN_KITTENS};
use clowder_core::{
    AdventureId, ArchId, Availability, ColonyId, ContentId, CraftId, GameState, IdeaId, JobId,
    Kitten, KittenTrait, Location, ResourceId, RiteId, SiteId, StructureId,
};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::Terminal;

use clowder_tui::nav::palette::{results, PaletteResult};
use clowder_tui::nav::rows::{revealed_sections, rows};
use clowder_tui::nav::{update, Action, Outcome, Overlay, RowKey, Section, ViewState};
use clowder_tui::render;
use clowder_tui::render::layout::{compute, Body};
use clowder_tui::render::widgets::overlays;

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

/// The name two kittens share, so two items carry one name inside one section.
const TWIN: &str = "Twin";

/// A colony left undiscovered: an item with no row, which must never be a result.
const UNDISCOVERED: ColonyId = ColonyId::Owls;

/// Every section open and long, every kind of row present — and two name
/// collisions planted on purpose: two kittens sharing a name in one section,
/// and a kitten sharing a structure's name across sections.
fn rich() -> GameState {
    let mut s = GameState::new(7);

    for r in ResourceId::ALL {
        s.resources[r.ix()] = 1.0e12;
    }
    // A craft and the resource it crafts are named alike by design (the
    // "Wands" craft makes "Wands"). Holding every resource would plant a
    // third, *unplanned* name collision — one between every crafted good's
    // stores row and its own craft row — on top of the two this fixture
    // means to plant below, so the crafted resources are left empty; the
    // Crafts section is still exercised fully through its own rows.
    for c in CraftId::ALL {
        s.resources[craft_output(c).ix()] = 0.0;
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
    let mut roster: Vec<Kitten> = (0..MEADOW_MOVE_MIN_KITTENS + 9)
        .map(|i| kitten(&format!("kitten-{i}"), None))
        .collect();
    for k in roster.iter_mut().take(4) {
        k.job = Some(JobId::Tinkerer);
    }
    roster[0].tinkering = Some(CraftId::ALL[0]);
    roster[5].name = TWIN.to_string();
    roster[6].name = TWIN.to_string();
    roster[7].name = structure_def(StructureId::ALL[0]).name.to_string();
    s.kittens = roster;
    s.elder = Some(0);

    s.peak_devotion = 1.0e12;
    s.devotion = 1.0e6;
    s.serenity = 1.0e12;
    s.rites = [RiteId::TheReverie, RiteId::Enlightenment].into_iter().collect::<BTreeSet<_>>();
    s.arches_bought = [ArchId::EverdreamArch].into_iter().collect::<BTreeSet<_>>();

    s.discovered =
        ColonyId::ALL.iter().copied().filter(|c| *c != UNDISCOVERED).collect::<BTreeSet<_>>();
    s.sites_reached = BTreeSet::new();
    s.whale_visit_years = 5;
    s.whale_contentment = 1;

    s.generation = 1;
    s.elder_wisdom = 1.0e9;
    s.warm_memories = 1.0e3;

    assert!(revealed_sections(&s).len() >= 8, "fixture: most sections are revealed");
    s
}

/// How many structures the young meadow has revealed.
const YOUNG_REVEALED: usize = 3;

/// A young meadow: a few structures revealed and nothing in the stores, so
/// every revealed structure is Teased — listed, not yet affordable.
fn young() -> GameState {
    let mut s = GameState::new(5);
    s.revealed = StructureId::ALL
        .iter()
        .take(YOUNG_REVEALED)
        .map(|b| ContentId::Structure(*b))
        .collect::<BTreeSet<_>>();
    for b in StructureId::ALL.iter().take(YOUNG_REVEALED) {
        assert_eq!(
            s.availability(ContentId::Structure(*b)),
            Availability::Teased,
            "fixture: a revealed structure with empty stores is Teased"
        );
    }
    s
}

fn fresh() -> GameState {
    GameState::new(3)
}

fn every_state() -> Vec<(&'static str, GameState)> {
    vec![("a maximal meadow", rich()), ("a young meadow", young()), ("a new meadow", fresh())]
}

// ---------------------------------------------------------------------------
// The truth, read off the section lists
// ---------------------------------------------------------------------------

/// What a row is *about*. A key names a row; several rows can be about one
/// thing — a craft's make and tinkerer rows, a kitten's two rows, a colony's
/// two rows — and that thing is one palette result.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Item {
    Content(ContentId),
    Resource(ResourceId),
    Colony(ColonyId),
    Site(SiteId),
    Adventure(Option<AdventureId>),
    Kitten(usize),
    Fixed(&'static str),
}

fn item_of(key: &RowKey) -> Item {
    match key {
        RowKey::Content(id, _) => Item::Content(*id),
        RowKey::Resource(r) => Item::Resource(*r),
        RowKey::Colony(c, _) => Item::Colony(*c),
        RowKey::Site(s) => Item::Site(*s),
        RowKey::Adventure(a) => Item::Adventure(*a),
        RowKey::Kitten(ix, _) => Item::Kitten(*ix),
        RowKey::Fixed(tag) => Item::Fixed(tag),
    }
}

/// Every item with a selectable row in a revealed section, as (section, the
/// item's first row), in tab-bar order and then list order.
fn listed_items(state: &GameState) -> Vec<(Section, RowKey)> {
    let mut out = Vec::new();
    for section in revealed_sections(state) {
        let mut seen: Vec<Item> = Vec::new();
        for row in rows(state, section) {
            if !row.selectable() {
                continue;
            }
            let item = item_of(&row.key);
            if !seen.contains(&item) {
                seen.push(item);
                out.push((section, row.key.clone()));
            }
        }
    }
    out
}

fn placed(list: &[PaletteResult]) -> Vec<(Section, RowKey)> {
    list.iter().map(|r| (r.section, r.key.clone())).collect()
}

/// The order TUI-NAV-062 states, computed from the unfiltered list (which is
/// itself asserted to be in tab-bar then list order): names equal to the query
/// first, then names beginning with it, then names containing it elsewhere,
/// ties kept in that order. Case is ignored everywhere and spaces are kept; a
/// query of nothing but spaces is no query.
fn expected_order(all: &[PaletteResult], query: &str) -> Vec<(Section, RowKey)> {
    if query.chars().all(|c| c == ' ') {
        return placed(all);
    }
    let q = query.to_lowercase();
    let mut hits: Vec<(u8, usize)> = all
        .iter()
        .enumerate()
        .filter_map(|(i, r)| {
            let name = r.name.to_lowercase();
            if name == q {
                Some((0, i))
            } else if name.starts_with(&q) {
                Some((1, i))
            } else if name.contains(&q) {
                Some((2, i))
            } else {
                None
            }
        })
        .collect();
    hits.sort();
    hits.into_iter().map(|(_, i)| (all[i].section, all[i].key.clone())).collect()
}

/// Queries drawn from the names themselves, so every tier of the ranking is
/// exercised: whole names (as written, upper, lower), heads, middles, spans
/// across a space, a leading space, and a trailing one.
fn query_corpus(all: &[PaletteResult]) -> Vec<String> {
    let mut qs: BTreeSet<String> = BTreeSet::new();
    for r in all {
        let chars: Vec<char> = r.name.chars().collect();
        let span = |a: usize, b: usize| -> String { chars[a.min(chars.len())..b.min(chars.len())].iter().collect() };
        qs.insert(r.name.clone());
        qs.insert(r.name.to_uppercase());
        qs.insert(r.name.to_lowercase());
        qs.insert(span(0, 3));
        qs.insert(span(1, 4));
        qs.insert(format!(" {}", span(0, 3)));
        qs.insert(format!("{} ", r.name));
        if let Some(sp) = chars.iter().position(|c| *c == ' ') {
            qs.insert(span(sp.saturating_sub(2), sp + 3));
            qs.insert(span(sp.saturating_sub(2), sp + 3).to_uppercase());
        }
    }
    for q in ["a", "e", "E", "s", " s", "an", "AN", "~~~~"] {
        qs.insert(q.to_string());
    }
    qs.retain(|q| !q.is_empty());
    qs.into_iter().collect()
}

// ---------------------------------------------------------------------------
// Drawing
// ---------------------------------------------------------------------------

fn lines_of(buf: &Buffer, rect: Rect) -> Vec<String> {
    (rect.y..rect.bottom())
        .map(|y| {
            (rect.x..rect.right())
                .map(|x| buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "))
                .collect::<String>()
        })
        .collect()
}

fn palette_view(query: &str) -> ViewState {
    ViewState {
        overlay: Some(Overlay::Palette { query: query.to_string(), selected: 0 }),
        ..ViewState::default()
    }
}

/// The palette widget drawn into `area` of a terminal just big enough.
fn draw_palette(state: &GameState, query: &str, area: Rect) -> Vec<String> {
    let mut term = Terminal::new(TestBackend::new(area.right(), area.bottom()))
        .expect("TestBackend is infallible");
    let view = palette_view(query);
    term.draw(|f| overlays::palette(f, area, state, &view)).expect("TestBackend is infallible");
    lines_of(term.backend().buffer(), area)
}

/// Tall and wide enough that nothing is cut: every result fits on its own
/// line. Width is kept well clear of ratatui's `TestBackend` — a `Buffer`
/// whose cell count (width × height) passes 65,536 corrupts its own
/// rendering, silently, rather than erroring — which a `rich()`-sized
/// palette (hundreds of results) would reach at the fuller 200 columns a
/// smaller fixture could afford.
fn roomy(state: &GameState) -> Rect {
    Rect::new(0, 0, 100, results(state, "").len() as u16 + 20)
}

/// The body at the 80x24 floor — the room the frame really gives the palette.
fn floor_rect() -> Rect {
    let layout = compute(Rect::new(0, 0, 80, 24), None, false);
    match layout.body {
        Body::Narrow { sidebar, main } => sidebar.union(main),
        Body::Wide { sidebar, inspector, .. } => sidebar.union(inspector),
        Body::TooSmall(r) => panic!("the floor is a supported size, not {r:?}"),
    }
}

/// Where `needle` occurs in `hay` as a whole name — not glued to a longer word
/// on either side — as byte offsets.
fn whole_occurrences(hay: &str, needle: &str) -> Vec<usize> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(at) = hay[from..].find(needle) {
        let start = from + at;
        let end = start + needle.len();
        let before = hay[..start].chars().next_back();
        let after = hay[end..].chars().next();
        let glued = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric());
        if !glued(before) && !glued(after) {
            out.push(start);
        }
        from = start + needle.len().max(1);
    }
    out
}

fn shows_whole(line: &str, name: &str) -> bool {
    !whole_occurrences(line, name).is_empty()
}

/// Whether `digit` stands alone in `line` outside the byte span `skip`.
fn has_lone_digit(line: &str, digit: char, skip: (usize, usize)) -> bool {
    let rest = format!("{} {}", &line[..skip.0], &line[skip.1..]);
    let chars: Vec<char> = rest.chars().collect();
    chars.iter().enumerate().any(|(i, c)| {
        *c == digit
            && !(i > 0 && chars[i - 1].is_alphanumeric())
            && !chars.get(i + 1).is_some_and(|n| n.is_alphanumeric())
    })
}

/// The name a list row displays: draw the section with that row focused, find
/// the cursor's line in the list pane, and read what follows the cursor glyph
/// and the availability marker.
fn displayed_after_cursor(state: &GameState, section: Section, key: &RowKey) -> String {
    let (w, h) = (150u16, 200u16);
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("TestBackend is infallible");
    let mut view = ViewState { section: Some(section), ..Default::default() };
    view.focus[section.index()] = Some(key.clone());
    let mut layout = None;
    term.draw(|f| layout = Some(render::frame(f, state, &view, None)))
        .expect("TestBackend is infallible");
    let list = match layout.expect("frame returns its layout").body {
        Body::Wide { list, .. } => list,
        other => panic!("fixture: {w}x{h} lays out a list pane, not {other:?}"),
    };
    let line = lines_of(term.backend().buffer(), list)
        .into_iter()
        .find(|l| l.contains('▸'))
        .unwrap_or_else(|| panic!("fixture: no cursor line drawn for {key:?} in {}", section.title()));
    // Cursor glyph, space, marker, space — then the name.
    line.chars().skip_while(|c| *c != '▸').skip(4).collect()
}

// ---------------------------------------------------------------------------
// TUI-NAV-063 — what the palette finds
// ---------------------------------------------------------------------------

/// The results are exactly the items the revealed sections list: one per item
/// with a selectable row, however many rows the item carries, and never a
/// group header. So nothing the cursor can reach is missing, and nothing is
/// offered that would land nowhere.
///
/// @spec TUI-NAV-063
#[test]
fn the_results_are_exactly_the_items_the_revealed_sections_list() {
    for (label, state) in every_state() {
        let got = placed(&results(&state, ""));
        let want = listed_items(&state);

        for (section, key) in &want {
            assert!(
                got.contains(&(*section, key.clone())),
                "in {label}, the item whose first row is {key:?} in {} is listed there but is \
                 not a palette result",
                section.title()
            );
        }
        for (section, key) in &got {
            assert!(
                want.contains(&(*section, key.clone())),
                "in {label}, the palette offered {key:?} in {}, which is not the first row of an \
                 item that section lists",
                section.title()
            );
        }
        assert_eq!(
            got.len(),
            want.len(),
            "in {label}, the palette has {} results for {} listed items; each item is exactly \
             one result",
            got.len(),
            want.len()
        );
    }
}

/// The edges of "listed": a Teased item is listed, so it is a result; Hidden
/// content, an undiscovered colony, and a gated action have no row, so they
/// never are.
///
/// @spec TUI-NAV-063
#[test]
fn a_teased_item_is_found_and_an_item_with_no_row_never_is() {
    let state = young();
    let keys = placed(&results(&state, ""));
    for b in StructureId::ALL.iter().take(YOUNG_REVEALED) {
        assert!(
            keys.contains(&(Section::Meadow, RowKey::Content(ContentId::Structure(*b), ""))),
            "a Teased structure is listed in its section and was not a palette result"
        );
    }
    for b in StructureId::ALL.iter().skip(YOUNG_REVEALED) {
        assert!(
            !keys.iter().any(|(_, k)| *k == RowKey::Content(ContentId::Structure(*b), "")),
            "a Hidden structure was a palette result; the palette never reads ahead"
        );
    }

    let state = rich();
    assert!(
        !results(&state, "")
            .iter()
            .any(|r| matches!(r.key, RowKey::Colony(c, _) if c == UNDISCOVERED)),
        "an undiscovered colony was a palette result; it has no row, so it is never one"
    );
}

/// Each result carries the name its rows display — the one the player would
/// read on that row in the section list.
///
/// @spec TUI-NAV-063
#[test]
fn each_result_carries_the_name_its_rows_display() {
    for (label, state) in every_state() {
        for r in results(&state, "") {
            let shown = displayed_after_cursor(&state, r.section, &r.key);
            let matches = if let Some(rest) = shown.strip_prefix(r.name.as_str()) {
                // The name is followed by padding, never by more of a longer name.
                rest.is_empty() || rest.starts_with("  ") || rest.starts_with(' ') && !rest[1..].starts_with(char::is_alphabetic)
            } else if let Some(cut) = shown.find('…') {
                cut > 0 && r.name.starts_with(&shown[..cut])
            } else {
                false
            };
            assert!(
                matches,
                "in {label}, the palette names {:?} in {} as {:?}, but its row in the list reads \
                 {:?}; a result shows the name its rows display",
                r.key,
                r.section.title(),
                r.name,
                shown.trim_end()
            );
        }
    }
}

/// Two items sharing a name are two results — within one section and across
/// two — so neither silently vanishes from the list.
///
/// @spec TUI-NAV-063
#[test]
fn two_items_sharing_a_name_are_two_results() {
    let state = rich();
    let all = results(&state, "");

    let twins: Vec<&PaletteResult> = all.iter().filter(|r| r.name == TWIN).collect();
    assert_eq!(
        twins.len(),
        2,
        "two kittens share one name, and the palette listed {} result(s) under it; each item is \
         its own result",
        twins.len()
    );
    assert_ne!(twins[0].key, twins[1].key, "the two same-named results point at one item");
    assert!(
        results(&state, TWIN).iter().filter(|r| r.name == TWIN).count() == 2,
        "searching the shared name found fewer than both items"
    );

    let shared = structure_def(StructureId::ALL[0]).name;
    let across: Vec<Section> = all.iter().filter(|r| r.name == shared).map(|r| r.section).collect();
    assert!(
        across.contains(&Section::Meadow) && across.contains(&Section::Clowder),
        "a kitten and a structure share a name in two sections, and the palette listed them in \
         {across:?}; neither may be merged into the other"
    );
}

/// Drawn, the palette shows each result beside its section's digit — and draws
/// each item on its own line, so two items sharing a name are two lines.
///
/// @spec TUI-NAV-063, TUI-NAV-038
#[test]
fn the_drawn_palette_shows_each_result_beside_its_sections_digit() {
    for (label, state) in every_state() {
        let all = results(&state, "");
        let lines = draw_palette(&state, "", roomy(&state));
        let mut used = vec![false; lines.len()];

        let mut by_length: Vec<&PaletteResult> = all.iter().collect();
        by_length.sort_by_key(|r| std::cmp::Reverse(r.name.chars().count()));

        for r in by_length {
            let digit = r.section.digit();
            let found = lines.iter().enumerate().find(|(i, line)| {
                !used[*i]
                    && whole_occurrences(line, &r.name)
                        .into_iter()
                        .any(|at| has_lone_digit(line, digit, (at, at + r.name.len())))
            });
            match found {
                Some((i, _)) => used[i] = true,
                None => panic!(
                    "in {label}, no line of the drawn palette shows {:?} beside the digit {digit:?} \
                     of {} (each result on a line of its own). The palette read:\n{}",
                    r.name,
                    r.section.title(),
                    lines.join("\n")
                ),
            }
        }
    }
}

/// The results are re-derived from current state: content revealed while the
/// palette sits open is found by the next Enter, with no reopening.
///
/// @spec TUI-NAV-063
#[test]
fn the_results_follow_the_state_beneath_an_open_palette() {
    let mut state = young();
    let revealed_names: Vec<String> =
        results(&state, "").into_iter().map(|r| r.name.to_lowercase()).collect();

    // A structure not yet revealed whose name no other name contains, so the
    // query aims at it alone once it appears.
    let target = StructureId::ALL
        .iter()
        .copied()
        .skip(YOUNG_REVEALED)
        .find(|b| {
            let n = structure_def(*b).name.to_lowercase();
            !revealed_names.iter().any(|m| m.contains(&n))
                && !StructureId::ALL
                    .iter()
                    .any(|o| o != b && structure_def(*o).name.to_lowercase().contains(&n))
        })
        .expect("fixture: some hidden structure has a name no other name contains");
    let name = structure_def(target).name;
    let target_key = RowKey::Content(ContentId::Structure(target), "");

    let mut view = ViewState::default();
    update(Action::SectionJump(Section::Meadow), &mut view, &state);
    update(Action::OpenPalette, &mut view, &state);
    for c in name.chars() {
        update(Action::Type(c), &mut view, &state);
    }
    assert!(results(&state, name).is_empty(), "fixture: nothing matches while it is hidden");
    update(Action::Activate, &mut view, &state);
    assert!(
        view.overlay.is_some(),
        "fixture: with nothing matching, Enter leaves the palette open (TUI-NAV-064)"
    );

    state.revealed.insert(ContentId::Structure(target));
    let now = results(&state, name);
    assert_eq!(
        now.first().map(|r| (r.section, r.key.clone())),
        Some((Section::Meadow, target_key.clone())),
        "content revealed beneath the palette is not its top result for its own name"
    );

    let out = update(Action::Activate, &mut view, &state);
    assert_eq!(out, Outcome::Handled);
    assert!(view.overlay.is_none(), "Enter did not jump to content revealed while the palette was open");
    assert_eq!(view.section, Some(Section::Meadow));
    assert_eq!(
        view.focus[Section::Meadow.index()],
        Some(target_key),
        "Enter did not land on content revealed while the palette was open; the results are \
         re-derived from current state, at the moment of the press"
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-062 — matching and order
// ---------------------------------------------------------------------------

/// With no query — empty, or nothing but spaces — the palette lists every
/// result, in tab-bar order and then in each section's list order.
///
/// @spec TUI-NAV-062
#[test]
fn a_blank_query_lists_every_result_in_tab_bar_then_list_order() {
    for (label, state) in every_state() {
        for query in ["", " ", "   "] {
            assert_eq!(
                placed(&results(&state, query)),
                listed_items(&state),
                "in {label}, the query {query:?} did not list every result in tab-bar then list \
                 order; a blank query is no query"
            );
        }
    }
}

/// The query matches as typed — spaces included, case ignored in every
/// comparison — and the matches rank by closeness: equal names, then names
/// beginning with the query, then names containing it elsewhere, ties in
/// tab-bar then list order. Quantified over queries cut from every name.
///
/// @spec TUI-NAV-062, TUI-NAV-038
#[test]
fn the_query_matches_ignoring_case_spaces_included_and_ranks_by_closeness() {
    let state = rich();
    let all = results(&state, "");
    for query in query_corpus(&all) {
        let got = results(&state, &query);
        assert_eq!(
            placed(&got),
            expected_order(&all, &query),
            "the query {query:?} found or ordered the wrong results; a name matches when it \
             contains the query, ignoring case, and matches rank equal, then prefix, then \
             elsewhere, ties in tab-bar then list order"
        );
        for r in &got {
            let unfiltered = all
                .iter()
                .find(|a| a.section == r.section && a.key == r.key)
                .expect("a filtered result is one of the unfiltered ones");
            assert_eq!(r.name, unfiltered.name, "a result's name changed with the query");
        }
    }
}

/// The list is drawn from its top: whatever the ranking put first is the first
/// result on screen — even when more match than fit, and even when the top
/// result comes from the last section in the tab bar.
///
/// @spec TUI-NAV-062
#[test]
fn the_top_result_is_the_first_one_drawn_and_always_on_screen() {
    let state = rich();
    let all = results(&state, "");
    let area = floor_rect();

    let mut queries: Vec<String> = vec!["e".to_string(), "a".to_string()];
    queries.push(all.last().expect("fixture: results exist").name.clone());
    queries.push(all.last().expect("fixture: results exist").name.to_uppercase());
    queries.push(TWIN.to_string());
    queries.push(structure_def(StructureId::ALL[0]).name.to_string());
    assert!(
        results(&state, "e").len() > area.height as usize,
        "fixture: more results match than the floor can show"
    );

    let head = |name: &str| -> String { name.chars().take(16).collect() };
    for query in queries {
        let ranked = results(&state, &query);
        let top = ranked.first().unwrap_or_else(|| panic!("fixture: {query:?} matches something"));
        let lines = draw_palette(&state, &query, area);
        let first_line_of =
            |name: &str| lines.iter().position(|l| l.contains(&head(name)));

        let top_at = first_line_of(&top.name).unwrap_or_else(|| {
            panic!(
                "for {query:?} the top result {:?} is not on screen at the floor; the list is \
                 drawn from its top, so the result Enter takes is always shown. It read:\n{}",
                top.name,
                lines.join("\n")
            )
        });
        let q = query.to_lowercase();
        for other in &ranked[1..] {
            let h = head(&other.name);
            let hl = h.to_lowercase();
            if top.name.to_lowercase().contains(&hl) || q.contains(&hl) {
                continue;
            }
            if let Some(at) = first_line_of(&other.name) {
                assert!(
                    at > top_at,
                    "for {query:?}, {:?} is drawn above the top result {:?}; the result Enter \
                     takes is the first one shown. It read:\n{}",
                    other.name,
                    top.name,
                    lines.join("\n")
                );
            }
        }
    }
}

/// Drawn, the palette narrows to exactly the matches — case ignored — rather
/// than to what the typed case happens to spell.
///
/// @spec TUI-NAV-038
#[test]
fn the_drawn_palette_narrows_to_the_matches_ignoring_case() {
    let state = rich();
    let all = results(&state, "");
    let area = roomy(&state);

    let mut queries: Vec<String> = Vec::new();
    for r in all.iter().step_by(7) {
        let head: String = r.name.chars().take(3).collect();
        queries.push(head.to_uppercase());
        queries.push(head.to_lowercase());
    }

    for query in queries {
        let matching = results(&state, &query);
        let lines = draw_palette(&state, &query, area);
        let text = lines.join("\n");
        for r in &matching {
            assert!(
                lines.iter().any(|l| shows_whole(l, &r.name)),
                "{query:?} matches {:?}, ignoring case, and the drawn palette does not show it:\n{text}",
                r.name
            );
        }
        let q = query.to_lowercase();
        for r in &all {
            if matching.iter().any(|m| m.name == r.name) {
                continue;
            }
            let n = r.name.to_lowercase();
            if q.contains(&n) || matching.iter().any(|m| m.name.to_lowercase().contains(&n)) {
                continue;
            }
            assert!(
                !lines.iter().any(|l| shows_whole(l, &r.name)),
                "{query:?} does not match {:?}, and the drawn palette still shows it:\n{text}",
                r.name
            );
        }
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-038 — where Enter lands
// ---------------------------------------------------------------------------

/// Enter closes the palette, moves to the top result's section, and focuses
/// that item's first row there — performing no row action. Quantified over
/// queries cut from every name, so every section and every kind of item is
/// landed on.
///
/// @spec TUI-NAV-038
#[test]
fn enter_lands_on_the_top_results_first_row_and_does_nothing_else() {
    let state = rich();
    let all = results(&state, "");
    let mut queries: BTreeSet<String> = BTreeSet::new();
    for r in &all {
        queries.insert(r.name.clone());
        queries.insert(r.name.to_uppercase());
        queries.insert(r.name.chars().take(2).collect());
    }

    let mut sections_landed: BTreeSet<Section> = BTreeSet::new();
    for query in queries {
        if query.chars().all(|c| c == ' ') {
            continue;
        }
        let top = results(&state, &query)
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("fixture: {query:?} matches something"));

        let mut view = ViewState::default();
        update(Action::SectionJump(Section::Meadow), &mut view, &state);
        update(Action::OpenPalette, &mut view, &state);
        for c in query.chars() {
            update(Action::Type(c), &mut view, &state);
        }
        let out = update(Action::Activate, &mut view, &state);

        assert_eq!(
            out,
            Outcome::Handled,
            "Enter on {query:?} asked the core for {out:?}; the palette's jump performs no row \
             action"
        );
        assert!(view.overlay.is_none(), "Enter on {query:?} left the palette open");
        assert_eq!(
            view.section,
            Some(top.section),
            "Enter on {query:?} did not move to the top result's section"
        );
        assert_eq!(
            view.focus[top.section.index()],
            Some(top.key.clone()),
            "Enter on {query:?} did not focus the top result's first row"
        );
        assert!(view.prompt.is_none() && view.armed.is_none(), "Enter on {query:?} opened or armed something");
        sections_landed.insert(top.section);
    }
    assert_eq!(
        sections_landed.len(),
        revealed_sections(&state).len(),
        "fixture: the queries land in every revealed section"
    );
}
