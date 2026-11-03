//! What a trade brings home: the TradeReturned event carries the caravans
//! that napped and each store the trade raised, by the amount it rose.
//! Written tests-first from SIM-TRADE-009 and SAVE-025.

use crate::common;

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

// =====================================================================
// SIM-TRADE-014 — a napping colony's courtesy gift
// =====================================================================

/// A caravan at a time, over enough meadows to see every outcome: a nap brings
/// home nothing, or a tenth of the colony's first good and the hello it counts
/// as; and about one nap in four does the latter.
// @spec SIM-TRADE-010, SIM-TRADE-014
#[test]
fn a_napping_caravan_sometimes_brings_home_a_tenth_of_the_first_good() {
    for (colony, good, amount) in [
        (ColonyId::Owls, ResourceId::Curiosity, 250.0),
        (ColonyId::Foxes, ResourceId::Buttons, 25.0),
    ] {
        let (mut gifts, mut naps) = (0, 0);
        for seed in 0..6_000 {
            let mut s = trader(seed, colony, 1);
            let events = s.apply(Command::Trade(colony, 1)).unwrap();
            let (_, _, napped, goods) = goods_of(&events);
            if napped == 0 {
                continue;
            }
            naps += 1;
            let offers = s.neighbour_offers(colony);
            if goods.is_empty() {
                assert!(!offers.reached, "no gift: a napping colony said nothing");
                assert!(offers.learned.is_empty());
            } else {
                gifts += 1;
                assert_eq!(
                    goods,
                    [(good, amount)],
                    "a tenth of the first good, no more"
                );
                assert!(
                    offers.reached,
                    "a colony that left a gift out has said hello"
                );
                assert_eq!(
                    offers.learned,
                    [good],
                    "and what it left is a good brought home"
                );
            }
        }
        let rate = gifts as f64 / naps as f64;
        assert!(naps > 300, "fixture: enough naps to judge, had {naps}");
        assert!(
            (0.2..0.3).contains(&rate),
            "about one nap in four, measured {gifts} of {naps}"
        );
    }
}

/// The gift's draw comes directly after the nap's own, and only on a nap: a twin
/// of the generator, walking the rule's own draws, ends where the trade does and
/// comes to the same stock.
// @spec SIM-TRADE-002, SIM-TRADE-014
#[test]
fn a_gifts_draw_follows_the_naps_own() {
    let colony = ColonyId::Owls;
    let mut s = trader(904, colony, 2_000);
    let def = content::colony_def(colony);
    let standing = s.standing(colony);
    assert!(standing < 0.0, "fixture: a shy colony");
    let mut twin = s.rng.clone();
    let mut expected = 0.0;
    for _ in 0..2_000 {
        if !twin.chance(1.0 + standing) {
            if twin.chance(content::NAP_GIFT_CHANCE) {
                expected += def.sells[0].amount * content::NAP_GIFT_SHARE;
            }
            continue;
        }
        for sell in def.sells {
            if twin.chance(sell.chance) {
                let fuzz = 1.0 + (twin.uniform() * 2.0 - 1.0) * content::TRADE_FUZZ_WIDTH;
                expected += sell.amount * fuzz;
            }
        }
    }
    let before = s.stock(def.sells[0].resource);
    s.apply(Command::Trade(colony, 2_000)).unwrap();
    assert_eq!(s.rng, twin, "the same draws, in the same order");
    let rose = s.stock(def.sells[0].resource) - before;
    assert!(
        (rose - expected).abs() < 1e-6,
        "{rose} where the rule makes {expected}"
    );
}

/// A gift is no trade: what raises a trade's goods leaves it where it is.
// @spec SIM-TRADE-014
#[test]
fn a_gift_takes_nothing_that_raises_a_trade() {
    let raiser = StructureId::ALL
        .into_iter()
        .find(|s| content::structure_def(*s).trade_ratio > 0.0)
        .expect("fixture: something raises the trade ratio");
    let mut seen = false;
    for seed in 0..6_000 {
        let mut s = trader(seed, ColonyId::Owls, 1);
        grant(&mut s, raiser, 5);
        let events = s.apply(Command::Trade(ColonyId::Owls, 1)).unwrap();
        let (_, _, napped, goods) = goods_of(&events);
        if napped == 1 && !goods.is_empty() {
            assert_eq!(goods, [(ResourceId::Curiosity, 250.0)]);
            seen = true;
        }
    }
    assert!(seen, "fixture: some nap brought a gift");
}

/// The chance and the share the design states.
// @spec SIM-TRADE-014
#[test]
fn a_gift_is_a_tenth_at_one_nap_in_four() {
    assert_eq!(content::NAP_GIFT_CHANCE, 0.25);
    assert_eq!(content::NAP_GIFT_SHARE, 0.1);
}

/// A gift comes from what a colony's Pen Pal level opens: every colony that can
/// nap has a good open from the first letter, so a gift always has something to
/// be. Standing never falls below where a colony starts.
// @spec SIM-TRADE-014
#[test]
fn every_colony_that_can_nap_has_a_good_to_give() {
    for colony in ColonyId::ALL {
        let def = content::colony_def(colony);
        if colony != ColonyId::StarWhales && def.start_standing < 0.0 {
            assert!(
                def.sells.iter().any(|sell| sell.min_penpal == 0),
                "{colony:?} can nap with nothing open to give"
            );
        }
    }
}
