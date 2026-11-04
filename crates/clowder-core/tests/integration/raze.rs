//! Taking a copy of a structure down: what it returns, what it leaves alone, and
//! what the core reports before it happens.
//!
//! @spec SIM-ECO-065, SIM-ECO-066, SIM-ENG-086, SAVE-040

use crate::common;

use clowder_core::*;
use common::*;

/// A meadow with `copies` of a structure and all stores empty.
fn standing(st: StructureId, copies: u32, seed: u64) -> GameState {
    let mut s = GameState::new(seed);
    s.ideas = IdeaId::ALL.into_iter().collect();
    s.sites_reached = SiteId::ALL.into_iter().collect();
    s.structures[st.ix()] = copies;
    for r in ResourceId::ALL {
        s.resources[r.ix()] = 0.0;
    }
    s
}

/// What a copy would cost to build back with `copies` standing.
fn price_at(st: StructureId, copies: u32) -> Vec<(ResourceId, f64)> {
    standing(st, copies, 1).next_cost(st)
}

/// One copy comes down and a quarter of what it would cost to build back comes
/// home, up to each store's cap with the copy gone, for every structure.
///
/// @spec SIM-ECO-065
#[test]
fn a_copy_comes_down_and_returns_a_quarter_of_its_price_to_build_back() {
    for st in StructureId::ALL {
        let mut s = standing(st, 3, 11);
        let events = s.apply(Command::Raze(st)).unwrap();
        assert_eq!(s.structures[st.ix()], 2, "{st:?}: one copy comes down");
        assert_eq!(
            events.first(),
            Some(&Event::StructureRazed { id: st, count: 2 }),
            "{st:?}: what is told"
        );
        for (r, price) in price_at(st, 2) {
            let want = (price * 0.25).min(s.cap(r));
            assert!(
                (s.stock(r) - want).abs() < 1e-9,
                "{st:?} returned {} of {r:?}, wanted {want}",
                s.stock(r)
            );
        }
    }
}

/// A return reaches only as far as the store's cap with the copy gone, and never
/// lowers a stock that already stands above it.
///
/// @spec SIM-ECO-065
#[test]
fn a_return_stops_at_the_cap_with_the_copy_gone_and_lowers_nothing() {
    let (st, r, price) = StructureId::ALL
        .into_iter()
        .find_map(|st| {
            let (r, price) = *price_at(st, 2).iter().find(|(r, price)| {
                let cap = standing(st, 2, 1).cap(*r);
                cap.is_finite() && cap > 0.0 && price * 0.25 > 0.0
            })?;
            Some((st, r, price))
        })
        .expect("fixture: a structure priced in a store with a cap");
    let room = 0.1 * price * 0.25;

    let mut near = standing(st, 3, 12);
    let cap = standing(st, 2, 1).cap(r);
    near.resources[r.ix()] = cap - room;
    near.apply(Command::Raze(st)).unwrap();
    assert!(
        (near.stock(r) - cap).abs() < 1e-9,
        "the return stops at the cap: {} of {cap}",
        near.stock(r)
    );

    let mut above = standing(st, 3, 13);
    above.resources[r.ix()] = cap * 2.0 + 1.0;
    above.apply(Command::Raze(st)).unwrap();
    assert_eq!(above.stock(r), cap * 2.0 + 1.0, "a stock is never lowered");
}

/// With no copy standing, the command is refused and nothing changes.
///
/// @spec SIM-ECO-065
#[test]
fn with_no_copy_standing_it_is_refused() {
    let mut s = standing(StructureId::ALL[0], 0, 14);
    let before = s.clone();
    assert_eq!(
        s.apply(Command::Raze(StructureId::ALL[0])).unwrap_err(),
        CommandError::NothingToRaze
    );
    assert_eq!(s, before);
}

/// The price comes back down with the count, the structure stays revealed, and
/// building and taking down again can never gain anything, for every structure.
///
/// @spec SIM-ECO-065
#[test]
fn building_one_and_taking_it_down_never_gains_anything() {
    for st in StructureId::ALL {
        let mut s = standing(st, 3, 15);
        for (r, price) in price_at(st, 3) {
            s.resources[r.ix()] = price * 2.0;
        }
        let before: Vec<f64> = ResourceId::ALL.iter().map(|r| s.stock(*r)).collect();
        s.apply(Command::Build(st)).unwrap();
        s.apply(Command::Raze(st)).unwrap();
        assert_eq!(s.structures[st.ix()], 3, "{st:?}: back where it stood");
        assert_eq!(
            s.next_cost(st),
            price_at(st, 3),
            "{st:?}: the price with it"
        );
        for (i, r) in ResourceId::ALL.iter().enumerate() {
            assert!(
                s.stock(*r) <= before[i] + 1e-9,
                "{st:?} gained {r:?}: {} from {}",
                s.stock(*r),
                before[i]
            );
        }
    }
}

/// Taking one down draws nothing from the state RNG.
///
/// @spec SIM-ECO-065
#[test]
fn it_takes_no_draw_from_the_state_rng() {
    let mut s = standing(StructureId::ALL[0], 3, 16);
    let before = s.rng.clone();
    s.apply(Command::Raze(StructureId::ALL[0])).unwrap();
    assert_eq!(s.rng, before);
}

/// A kitten past the new room stays, and the next arrival waits: nobody is turned
/// out, whatever falls.
///
/// @spec SIM-ECO-066
#[test]
fn a_kitten_past_the_new_room_stays() {
    let mut s = GameState::new(21);
    adopt(&mut s, 4);
    s.apply(Command::AssignJob(JobId::Gardener)).unwrap();
    let homes = StructureId::ALL
        .into_iter()
        .filter(|st| content::structure_def(*st).housing > 0 && s.structures[st.ix()] > 0)
        .collect::<Vec<_>>();
    assert!(!homes.is_empty(), "fixture: the kittens have homes");
    let roster = s.kittens.clone();
    for st in homes {
        while s.structures[st.ix()] > 0 {
            s.apply(Command::Raze(st)).unwrap();
        }
    }
    assert_eq!(s.housing_capacity(), 0, "fixture: no room at all");
    assert_eq!(s.kittens, roster, "nobody is turned out");
    assert_eq!(s.free_slots(), 0, "the next arrival waits");
    give(&mut s, ResourceId::Catnip, 500.0);
    run_days(&mut s, 3);
    assert_eq!(
        s.kittens.len(),
        roster.len(),
        "and none arrives past the room"
    );
}

/// A tinkerer past the new seats stays bound, and the next binding waits.
///
/// @spec SIM-ECO-066
#[test]
fn a_tinkerer_past_the_new_seats_stays_bound() {
    let mut s = GameState::new(22);
    s.ideas = IdeaId::ALL.into_iter().collect();
    adopt(&mut s, 3);
    let seats = StructureId::ALL
        .into_iter()
        .find(|st| content::structure_def(*st).tinkerer_seats > 0)
        .expect("fixture: a structure gives seats");
    grant(&mut s, seats, 3);
    for _ in 0..3 {
        s.apply(Command::AssignJob(JobId::Tinkerer)).unwrap();
    }
    for _ in 0..2 {
        s.apply(Command::AssignTinkerer(CraftId::Wands)).unwrap();
    }
    let bound = |s: &GameState| s.kittens.iter().filter(|k| k.tinkering.is_some()).count();
    assert_eq!(bound(&s), 2, "fixture: two seated");

    while s.structures[seats.ix()] > 0 {
        s.apply(Command::Raze(seats)).unwrap();
    }
    assert_eq!(bound(&s), 2, "nobody is unbound");
    assert_eq!(
        s.apply(Command::AssignTinkerer(CraftId::Wands))
            .unwrap_err(),
        CommandError::NoSeats,
        "the next binding waits for a seat"
    );
}

/// The report is what applying the command then does, for every structure, and
/// asking changes nothing.
///
/// @spec SIM-ENG-086
#[test]
fn the_report_says_what_taking_one_down_would_do() {
    for st in StructureId::ALL {
        let mut s = standing(st, 3, 31);
        for r in ResourceId::ALL {
            let cap = s.cap(r);
            s.resources[r.ix()] = if cap.is_finite() { cap } else { 10.0 };
        }
        let before = s.clone();
        let report = s.raze_report(st).expect("a copy stands");
        assert_eq!(s, before, "{st:?}: asking changes nothing");
        assert_eq!(report.left, 2, "{st:?}: the copies left");

        s.apply(Command::Raze(st)).unwrap();
        for r in ResourceId::ALL {
            let returned = report
                .returned
                .iter()
                .find(|(x, _)| *x == r)
                .map_or(0.0, |(_, n)| *n);
            assert!(
                (s.stock(r) - before.stock(r) - returned).abs() < 1e-9,
                "{st:?}: the report returned {returned} of {r:?}"
            );
            let trimmed = report
                .trimmed
                .iter()
                .find(|(x, _)| *x == r)
                .map_or(0.0, |(_, n)| *n);
            let over = (s.stock(r) - s.cap(r)).max(0.0);
            assert!(
                (trimmed - over).abs() < 1e-9,
                "{st:?}: the report trims {trimmed} of {r:?}, and {over} stands over"
            );
            assert!(
                report.returned.iter().all(|(_, n)| *n > 0.0)
                    && report.trimmed.iter().all(|(_, n)| *n > 0.0),
                "{st:?}: nothing of no amount is reported"
            );
        }
    }
    assert!(
        StructureId::ALL.into_iter().any(|st| {
            let mut s = standing(st, 3, 32);
            for r in ResourceId::ALL {
                let cap = s.cap(r);
                s.resources[r.ix()] = if cap.is_finite() { cap } else { 10.0 };
            }
            !s.raze_report(st).expect("a copy stands").trimmed.is_empty()
        }),
        "fixture: taking some structure down trims a store"
    );
}

/// With no copy standing, there is nothing to report.
///
/// @spec SIM-ENG-086
#[test]
fn the_report_says_nothing_with_no_copy_standing() {
    let s = standing(StructureId::ALL[0], 0, 33);
    assert!(s.raze_report(StructureId::ALL[0]).is_none());
}

/// A store over its new cap is trimmed at the next clamp, as any is.
///
/// @spec SIM-ECO-065, SIM-ENG-086
#[test]
fn a_store_over_its_new_cap_is_trimmed_at_the_next_clamp() {
    let (st, r) = StructureId::ALL
        .into_iter()
        .find_map(|st| {
            let s = standing(st, 3, 41);
            let less = standing(st, 2, 41);
            ResourceId::ALL
                .into_iter()
                .find(|r| s.cap(*r).is_finite() && less.cap(*r) < s.cap(*r))
                .map(|r| (st, r))
        })
        .expect("fixture: a structure gives a cap");
    let mut s = standing(st, 3, 42);
    s.resources[r.ix()] = s.cap(r);
    s.apply(Command::Raze(st)).unwrap();
    s.tick();
    assert!(
        s.stock(r) <= s.cap(r) + 1e-9,
        "{r:?} stands at {} over a cap of {}",
        s.stock(r),
        s.cap(r)
    );
}

/// A journal holding a raze is saved at the version that says so, and loads.
///
/// @spec SAVE-040
#[test]
fn a_journal_holding_a_raze_saves_and_loads() {
    assert_eq!(SAVE_VERSION, 12, "the version that may hold it");
    let mut s = standing(StructureId::ALL[0], 3, 51);
    s.apply(Command::Raze(StructureId::ALL[0])).unwrap();
    let text = save_string(&s, 1_000);
    let loaded = load_string(&text, 1_000).expect("the save loads");
    assert!(
        text.contains("StructureRazed"),
        "fixture: the save holds the event"
    );
    assert_eq!(loaded.state, s, "the meadow comes back whole");
}

/// A v11 save migrates changing no field, and is saved back at the new version.
///
/// @spec SAVE-040
#[test]
fn a_v11_save_migrates_changing_no_field() {
    let fixture = include_str!("../fixtures/save_v11.json");
    let v11: serde_json::Value = serde_json::from_str(fixture).expect("the fixture parses");
    assert_eq!(v11["save_version"], 11, "fixture: a v11 save");
    let at = v11["saved_at_unix"].as_u64().expect("a stamp");

    let loaded = load_string(fixture, at).expect("the save loads");
    let resaved: serde_json::Value =
        serde_json::from_str(&save_string(&loaded.state, at)).expect("the save parses");
    assert_eq!(resaved["save_version"], 12);
    assert_eq!(
        resaved["state"], v11["state"],
        "no field of the meadow moved"
    );
}
