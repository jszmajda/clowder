//! Tests-first for what a neighbour has shown the clowder:
//!
//!   * SIM-TRADE-010 — a caravan that does not nap reaches its colony and
//!     teaches every good its draws bring home.
//!   * SIM-TRADE-011 — a Great Meadow Move forgets it all.
//!   * SIM-TRADE-012 — the offers query: reach, learned goods, unseen goods
//!     each with whether they are offered now, and whether the colony tells
//!     of them by name.
//!   * SAVE-026 — reach and learned goods are saved, a v6 save migrates with
//!     none, and stray entries are dropped on load.

mod common;

use std::collections::{BTreeMap, BTreeSet};

use clowder_core::*;
use common::*;

/// A meadow that can afford plenty of caravans to every tradeable colony.
fn trading_meadow(seed: u64, colonies: &[ColonyId]) -> GameState {
    let mut s = GameState::new(seed);
    s.ideas.insert(IdeaId::GiftEconomy);
    for c in colonies {
        s.discovered.insert(*c);
    }
    // Storage everywhere, so stocks can pay for many caravans.
    grant(&mut s, StructureId::Shed, 200);
    grant(&mut s, StructureId::Porch, 200);
    grant(&mut s, StructureId::SunbeamAltar, 50);
    s
}

/// Refill what every caravan and buy-price costs.
fn refill(s: &mut GameState) {
    for r in ResourceId::ALL {
        let cap = s.cap(r);
        let fill = if cap.is_finite() && cap > 0.0 {
            cap
        } else {
            1.0e9
        };
        give(s, r, fill);
    }
}

fn goods_of(events: &[Event]) -> (u32, u32, BTreeSet<ResourceId>) {
    for e in events {
        if let Event::TradeReturned {
            caravans,
            napped,
            goods,
            ..
        } = e
        {
            return (*caravans, *napped, goods.iter().map(|(r, _)| *r).collect());
        }
    }
    panic!("an accepted trade reports what came home");
}

// ---------------------------------------------------------------------------
// SIM-TRADE-010 — what a caravan teaches
// ---------------------------------------------------------------------------

/// One caravan at a time: a caravan that reaches its colony records it
/// reached, and teaches exactly the goods it brought home; learning only
/// grows.
/// @spec SIM-TRADE-010
#[test]
fn a_reaching_caravan_teaches_exactly_what_it_brings_home() {
    for c in [
        ColonyId::Otters,
        ColonyId::Raccoons,
        ColonyId::Hedgehogs,
        ColonyId::Rabbits,
        ColonyId::Deer,
    ] {
        let mut s = trading_meadow(1010 + c as u64, &[c]);
        assert!(s.neighbour_offers(c).learned.is_empty());
        assert!(!s.neighbour_offers(c).reached);
        let mut seen: BTreeSet<ResourceId> = BTreeSet::new();
        for _ in 0..40 {
            refill(&mut s);
            let events = s.apply(Command::Trade(c, 1)).unwrap();
            let (_, napped, goods) = goods_of(&events);
            assert_eq!(napped, 0, "{c:?} is not shy, so never naps");
            seen.extend(goods);
            let offers = s.neighbour_offers(c);
            assert!(offers.reached, "{c:?}: a caravan reached them");
            let learned: BTreeSet<ResourceId> = offers.learned.iter().copied().collect();
            assert_eq!(learned, seen, "{c:?}: learned is what came home, no more");
        }
    }
}

/// Foxes' moonstone is learned only on a caravan whose own roll brought some.
/// @spec SIM-TRADE-010
#[test]
fn foxes_moonstone_is_learned_only_when_it_comes_home() {
    let mut learned_without = false;
    let mut learned_with = false;
    for seed in 0..200u64 {
        let mut s = trading_meadow(2000 + seed, &[ColonyId::Foxes]);
        refill(&mut s);
        give(&mut s, ResourceId::Kites, 0.0);
        let events = s.apply(Command::Trade(ColonyId::Foxes, 1)).unwrap();
        let (_, napped, goods) = goods_of(&events);
        let learned = s.neighbour_offers(ColonyId::Foxes).learned;
        assert_eq!(
            learned.contains(&ResourceId::Moonstone),
            goods.contains(&ResourceId::Moonstone),
            "seed {seed}: moonstone learned exactly when it came home"
        );
        if napped == 0 {
            if goods.contains(&ResourceId::Moonstone) {
                learned_with = true;
            } else {
                learned_without = true;
            }
        }
    }
    assert!(
        learned_with && learned_without,
        "fixture: both outcomes seen"
    );
}

/// A shy colony's napping caravan teaches nothing and reaches no one; the
/// Star Whale's exchange records nothing at all.
/// @spec SIM-TRADE-010
#[test]
fn a_napping_caravan_and_the_whale_teach_nothing() {
    let mut napped_alone = false;
    for seed in 0..300u64 {
        let mut s = trading_meadow(3000 + seed, &[ColonyId::Foxes]);
        refill(&mut s);
        let events = s.apply(Command::Trade(ColonyId::Foxes, 1)).unwrap();
        let (_, napped, _) = goods_of(&events);
        if napped == 1 {
            napped_alone = true;
            let offers = s.neighbour_offers(ColonyId::Foxes);
            assert!(!offers.reached, "seed {seed}: a nap reaches no one");
            assert!(
                offers.learned.is_empty(),
                "seed {seed}: a nap teaches nothing"
            );
        }
    }
    assert!(napped_alone, "fixture: a shy colony's lone caravan napped");

    let mut w = trading_meadow(3999, &[]);
    w.whale_visit_years = 3;
    refill(&mut w);
    w.apply(Command::Trade(ColonyId::StarWhales, 1)).unwrap();
    assert_eq!(
        w.neighbour_offers(ColonyId::StarWhales),
        Offers::default(),
        "the whale's exchange records nothing"
    );
}

/// A caravan that reaches a colony and draws nothing has still met it.
/// @spec SIM-TRADE-010
#[test]
fn a_caravan_that_draws_nothing_still_reaches() {
    let mut seen = false;
    for seed in 0..2000u64 {
        let mut s = trading_meadow(4000 + seed, &[ColonyId::Deer]);
        refill(&mut s);
        let events = s.apply(Command::Trade(ColonyId::Deer, 1)).unwrap();
        let (_, napped, goods) = goods_of(&events);
        if napped == 0 && goods.is_empty() {
            seen = true;
            let offers = s.neighbour_offers(ColonyId::Deer);
            assert!(offers.reached, "seed {seed}: met, though nothing was drawn");
            assert!(offers.learned.is_empty());
            break;
        }
    }
    assert!(seen, "fixture: some Deer caravan drew nothing");
}

// ---------------------------------------------------------------------------
// SIM-TRADE-011 — the Move forgets
// ---------------------------------------------------------------------------

fn kitten(i: usize) -> Kitten {
    Kitten {
        name: format!("Kit{i:03}"),
        job: None,
        location: Location::Home,
        arrived_day: 0,
        job_seq: None,
        away_seq: None,
        skills: [0.0; 8],
        exp: 0.0,
        rank: 0,
        trait_id: KittenTrait::None,
        tinkering: None,
    }
}

/// @spec SIM-TRADE-011
#[test]
fn a_move_forgets_every_colonys_reach_and_learned_goods() {
    let mut s = trading_meadow(5000, &[ColonyId::Otters, ColonyId::Rabbits]);
    for c in [ColonyId::Otters, ColonyId::Rabbits] {
        refill(&mut s);
        s.apply(Command::Trade(c, 1)).unwrap();
    }
    assert!(s.neighbour_offers(ColonyId::Otters).reached);
    s.kittens = (0..71).map(kitten).collect();
    s.apply(Command::GreatMeadowMove).unwrap();
    for c in ColonyId::ALL {
        assert_eq!(
            s.neighbour_offers(c),
            Offers::default(),
            "{c:?}: the new meadow's neighbours are met afresh"
        );
    }
    assert!(s.learned_goods.is_empty());
}

// ---------------------------------------------------------------------------
// SIM-TRADE-012 — the offers query
// ---------------------------------------------------------------------------

/// Every good a colony offers, in its table's order, then Foxes' moonstone.
fn offered(c: ColonyId) -> Vec<ResourceId> {
    let mut out: Vec<ResourceId> = content::colony_def(c)
        .sells
        .iter()
        .map(|g| g.resource)
        .collect();
    if c == ColonyId::Foxes {
        out.push(ResourceId::Moonstone);
    }
    out
}

/// The query splits a colony's goods into learned and unseen in the colony's
/// own order, marks each unseen good with whether its Pen Pal level is met —
/// moonstone wanting none — and reports the colony names its unseen goods
/// exactly while its standing is above zero.
/// @spec SIM-TRADE-012
#[test]
fn the_offers_query_splits_goods_in_order_and_follows_standing() {
    for c in ColonyId::ALL {
        if c == ColonyId::StarWhales {
            continue;
        }
        let goods = offered(c);
        let mut s = trading_meadow(6000 + c as u64, &[c]);
        assert_eq!(s.neighbour_offers(c), Offers::default(), "{c:?}: unmet");

        // Learn every other good, by hand.
        let learned: Vec<ResourceId> = goods.iter().copied().step_by(2).collect();
        s.learned_goods
            .insert(c, learned.iter().copied().collect::<BTreeSet<_>>());
        for level in [0u32, 5, 9] {
            s.penpal[c.ix()] = level;
            let offers = s.neighbour_offers(c);
            assert!(offers.reached);
            assert_eq!(offers.learned, learned, "{c:?}: learned, in order");
            let unseen: Vec<(ResourceId, bool)> = goods
                .iter()
                .copied()
                .filter(|g| !learned.contains(g))
                .map(|g| {
                    let min = content::colony_def(c)
                        .sells
                        .iter()
                        .find(|x| x.resource == g)
                        .map(|x| x.min_penpal)
                        .unwrap_or(0);
                    (g, level >= min)
                })
                .collect();
            assert_eq!(
                offers.unseen, unseen,
                "{c:?} at level {level}: unseen, in order"
            );
            assert_eq!(
                offers.tells_by_name,
                s.standing(c) > 0.0,
                "{c:?} at level {level}: names unseen goods exactly while warm"
            );
        }
    }
}

/// Some colony keeps a good for closer friends, and warming shows it.
/// @spec SIM-TRADE-012
#[test]
fn a_good_for_closer_friends_is_offered_once_the_level_is_met() {
    let (c, good, min) = ColonyId::ALL
        .into_iter()
        .filter(|c| *c != ColonyId::StarWhales)
        .find_map(|c| {
            content::colony_def(c)
                .sells
                .iter()
                .find(|g| g.min_penpal > 0)
                .map(|g| (c, g.resource, g.min_penpal))
        })
        .expect("fixture: some colony keeps a good for closer friends");
    let mut s = trading_meadow(6500, &[c]);
    s.learned_goods.insert(c, BTreeSet::new());
    let at = |s: &GameState| {
        s.neighbour_offers(c)
            .unseen
            .iter()
            .find(|(g, _)| *g == good)
            .map(|(_, now)| *now)
            .expect("the good is unseen")
    };
    s.penpal[c.ix()] = min - 1;
    assert!(!at(&s), "below its level the good is for closer friends");
    s.penpal[c.ix()] = min;
    assert!(at(&s), "at its level it is offered now");
}

// ---------------------------------------------------------------------------
// SAVE-026 — saved, migrated, normalized
// ---------------------------------------------------------------------------

const V6_FIXTURE: &str = include_str!("fixtures/save_v6.json");

/// Reach and learned goods survive a save and a load, under a version newer
/// than v6.
/// @spec SAVE-026
#[test]
fn reach_and_learned_goods_are_saved() {
    const { assert!(SAVE_VERSION >= 7, "a version step carries them") };
    let mut s = trading_meadow(7000, &[ColonyId::Otters, ColonyId::Deer]);
    s.learned_goods
        .insert(ColonyId::Otters, [ResourceId::Twigs].into_iter().collect());
    s.learned_goods.insert(ColonyId::Deer, BTreeSet::new());
    let json = save_string(&s, 1_000);
    let back = load_string(&json, 1_000).unwrap().state;
    assert_eq!(back.learned_goods, s.learned_goods);
    assert!(back.neighbour_offers(ColonyId::Deer).reached);
    assert!(back.neighbour_offers(ColonyId::Deer).learned.is_empty());
}

/// A v6 save migrates with no colony reached and no good learned, though its
/// journal holds trades.
/// @spec SAVE-026, SAVE-013
#[test]
fn a_v6_save_migrates_with_nothing_learned() {
    assert!(V6_FIXTURE.contains("\"save_version\": 6"));
    assert!(
        V6_FIXTURE.contains("TradeReturned"),
        "fixture: its journal holds trades, which are not mined"
    );
    let s = load_string(V6_FIXTURE, 1_760_303_600).unwrap().state;
    assert!(!s.discovered.is_empty(), "fixture: colonies are discovered");
    assert!(s.learned_goods.is_empty());
    for c in ColonyId::ALL {
        assert_eq!(s.neighbour_offers(c), Offers::default(), "{c:?}");
    }
}

/// Stray entries — the whale, an undiscovered colony, a good the colony does
/// not offer — are dropped on load, the rest kept.
/// @spec SAVE-026
#[test]
fn stray_learned_entries_are_dropped_on_load() {
    let mut s = trading_meadow(7100, &[ColonyId::Otters]);
    let mut learned: BTreeMap<ColonyId, BTreeSet<ResourceId>> = BTreeMap::new();
    learned.insert(
        ColonyId::Otters,
        [ResourceId::Twigs, ResourceId::Stardust]
            .into_iter()
            .collect(),
    );
    learned.insert(
        ColonyId::Deer,
        [ResourceId::Moonstone].into_iter().collect(),
    );
    learned.insert(ColonyId::StarWhales, BTreeSet::new());
    s.learned_goods = learned;
    let back = load_string(&save_string(&s, 1_000), 1_000).unwrap().state;
    let mut want: BTreeMap<ColonyId, BTreeSet<ResourceId>> = BTreeMap::new();
    want.insert(ColonyId::Otters, [ResourceId::Twigs].into_iter().collect());
    assert_eq!(back.learned_goods, want);
}

/// Every field a v6 save holds survives migration exactly; the one field it
/// gains is what each colony has shown, empty.
/// @spec SAVE-026, SAVE-013
#[test]
fn a_v6_save_migrates_with_everything_else_exact() {
    let before: serde_json::Value = serde_json::from_str(V6_FIXTURE).unwrap();
    let at = before["saved_at_unix"].as_u64().unwrap();
    let sd = begin_load(V6_FIXTURE, at)
        .expect("a v6 save loads")
        .set_down();
    let resaved: serde_json::Value = serde_json::from_str(&save_string_carrying(
        &sd.state,
        at,
        sd.owed_ticks,
        Some(&sd.report_so_far),
    ))
    .unwrap();
    assert_eq!(resaved["save_version"], SAVE_VERSION);
    for (key, was) in before["state"].as_object().unwrap() {
        assert_eq!(
            was, &resaved["state"][key],
            "migrating a v6 save changed `{key}`"
        );
    }
    let added: Vec<&String> = resaved["state"]
        .as_object()
        .unwrap()
        .keys()
        .filter(|k| !before["state"].as_object().unwrap().contains_key(*k))
        .collect();
    assert_eq!(added, vec!["learned_goods"]);
    assert!(resaved["state"]["learned_goods"]
        .as_object()
        .is_some_and(|m| m.is_empty()));
}
