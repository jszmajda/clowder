//! A row that spends states what it takes: every action row on a meadow where each
//! can be taken, read against what applying its command does.
//!
//! @spec TUI-NAV-207, TUI-RENDER-020

use clowder_core::{ArchId, ColonyId, Command, GameState, IdeaId, Kitten, KittenTrait, Location};
use clowder_core::{ResourceId, SiteId, StructureId};
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;

use clowder_tui::nav::rows::{rows, Row, RowKind};
use clowder_tui::nav::{RowKey, Section, ViewState};
use clowder_tui::render;
use clowder_tui::render::fmt;
use clowder_tui::render::layout::{compute, Body};
use clowder_tui::render::widgets::inspector;

const WIDE: u16 = 200;
const TALL: u16 = 70;

fn rich() -> GameState {
    let mut s = GameState::new(46);
    s.ideas = IdeaId::ALL.into_iter().collect();
    s.arches_bought = ArchId::ALL.into_iter().collect();
    s.sites_reached = SiteId::ALL.into_iter().take(3).collect();
    s.discovered = ColonyId::ALL.into_iter().collect();
    s.whale_visit_years = 3;
    for st in StructureId::ALL {
        s.structures[st.ix()] = 8;
    }
    for r in ResourceId::ALL {
        s.resources[r.ix()] = 1.0e7;
    }
    s.resources[ResourceId::Twigs.ix()] = 0.0;
    s.resources[ResourceId::JarsOfPurrs.ix()] = 0.0;
    s.kittens = vec![Kitten {
        name: "Pip".to_string(),
        job: None,
        location: Location::Home,
        arrived_day: 0,
        job_seq: None,
        away_seq: None,
        skills: [0.0; 8],
        exp: 1.0e9,
        rank: 0,
        trait_id: KittenTrait::None,
        tinkering: None,
        profile: None,
    }];
    s
}

fn looking_at(section: Section, key: &RowKey) -> ViewState {
    let mut v = ViewState {
        section: Some(section),
        ..Default::default()
    };
    v.focus[section.index()] = Some(key.clone());
    v
}

fn pane(s: &GameState, v: &ViewState) -> String {
    let rect = match compute(Rect::new(0, 0, WIDE, TALL), None, false).body {
        Body::Wide { inspector, .. } => inspector,
        other => panic!("fixture: three columns, not {other:?}"),
    };
    let mut term = Terminal::new(TestBackend::new(WIDE, TALL)).unwrap();
    term.draw(|f| inspector::draw(f, rect, s, v)).unwrap();
    let buf = term.backend().buffer().clone();
    (rect.y..rect.bottom())
        .map(|y| {
            (rect.x..rect.right())
                .map(|x| buf[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// A store's stores-row name, spaced at each capital.
fn store(r: ResourceId) -> String {
    let tag = format!("{r:?}");
    let mut out = String::new();
    for (i, c) in tag.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

fn action_rows(s: &GameState) -> Vec<(Section, Row, Command)> {
    Section::ALL
        .into_iter()
        .flat_map(|section| {
            rows(s, section).into_iter().filter_map(move |row| {
                let RowKind::Act { command, .. } = row.kind else {
                    return None;
                };
                Some((section, row, command))
            })
        })
        .collect()
}

/// Every action row that takes something from the stores says so in the inspector,
/// with need and have, read against what applying it does. A kitten's row waits on
/// #48, and a nap's price is previewed when its length is typed (TUI-NAV-067).
///
/// @spec TUI-NAV-207
#[test]
fn every_row_that_takes_from_the_stores_states_it_in_the_inspector() {
    let s = rich();
    let mut stated = 0;
    for (section, row, command) in action_rows(&s) {
        if matches!(row.key, RowKey::Kitten(..)) || matches!(command, Command::DeepNap(_)) {
            continue;
        }
        if matches!(row.key, RowKey::Content(..)) {
            continue; // Priced by a content item, as TUI-NAV-078 holds.
        }
        let mut after = s.clone();
        if after.apply(command).is_err() {
            continue;
        }
        let fell: Vec<ResourceId> = ResourceId::ALL
            .into_iter()
            .filter(|r| s.stock(*r) - after.stock(*r) > 1e-6)
            .collect();
        let text = pane(&s, &looking_at(section, &row.key));
        if fell.is_empty() {
            assert!(
                !text.contains(": need"),
                "{:?} takes nothing and states a price:\n{text}",
                row.key
            );
        }
        for r in fell {
            stated += 1;
            let head = format!("{}: need", store(r));
            assert!(
                text.contains(&head),
                "{:?} takes {r:?} and does not say so:\n{text}",
                row.key
            );
            assert!(
                text.contains(&format!("have {}", fmt::exact(s.stock(r)))),
                "{:?} does not say what is held of {r:?}:\n{text}",
                row.key
            );
        }
    }
    assert!(
        stated >= 12,
        "fixture: rows that spend were read ({stated})"
    );
}

/// The list draws the same price in the row's cost field, the Sky list among them.
///
/// @spec TUI-NAV-207, TUI-RENDER-020
#[test]
fn every_priced_row_draws_its_price_in_the_cost_field() {
    let s = rich();
    let mut drawn = 0;
    let mut sections = std::collections::BTreeSet::new();
    for (section, row, command) in action_rows(&s) {
        let Some(price) = s.act_price(&command) else {
            continue;
        };
        drawn += 1;
        sections.insert(format!("{section:?}"));
        let v = looking_at(section, &row.key);
        let mut term = Terminal::new(TestBackend::new(WIDE, TALL)).unwrap();
        term.draw(|f| {
            render::frame(f, &s, &v, None);
        })
        .unwrap();
        let buf = term.backend().buffer().clone();
        let line = (0..TALL)
            .map(|y| {
                (0..WIDE)
                    .map(|x| buf[(x, y)].symbol().to_string())
                    .collect::<String>()
            })
            .find(|l| l.contains('▸'))
            .expect("the cursor's row is drawn");
        let (r, amount) = price[0];
        assert!(
            line.contains(&format!("{} {}", fmt::compact(amount), store(r))),
            "{:?} does not draw its first component in its cost field: {line}",
            row.key
        );
    }
    assert!(drawn >= 10, "fixture: priced rows were listed ({drawn})");
    assert!(sections.contains("Sky"), "fixture: the Sky list is read");
}

/// A row that takes nothing leaves its cost field blank, and an empty store leaves
/// the act that spends all of it with nothing to state.
///
/// @spec TUI-NAV-207
#[test]
fn a_row_with_nothing_to_state_states_nothing() {
    let mut s = rich();
    s.resources[ResourceId::Warmth.ix()] = 0.0;
    let (section, row, _) = action_rows(&s)
        .into_iter()
        .find(|(_, _, c)| matches!(c, Command::Bask))
        .expect("fixture: the row is listed");
    let text = pane(&s, &looking_at(section, &row.key));
    assert!(!text.contains(": need"), "nothing to state:\n{text}");
}
