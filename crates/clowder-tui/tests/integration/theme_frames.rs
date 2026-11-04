//! The theme read off whole frames rather than tables: what any cell of a
//! drawn frame may wear, and what the frame's header and focused row wear.
//!
//! @spec TUI-THEME-001, TUI-THEME-003, TUI-THEME-004, TUI-THEME-005,
//! TUI-THEME-006, TUI-THEME-007, TUI-THEME-008, TUI-THEME-010

use std::collections::{BTreeMap, BTreeSet, HashSet};

use clowder_core::content;
use clowder_core::{
    AdventureId, ArchId, Availability, ContentId, CraftId, DreamLoomId, EyeColour, EyeLook, Fur,
    FurColour, FurPattern, GameState, IdeaId, JobId, Kitten, KittenTrait, Location, Pose, Profile,
    RefinementId, ResourceId, RiteId, RuleId, Season, StructureId, TraditionId,
};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier};
use ratatui::Terminal;

use clowder_tui::nav::rows::{drawn_rows, rows, RowKind};
use clowder_tui::nav::{RowKey, Section, ViewState};
use clowder_tui::render::layout::{Body, Layout};
use clowder_tui::theme::fur::{eye_colour, fur_colour};
use clowder_tui::theme::{coat, coat_colours, seasonal_accent, Role};

/// The sizes a frame takes: the floor, each layout, and room to spare.
const SIZES: [(u16, u16); 4] = [(80, 24), (100, 30), (130, 40), (200, 60)];

fn world() -> GameState {
    let mut s = GameState::new(11);
    for r in ResourceId::ALL {
        s.resources[r.ix()] = 5.0;
    }
    for b in StructureId::ALL {
        s.structures[b.ix()] = 2;
    }
    let fur = Fur {
        pattern: FurPattern::Solid,
        colours: vec![content::fur_pattern_def(FurPattern::Solid).slots[0][0]],
    };
    s.kittens = vec![Kitten {
        name: "Pip".to_string(),
        job: None,
        location: Location::Home,
        arrived_day: 12,
        job_seq: None,
        away_seq: None,
        skills: [0.0; 8],
        exp: 0.0,
        rank: 2,
        trait_id: KittenTrait::Brassy,
        tinkering: None,
        profile: Some(Profile {
            fur,
            eyes: EyeLook::Green,
            favourites: Vec::new(),
            pose: Pose::Sitting,
        }),
    }];
    s
}

fn drawn(s: &GameState, view: &ViewState, (w, h): (u16, u16)) -> Buffer {
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("TestBackend is infallible");
    term.draw(|f| {
        clowder_tui::render::frame(f, s, view, None);
    })
    .expect("TestBackend is infallible");
    term.backend().buffer().clone()
}

fn line(buf: &Buffer, y: u16) -> String {
    (0..buf.area.width)
        .map(|x| buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "))
        .collect()
}

/// Every colour the theme lets a frame's cell wear.
fn allowed() -> HashSet<Color> {
    let mut out: HashSet<Color> = HashSet::new();
    out.insert(Color::Reset);
    for role in [
        Role::Dim,
        Role::Ready,
        Role::Partial,
        Role::Attention,
        Role::Milestone,
    ] {
        out.insert(role.color().expect("a state colour"));
    }
    for season in Season::ALL {
        out.insert(seasonal_accent(season));
    }
    for r in ResourceId::ALL {
        for n in 1..=32 {
            out.extend(coat_colours(coat(r), n));
        }
    }
    out.extend(FurColour::ALL.map(fur_colour));
    out.extend(EyeColour::ALL.map(eye_colour));
    out
}

/// On every section at every size, no cell paints a ground, and none wears a
/// colour outside the theme's vocabulary.
/// @spec TUI-THEME-001, TUI-THEME-004
#[test]
fn no_cell_of_any_frame_paints_a_ground_or_leaves_the_vocabulary() {
    let s = world();
    let allowed = allowed();
    for section in Section::ALL {
        let view = ViewState {
            section: Some(section),
            ..ViewState::default()
        };
        for size in SIZES {
            let buf = drawn(&s, &view, size);
            for y in 0..size.1 {
                for x in 0..size.0 {
                    let cell = buf.cell((x, y)).expect("every cell exists");
                    assert_eq!(cell.bg, Color::Reset, "{section:?} {size:?} ({x},{y})");
                    assert!(
                        allowed.contains(&cell.fg),
                        "{section:?} {size:?} ({x},{y}) wears {:?}",
                        cell.fg
                    );
                }
            }
        }
    }
}

/// The header's mood is a word in the inherited foreground.
/// @spec TUI-THEME-005
#[test]
fn the_mood_word_on_a_drawn_header_wears_no_colour() {
    let s = world();
    let buf = drawn(&s, &ViewState::default(), (200, 60));
    let header = line(&buf, 0);
    let facts: Vec<&str> = header.split(" \u{b7} ").collect();
    let mood = facts[3].trim();
    assert!(!mood.is_empty(), "fixture: the full facts: {header:?}");
    let chars: Vec<char> = header.chars().collect();
    let word: Vec<char> = mood.chars().collect();
    let at = chars
        .windows(word.len())
        .position(|w| w == word.as_slice())
        .expect("the word stands in the header");
    for x in at..at + word.len() {
        assert_eq!(
            buf.cell((x as u16, 0)).unwrap().fg,
            Color::Reset,
            "{mood:?} at column {x}"
        );
    }
}

/// A focused row that wears no state's colour takes focus from the cursor glyph
/// and the reversal alone: no palette colour reaches it, and the terminal's own
/// pair does the inverting.
/// @spec TUI-THEME-006
#[test]
fn focus_on_a_row_with_no_state_colour_is_the_glyph_and_the_terminals_pair() {
    let s = world();
    let mut view = ViewState {
        section: Some(Section::Meadow),
        ..ViewState::default()
    };
    view.focus[Section::Meadow.index()] = Some(RowKey::Resource(ResourceId::Catnip));
    let (w, h) = (130u16, 60u16);
    let buf = drawn(&s, &view, (w, h));
    let y = (0..h)
        .find(|y| line(&buf, *y).contains('\u{25b8}'))
        .expect("the cursor is drawn");
    let row = line(&buf, y);
    let start = row.chars().position(|c| c == '\u{25b8}').unwrap();
    let name = "Catnip".chars().count();
    for x in start..start + 2 + 2 + name {
        let cell = buf.cell((x as u16, y)).unwrap();
        assert!(
            cell.modifier.contains(Modifier::REVERSED),
            "column {x} of {row:?}"
        );
        assert_eq!(cell.fg, Color::Reset, "column {x} of {row:?}");
        assert_eq!(cell.bg, Color::Reset, "column {x} of {row:?}");
    }
}

/// While the meadow is held in a season the calendar doesn't say, the header's
/// word follows the meadow at every width step, as the chrome's accent does.
/// @spec TUI-THEME-008
#[test]
fn the_headers_season_word_follows_the_season_the_meadow_is_in() {
    let mut s = world();
    assert_eq!(s.season(), Season::Spring, "fixture: a new game's calendar");
    s.active_adventure = Some(AdventureId::LongWinter);
    assert_eq!(s.season(), Season::Winter, "fixture: the season is held");
    for w in [200u16, 130, 80] {
        let header = line(&drawn(&s, &ViewState::default(), (w, 30)), 0);
        assert!(header.contains("Win"), "at {w} columns: {header:?}");
        assert!(!header.contains("Spr"), "at {w} columns: {header:?}");
    }
}

/// A focused row keeps the state colour it wears: the reversal is the only thing
/// focus adds, so the bar is that colour, nothing paints a ground, and the row
/// looks the same under the cursor at every state save for being reversed.
/// @spec TUI-THEME-006, TUI-THEME-007
#[test]
fn a_focused_row_keeps_its_state_colour_under_the_reversal() {
    let mut s = world();
    // Some prices met, some half met, some not at all, so every marker stands.
    for r in ResourceId::ALL {
        s.resources[r.ix()] = 0.0;
    }
    s.resources[ResourceId::Catnip.ix()] = 40.0;
    s.resources[ResourceId::Twigs.ix()] = 1.0;
    s.ideas = IdeaId::ALL.into_iter().collect();
    s.revealed = StructureId::ALL
        .into_iter()
        .map(ContentId::Structure)
        .collect();
    let (w, h) = (130u16, 60u16);
    let mut seen: HashSet<Color> = HashSet::new();
    // A kind with no reveal ladder is never teased, so `partial` shows on those
    // rows: walk every section, not only the one with the structures.
    let every_row: Vec<(Section, clowder_tui::nav::rows::Row)> = Section::ALL
        .into_iter()
        .flat_map(|section| {
            clowder_tui::nav::rows::rows(&s, section)
                .into_iter()
                .map(move |row| (section, row))
        })
        .collect();
    for (section, row) in every_row {
        if !row.selectable() {
            continue;
        }
        let mut view = ViewState {
            section: Some(section),
            ..ViewState::default()
        };
        view.focus[section.index()] = Some(row.key.clone());
        let buf = drawn(&s, &view, (w, h));
        let Some(y) = (2..h - 2).find(|y| line(&buf, *y).contains('\u{25b8}')) else {
            continue;
        };
        let text: Vec<char> = line(&buf, y).chars().collect();
        let at = text.iter().position(|c| *c == '\u{25b8}').unwrap();
        let marker = text[at + 2];
        // A teased row recedes whatever its glyph (TUI-NAV-009), and keeps that
        // dim under the cursor as any row keeps its colour.
        let teased = matches!(
            row.key,
            RowKey::Content(id, _) if s.availability(id) == Availability::Teased
        );
        let want = match marker {
            '\u{25cf}' => Role::Ready,
            '\u{25d0}' if !teased => Role::Partial,
            '\u{25d0}' | '\u{25cb}' => Role::Dim,
            _ => continue,
        }
        .color()
        .expect("a state colour");
        seen.insert(want);
        for x in at..at + 2 + 2 + 3 {
            let cell = buf.cell((x as u16, y)).unwrap();
            assert!(
                cell.modifier.contains(Modifier::REVERSED),
                "column {x} of the row with {marker:?} is reversed"
            );
            assert_eq!(cell.fg, want, "column {x} of the row with {marker:?}");
            assert_eq!(cell.bg, Color::Reset, "column {x}: no ground of its own");
        }
    }
    assert_eq!(seen.len(), 3, "fixture: every state's colour met: {seen:?}");
}

// ---------------------------------------------------------------------------
// TUI-THEME-003 — state, never identity
// ---------------------------------------------------------------------------

/// Tall enough that no list scrolls, so a row stands at its place in the list.
const TALL_FRAME: (u16, u16) = (200, 300);

fn every_content_id() -> Vec<ContentId> {
    let mut out: Vec<ContentId> = Vec::new();
    out.extend(StructureId::ALL.map(ContentId::Structure));
    out.extend(IdeaId::ALL.map(ContentId::Idea));
    out.extend(JobId::ALL.map(ContentId::Job));
    out.extend(CraftId::ALL.map(ContentId::Craft));
    out.extend(RefinementId::ALL.map(ContentId::Refinement));
    out.extend(RuleId::ALL.map(ContentId::Rule));
    out.extend(RiteId::ALL.map(ContentId::Rite));
    out.extend(ArchId::ALL.map(ContentId::Arch));
    out.extend(TraditionId::ALL.map(ContentId::Tradition));
    out.extend(DreamLoomId::ALL.map(ContentId::DreamLoom));
    out
}

/// The meadow with every item revealed and every store at `stock`.
fn everything_listed(stock: f64) -> GameState {
    let mut s = world();
    s.revealed = every_content_id().into_iter().collect();
    for r in ResourceId::ALL {
        s.resources[r.ix()] = stock;
    }
    s
}

/// A frame and its layout, the cursor on the section's first row it can rest on.
fn section_frame(s: &GameState, section: Section) -> Option<(Buffer, Layout, ViewState)> {
    let mut view = ViewState {
        section: Some(section),
        done_open: true,
        ..Default::default()
    };
    let first = drawn_rows(s, section, &view)
        .into_iter()
        .find(|r| !matches!(r.kind, RowKind::Group | RowKind::Display))?;
    view.focus[section.index()] = Some(first.key);

    let (w, h) = TALL_FRAME;
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("TestBackend is infallible");
    let mut layout = None;
    term.draw(|f| layout = Some(clowder_tui::render::frame(f, s, &view, None)))
        .expect("TestBackend is infallible");
    let layout = layout.expect("a frame returns its layout");
    Some((term.backend().buffer().clone(), layout, view))
}

/// The list's first column and first row: a row's cursor stands there, its marker
/// two columns on, and its name two more.
fn list_origin(layout: &Layout) -> (u16, u16) {
    match layout.body {
        Body::Wide { list, .. } => (list.x + 1, list.y),
        _ => panic!("fixture: {TALL_FRAME:?} lays out three columns"),
    }
}

/// Two rows in one state wear one colour, whichever items they are: read off
/// every content row of every section, in meadows that cover every price, some
/// prices, and no price, with nothing learned and everything learned, so a late
/// item meets the same rows as an early one.
///
/// @spec TUI-THEME-003
#[test]
fn a_row_takes_its_colours_from_its_state_and_never_from_which_item_it_is() {
    let mut worn: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut items: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    let meadows = [1.0e12, 5.0, 0.0].into_iter().flat_map(|stock| {
        let open = everything_listed(stock);
        let mut learned = open.clone();
        learned.ideas = IdeaId::ALL.into_iter().collect();
        [open, learned]
    });
    for s in meadows {
        for section in Section::ALL {
            let Some((buf, layout, view)) = section_frame(&s, section) else {
                continue;
            };
            let (x, top) = list_origin(&layout);
            for (i, row) in drawn_rows(&s, section, &view).iter().enumerate() {
                let RowKey::Content(id, _) = row.key else {
                    continue;
                };
                let y = top + i as u16;
                if buf[(x, y)].symbol() == "▸" {
                    continue;
                }
                let (marker, name) = (&buf[(x + 2, y)], &buf[(x + 4, y)]);
                let state = format!("{} {:?}", marker.symbol(), s.availability(id));
                worn.entry(state.clone())
                    .or_default()
                    .insert(format!("marker {:?}, name {:?}", marker.fg, name.fg));
                items.entry(state).or_default().insert(format!("{id:?}"));
            }
        }
    }

    for (state, colours) in &worn {
        assert_eq!(
            colours.len(),
            1,
            "rows in the state {state:?} wear different colours, so a colour is saying which \
             item this is: {colours:?}"
        );
    }
    assert!(
        worn.len() >= 3,
        "fixture: rows in several states: {:?}",
        worn.keys()
    );
    assert!(
        items.values().filter(|i| i.len() > 1).count() >= 3,
        "fixture: several states held by more than one item: {items:?}"
    );
}

// ---------------------------------------------------------------------------
// TUI-THEME-010 — colour is always the second channel
// ---------------------------------------------------------------------------

/// The text of one row of a frame's list, every colour gone.
fn row_text(s: &GameState, section: Section, key: &RowKey) -> String {
    let (buf, layout, view) = section_frame(s, section).expect("fixture: a list");
    let (_, top) = list_origin(&layout);
    let at = drawn_rows(s, section, &view)
        .iter()
        .position(|r| r.key == *key)
        .expect("fixture: the row is listed");
    line(&buf, top + at as u16)
}

/// With every colour taken off a drawn frame, a row still says which state it
/// is in: a price covered, part covered, and not covered at all read as three
/// different lines, a finished item as a fourth, and a store at its cap differs
/// from one just under it by the word `full`. The rest of what colour carries is
/// read off frames beside its own surface: a chip's glyph and word (`chrome.rs`),
/// an armed row's expansion (`armed_summary.rs`), an event's glyph
/// (`overlays.rs`), and the mood and season words here.
///
/// @spec TUI-THEME-010
#[test]
fn with_every_colour_removed_a_frame_still_says_each_state() {
    let id = StructureId::ALL
        .into_iter()
        .map(ContentId::Structure)
        .find(|id| {
            let s = everything_listed(1.0e12);
            s.availability(*id) == Availability::Available && !s.cost_of_next(*id, 1).is_empty()
        })
        .expect("fixture: a structure open to build");
    let key = rows(&everything_listed(1.0e12), Section::Meadow)
        .into_iter()
        .map(|r| r.key)
        .find(|k| matches!(k, RowKey::Content(c, _) if *c == id))
        .expect("fixture: its row is listed");

    let priced = |cover: f64| {
        let mut s = everything_listed(1.0e12);
        for (r, amount) in s.cost_of_next(id, 1) {
            s.resources[r.ix()] = amount * cover;
        }
        row_text(&s, Section::Meadow, &key)
    };
    let (covered, part, none) = (priced(2.0), priced(0.5), priced(0.0));
    for (line, glyph) in [(&covered, "●"), (&part, "◐"), (&none, "○")] {
        assert!(
            line.contains(glyph),
            "with its colour gone, the row should still carry {glyph}: {line:?}"
        );
    }
    assert!(
        covered != part && part != none && covered != none,
        "two affordability states read the same with colour gone:\n{covered}\n{part}\n{none}"
    );

    let idea = IdeaId::ALL[0];
    let idea_key = rows(&everything_listed(1.0e12), Section::Ideas)
        .into_iter()
        .map(|r| r.key)
        .find(|k| matches!(k, RowKey::Content(ContentId::Idea(i), _) if *i == idea))
        .expect("fixture: an idea is listed");
    let mut done = everything_listed(1.0e12);
    done.ideas.insert(idea);
    assert!(
        row_text(&done, Section::Ideas, &idea_key).contains('✓'),
        "with its colour gone, a finished item should still carry ✓"
    );

    let store = ResourceId::ALL
        .into_iter()
        .find(|r| {
            let cap = everything_listed(1.0).cap(*r);
            cap.is_finite() && cap > 1.0
        })
        .expect("fixture: some store has a cap");
    let sidebar_says_full = |stock: f64| {
        let mut s = everything_listed(1.0);
        s.resources[store.ix()] = stock;
        let (buf, layout, _) = section_frame(&s, Section::Meadow).expect("fixture: a list");
        let Body::Wide { sidebar, .. } = layout.body else {
            panic!("fixture: three columns");
        };
        (sidebar.y..sidebar.bottom())
            .map(|y| {
                (sidebar.x..sidebar.right())
                    .map(|x| buf[(x, y)].symbol().to_string())
                    .collect::<String>()
            })
            .any(|l| l.contains("full"))
    };
    let cap = everything_listed(1.0).cap(store);
    assert!(
        sidebar_says_full(cap),
        "with its colour gone, a store at its cap should still say `full`"
    );
    assert!(
        !sidebar_says_full(cap - 1.0),
        "a store under its cap says `full`"
    );
}
