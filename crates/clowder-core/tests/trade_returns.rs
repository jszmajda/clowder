//! What a trade brings home: the TradeReturned event carries the caravans
//! that napped and each store the trade raised, by the amount it rose.
//! Written tests-first from SIM-TRADE-009 and SAVE-025.

mod common;

use clowder_core::*;
use common::*;

fn goods_of(events: &[Event]) -> (ColonyId, u32, u32, Vec<(ResourceId, f64)>) {
    let mut found = events.iter().filter_map(|e| match e {
        Event::TradeReturned {
            colony,
            caravans,
            napped,
            goods,
        } => Some((*colony, *caravans, *napped, goods.clone())),
        _ => None,
    });
    let one = found.next().expect("a TradeReturned event");
    assert!(found.next().is_none(), "exactly one TradeReturned event");
    one
}

/// Every stock the trade raised, by how far it rose, in resource order —
/// read off the meadow itself, before and after.
fn raised(before: &GameState, after: &GameState) -> Vec<(ResourceId, f64)> {
    ResourceId::ALL
        .into_iter()
        .map(|r| (r, after.stock(r) - before.stock(r)))
        .filter(|(_, d)| *d > 0.0)
        .collect()
}

/// A meadow ready to trade `n` caravans with `colony`, its costs paid in full.
fn trader(seed: u64, colony: ColonyId, n: u32) -> GameState {
    let mut s = GameState::new(seed);
    s.discovered.insert(colony);
    let def = content::colony_def(colony);
    let n = n as f64;
    give(&mut s, ResourceId::Pep, content::TRADE_PEP_PER_CARAVAN * n);
    give(
        &mut s,
        ResourceId::Shinies,
        content::TRADE_SHINIES_PER_CARAVAN * n,
    );
    give(&mut s, def.buy.0, def.buy.1 * n);
    s
}

// @spec SIM-TRADE-009
#[test]
fn a_trade_reports_each_store_it_raised_by_the_amount_it_rose() {
    let mut s = trader(901, ColonyId::Otters, 40);
    // The costs are spent before anything comes home, so measure from there.
    let mut paid = s.clone();
    let def = content::colony_def(ColonyId::Otters);
    give(&mut paid, ResourceId::Pep, 0.0);
    give(&mut paid, ResourceId::Shinies, 0.0);
    give(&mut paid, def.buy.0, 0.0);

    let events = s.apply(Command::Trade(ColonyId::Otters, 40)).unwrap();
    let (colony, caravans, napped, goods) = goods_of(&events);
    assert_eq!(colony, ColonyId::Otters);
    assert_eq!(caravans, 40);
    assert_eq!(napped, 0, "a warm colony's caravans never nap");
    assert!(
        !goods.is_empty(),
        "forty warm caravans bring something home"
    );
    assert_eq!(
        goods,
        raised(&paid, &s),
        "each store by exactly how far it rose"
    );
}

// @spec SIM-TRADE-009
#[test]
fn a_trade_counts_the_caravans_that_napped() {
    // A shy colony naps some caravans and trades the rest.
    let mut s = trader(902, ColonyId::Owls, 200);
    let events = s.apply(Command::Trade(ColonyId::Owls, 200)).unwrap();
    let (_, caravans, napped, goods) = goods_of(&events);
    assert_eq!(caravans, 200);
    assert!(
        (1..200).contains(&napped),
        "a shy colony naps some caravans, not all: {napped}"
    );
    assert!(goods.iter().all(|(_, a)| *a > 0.0), "only stores that rose");
}

// @spec SIM-TRADE-009
#[test]
fn the_whales_exchange_reports_what_it_brought_as_the_stocks_moved() {
    let mut s = GameState::new(903);
    s.whale_visit_years = 5;
    s.discovered.insert(ColonyId::StarWhales);
    give(&mut s, ResourceId::Hushfelt, 1e6);
    let mut paid = s.clone();
    let events = s.apply(Command::Trade(ColonyId::StarWhales, 3)).unwrap();
    give(
        &mut paid,
        ResourceId::Hushfelt,
        s.stock(ResourceId::Hushfelt),
    );
    let (colony, caravans, napped, goods) = goods_of(&events);
    assert_eq!(colony, ColonyId::StarWhales);
    assert_eq!((caravans, napped), (3, 0));
    assert_eq!(goods, raised(&paid, &s));
}

// @spec SAVE-025
#[test]
fn a_trade_entry_written_before_its_goods_loads_with_none() {
    let old = r#"{ "TradeReturned": { "colony": "Otters", "caravans": 4 } }"#;
    let event: Event = serde_json::from_str(old).expect("an old entry still loads");
    assert_eq!(
        event,
        Event::TradeReturned {
            colony: ColonyId::Otters,
            caravans: 4,
            napped: 0,
            goods: vec![],
        }
    );
}
