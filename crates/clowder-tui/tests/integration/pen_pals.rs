//! A neighbour's pen pals made visible: the next letter's price on the row,
//! what the letters have done in the inspector, and a ticker line that says
//! which neighbour and what level.
//!
//! @spec TUI-NAV-171, TUI-NAV-172, TUI-NAV-173

use clowder_core::content::colony_def;
use clowder_core::{ColonyId, Event, GameState, IdeaId};
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;

use clowder_tui::nav::{RowKey, Section, ViewState};
use clowder_tui::render::fmt;
use clowder_tui::render::widgets::{inspector, overlays};

const W: u16 = 120;
const H: u16 = 40;

fn meadow() -> GameState {
    let mut s = GameState::new(51);
    s.ideas.insert(IdeaId::GiftEconomy);
    s.ideas.insert(IdeaId::PenPalNetworks);
    for c in [ColonyId::Otters, ColonyId::Foxes, ColonyId::Deer] {
        s.discovered.insert(c);
    }
    s
}

fn pen_pal_text(state: &GameState, c: ColonyId) -> String {
    let mut view = ViewState {
        section: Some(Section::Friends),
        ..ViewState::default()
    };
    view.focus[Section::Friends.index()] = Some(RowKey::Colony(c, "penpal"));
    let rect = Rect::new(0, 0, W, H);
    let mut term = Terminal::new(TestBackend::new(W, H)).expect("TestBackend is infallible");
    term.draw(|f| inspector::draw(f, rect, state, &view))
        .expect("TestBackend is infallible");
    let buf = term.backend().buffer().clone();
    (0..H)
        .map(|y| {
            (0..W)
                .map(|x| buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "))
                .collect::<String>()
                .trim()
                .to_string()
        })
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// The next letter's price is stated as the row's next cost.
/// @spec TUI-NAV-171
#[test]
fn the_pen_pal_row_states_the_next_letters_price() {
    let mut s = meadow();
    s.penpal[ColonyId::Otters.ix()] = 2;
    let price = s.pen_pal(ColonyId::Otters).unwrap().price;
    let text = pen_pal_text(&s, ColonyId::Otters);
    assert!(text.contains(&fmt::exact(price)), "{price} in: {text}");
}

/// Level, standing in words, what it does to a caravan, what another letter
/// would do, and the level at which something more opens.
/// @spec TUI-NAV-172
#[test]
fn the_pen_pal_row_says_what_letters_have_done() {
    let mut s = meadow();
    s.penpal[ColonyId::Otters.ix()] = 2;
    let otters = colony_def(ColonyId::Otters).name;
    let text = pen_pal_text(&s, ColonyId::Otters);
    for want in [
        "Pen pals: 2.".to_string(),
        format!("The {otters} are warm to you: +27%. Your letters added +2%."),
        "About 27% of caravans come home with a bonus.".to_string(),
        "Another letter warms them by 1%.".to_string(),
    ] {
        assert!(text.contains(&want), "missing {want:?} in: {text}");
    }

    let deer = colony_def(ColonyId::Deer).name;
    let text = pen_pal_text(&s, ColonyId::Deer);
    for want in [
        "Pen pals: 0.".to_string(),
        format!("The {deer} are neutral towards you."),
        "Caravans trade at the usual odds.".to_string(),
        "At pen pal level 5, they'll trade something more.".to_string(),
    ] {
        assert!(text.contains(&want), "missing {want:?} in: {text}");
    }

    s.penpal[ColonyId::Foxes.ix()] = 5;
    let foxes = colony_def(ColonyId::Foxes).name;
    let text = pen_pal_text(&s, ColonyId::Foxes);
    for want in [
        format!("The {foxes} are shy of you: −10%. Your letters added +5%."),
        "About 10% of caravans find them napping.".to_string(),
    ] {
        assert!(text.contains(&want), "missing {want:?} in: {text}");
    }

    s.penpal[ColonyId::Foxes.ix()] = 30;
    let text = pen_pal_text(&s, ColonyId::Foxes);
    assert!(
        text.contains("Letters can't warm them any further."),
        "{text}"
    );
}

/// A letter's ticker line names the neighbour and the level.
/// @spec TUI-NAV-173
#[test]
fn a_letter_reads_as_a_letter_on_the_ticker() {
    let mut s = meadow();
    s.journal.record(
        s.ticks,
        &Event::PenPalSent {
            colony: ColonyId::Otters,
            level: 3,
        },
    );
    let mut term = Terminal::new(TestBackend::new(W, 1)).expect("TestBackend is infallible");
    term.draw(|f| overlays::ticker(f, Rect::new(0, 0, W, 1), &s))
        .expect("TestBackend is infallible");
    let line: String = (0..W)
        .map(|x| {
            term.backend()
                .buffer()
                .cell((x, 0))
                .unwrap()
                .symbol()
                .to_string()
        })
        .collect();
    let want = format!(
        "a letter to the {} — pen pals, level 3",
        colony_def(ColonyId::Otters).name
    );
    assert!(line.contains(&want), "{line}");
}
