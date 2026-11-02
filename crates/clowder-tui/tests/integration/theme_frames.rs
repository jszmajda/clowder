//! The theme read off whole frames rather than tables: what any cell of a
//! drawn frame may wear, and what the frame's header and focused row wear.
//!
//! @spec TUI-THEME-001, TUI-THEME-004, TUI-THEME-005, TUI-THEME-006,
//! TUI-THEME-007, TUI-THEME-008

use std::collections::HashSet;

use clowder_core::content;
use clowder_core::{
    AdventureId, Availability, ContentId, EyeColour, EyeLook, Fur, FurColour, FurPattern,
    GameState, IdeaId, Kitten, KittenTrait, Location, Pose, Profile, ResourceId, Season,
    StructureId,
};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier};
use ratatui::Terminal;

use clowder_tui::nav::{RowKey, Section, ViewState};
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
