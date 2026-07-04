//! Shared scenario helpers. Tests manipulate public state fields directly to
//! construct scenarios, then assert on behavior through tick()/apply().
#![allow(dead_code)]

use clowder_core::*;

pub fn run_ticks(s: &mut GameState, n: u64) -> Vec<Event> {
    (0..n).flat_map(|_| s.tick()).collect()
}

/// Run whole days. Assumes (and asserts) day-aligned ticks so boundary
/// processing lands predictably.
pub fn run_days(s: &mut GameState, days: u64) -> Vec<Event> {
    assert_eq!(s.ticks % 10, 0, "run_days requires day-aligned state");
    run_ticks(s, days * 10)
}

pub fn give(s: &mut GameState, r: ResourceId, amount: f64) {
    s.resources[r.ix()] = amount;
}

pub fn grant(s: &mut GameState, st: StructureId, n: u32) {
    s.structures[st.ix()] += n;
}

/// Adopt exactly `n` kittens by forcing pity-guaranteed arrivals, one per day.
/// Ensures housing and food eligibility; leaves stock wherever the sim put it.
pub fn adopt(s: &mut GameState, n: usize) {
    for _ in 0..n {
        while s.free_slots() == 0 {
            grant(s, StructureId::CozyCushion, 1);
        }
        let stock = s.stock(ResourceId::Catnip);
        give(s, ResourceId::Catnip, stock.max(100.0));
        s.pity = 4;
        let before = s.kittens.len();
        let events = run_days(s, 1);
        assert_eq!(
            s.kittens.len(),
            before + 1,
            "pity-forced arrival must land in one day; events: {events:?}"
        );
    }
}

pub fn names_of(events: &[Event]) -> Vec<String> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::KittenArrived { name } => Some(name.clone()),
            _ => None,
        })
        .collect()
}
