//! What a neighbour's trade row says it offers: learned goods, the rest
//! counted by a reserved colony or named by a warm one.
//!
//! @spec TUI-NAV-162

use std::collections::BTreeSet;

use clowder_core::{ColonyId, Event, GameState, IdeaId, ResourceId};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::Terminal;

use clowder_tui::nav::{RowKey, Section, ViewState};
use clowder_tui::render::widgets::inspector;

const W: u16 = 120;
const H: u16 = 40;

fn lines(buf: &Buffer, rect: Rect) -> Vec<String> {
    (rect.y..rect.bottom())
        .map(|y| {
            (rect.x..rect.right())
                .map(|x| buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "))
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect()
}

fn trade_lines(state: &GameState, c: ColonyId) -> Vec<String> {
    let mut view = ViewState {
        section: Some(Section::Friends),
        ..ViewState::default()
    };
    view.focus[Section::Friends.index()] = Some(RowKey::Colony(c, "trade"));
    let rect = Rect::new(0, 0, W, H);
    let mut term = Terminal::new(TestBackend::new(W, H)).expect("TestBackend is infallible");
    term.draw(|f| inspector::draw(f, rect, state, &view))
        .expect("TestBackend is infallible");
    lines(term.backend().buffer(), rect)
}

/// A resource named as its stores row names it.
fn label(r: ResourceId) -> String {
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

/// A meadow trading with `c`, which has shown the clowder `learned`, or —
/// with `None` — no caravan has reached.
fn meadow(c: ColonyId, learned: Option<&[ResourceId]>, penpal: u32) -> GameState {
    let mut s = GameState::new(31);
    s.ideas.insert(IdeaId::GiftEconomy);
    s.discovered.insert(c);
    s.penpal[c.ix()] = penpal;
    if let Some(goods) = learned {
        s.learned_goods
            .insert(c, goods.iter().copied().collect::<BTreeSet<_>>());
    }
    s
}

/// The lines naming offers, in order.
fn offer_lines(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .filter(|l| {
            let t = l.trim_start();
            t.starts_with("Offers") || t.starts_with("Also mentions") || t.starts_with("Mentions")
        })
        .map(|l| l.trim().to_string())
        .collect()
}

/// Before any caravan has reached a colony, nothing is said of its goods.
/// @spec TUI-NAV-162
#[test]
fn an_unreached_colony_says_nothing_of_its_goods() {
    for c in [ColonyId::Otters, ColonyId::Raccoons, ColonyId::Foxes] {
        let s = meadow(c, None, 0);
        assert!(
            offer_lines(&trade_lines(&s, c)).is_empty(),
            "{c:?}: nothing is known before a caravan reaches it"
        );
    }
}

/// A reserved colony lists what it has shown, and counts the rest.
/// @spec TUI-NAV-162
#[test]
fn a_reserved_colony_lists_what_it_showed_and_counts_the_rest() {
    let c = ColonyId::Raccoons;
    let s0 = meadow(c, Some(&[]), 0);
    assert!(
        s0.standing(c) <= 0.0,
        "fixture: a colony that does not name what it keeps"
    );
    let goods: Vec<ResourceId> = s0
        .neighbour_offers(c)
        .unseen
        .iter()
        .map(|(g, _)| *g)
        .collect();
    assert_eq!(goods.len(), 3, "fixture: three goods");
    let (a, b, z) = (label(goods[0]), label(goods[1]), label(goods[2]));

    let cases: Vec<(Vec<ResourceId>, String)> = vec![
        (vec![], "Offers: 3 goods you haven't seen yet.".to_string()),
        (vec![goods[0]], format!("Offers: {a} and 2 more.")),
        (
            vec![goods[0], goods[1]],
            format!("Offers: {a}, {b}, and 1 more."),
        ),
        (goods.clone(), format!("Offers: {a}, {b}, and {z}.")),
    ];
    for (learned, want) in cases {
        let s = meadow(c, Some(&learned), 0);
        assert_eq!(offer_lines(&trade_lines(&s, c)), vec![want.clone()]);
    }

    // One good unseen and nothing learned: the singular.
    let d = ColonyId::Deer;
    let s = meadow(d, Some(&[]), 0);
    let offered = s.neighbour_offers(d);
    if !offered.tells_by_name && offered.unseen.len() == 1 {
        assert_eq!(
            offer_lines(&trade_lines(&s, d)),
            vec!["Offers: 1 good you haven't seen yet.".to_string()]
        );
    }
}

/// A warm colony names what it keeps, marking what it saves for closer
/// friends; with nothing shown yet it only mentions.
/// @spec TUI-NAV-162
#[test]
fn a_warm_colony_names_what_it_keeps() {
    let c = ColonyId::Otters;
    let s0 = meadow(c, Some(&[]), 0);
    assert!(s0.standing(c) > 0.0, "fixture: a warm colony");
    let goods: Vec<ResourceId> = s0
        .neighbour_offers(c)
        .unseen
        .iter()
        .map(|(g, _)| *g)
        .collect();
    assert_eq!(goods.len(), 2, "fixture: two goods");
    let (a, b) = (label(goods[0]), label(goods[1]));

    assert_eq!(
        offer_lines(&trade_lines(&s0, c)),
        vec![format!("Mentions: {a} and {b}.")]
    );
    let s = meadow(c, Some(&[goods[0]]), 0);
    assert_eq!(
        offer_lines(&trade_lines(&s, c)),
        vec![format!("Offers: {a}."), format!("Also mentions: {b}.")]
    );
    let s = meadow(c, Some(&goods), 0);
    assert_eq!(
        offer_lines(&trade_lines(&s, c)),
        vec![format!("Offers: {a} and {b}.")]
    );

    // A colony warmed by letters, keeping a good for closer friends still.
    let (d, gated) = ColonyId::ALL
        .into_iter()
        .filter(|c| *c != ColonyId::StarWhales)
        .find_map(|c| {
            clowder_core::content::colony_def(c)
                .sells
                .iter()
                .find(|g| g.min_penpal > 1)
                .map(|g| (c, g.resource))
        })
        .expect("fixture: a good for closer friends");
    let s = meadow(d, Some(&[]), 1);
    let offered = s.neighbour_offers(d);
    assert!(offered.tells_by_name, "fixture: one letter warms {d:?}");
    let named: Vec<String> = offered
        .unseen
        .iter()
        .map(|(g, now)| {
            if *now {
                label(*g)
            } else {
                format!("{} (for closer friends)", label(*g))
            }
        })
        .collect();
    assert!(
        named.contains(&format!("{} (for closer friends)", label(gated))),
        "fixture: {gated:?} is for closer friends"
    );
    let joined = match named.len() {
        1 => named[0].clone(),
        2 => format!("{} and {}", named[0], named[1]),
        _ => panic!("fixture: one or two goods"),
    };
    assert_eq!(
        offer_lines(&trade_lines(&s, d)),
        vec![format!("Mentions: {joined}.")]
    );
}

/// The offers stand directly beneath the price lines and above the last-time
/// history.
/// @spec TUI-NAV-162
#[test]
fn offers_stand_beneath_the_price_and_above_the_history() {
    let c = ColonyId::Raccoons;
    let mut s = meadow(c, Some(&[ResourceId::Catnip]), 0);
    s.journal.record(
        5,
        &Event::TradeReturned {
            colony: c,
            caravans: 1,
            napped: 0,
            goods: vec![(ResourceId::Catnip, 100.0)],
        },
    );
    let all = trade_lines(&s, c);
    let at = |needle: &str| {
        all.iter()
            .position(|l| l.contains(needle))
            .unwrap_or_else(|| panic!("{needle:?} in:\n{}", all.join("\n")))
    };
    let price = at("need");
    let offers = at("Offers:");
    let history = at("last time");
    assert!(price < offers && offers < history, "{}", all.join("\n"));
    let last_price = all
        .iter()
        .rposition(|l| l.contains("need"))
        .expect("price lines");
    assert_eq!(offers, last_price + 1, "directly beneath the price lines");
}
