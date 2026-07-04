mod common;

use clowder_core::*;
use common::*;

// @spec SIM-KIT-001
#[test]
fn the_roster_only_ever_grows() {
    let mut s = GameState::new(1);
    adopt(&mut s, 3);
    give(&mut s, ResourceId::Catnip, 0.0);
    run_days(&mut s, 30); // deep famine
    assert_eq!(s.kittens.len(), 3, "hard times relocate kittens, never delete them");
    give(&mut s, ResourceId::Catnip, 0.0);
    run_days(&mut s, 300); // absurdly deep famine
    assert_eq!(s.kittens.len(), 3);
}

// @spec SIM-KIT-002
#[test]
fn names_are_unique_pool_first_then_numbered() {
    let mut s = GameState::new(2);
    adopt(&mut s, 45);
    let names: Vec<&str> = s.kittens.iter().map(|k| k.name.as_str()).collect();
    let unique: std::collections::BTreeSet<&&str> = names.iter().collect();
    assert_eq!(unique.len(), 45, "all names unique");
    for k in &s.kittens[..40] {
        assert!(content::NAME_POOL.contains(&k.name.as_str()), "first cycle from pool: {}", k.name);
    }
    for k in &s.kittens[40..] {
        assert!(k.name.ends_with(" II"), "second cycle is numbered: {}", k.name);
        let base = k.name.trim_end_matches(" II");
        assert!(content::NAME_POOL.contains(&base));
    }
}

// @spec SIM-KIT-003
#[test]
fn arrival_needs_housing_food_and_at_most_one_a_day() {
    // No housing: nobody arrives, ever.
    let mut s = GameState::new(3);
    give(&mut s, ResourceId::Catnip, 200.0);
    s.pity = 4;
    run_days(&mut s, 15);
    assert!(s.kittens.is_empty());

    // Housing but thin pantry (< 50): still nobody.
    let mut s = GameState::new(4);
    grant(&mut s, StructureId::CozyCushion, 5);
    give(&mut s, ResourceId::Catnip, 49.0);
    s.pity = 4;
    run_days(&mut s, 15);
    assert!(s.kittens.is_empty());

    // Eligible with pity forced: exactly one per day, even with lots of room.
    let mut s = GameState::new(5);
    grant(&mut s, StructureId::CozyCushion, 10);
    give(&mut s, ResourceId::Catnip, 200.0);
    s.pity = 4;
    run_days(&mut s, 1);
    assert_eq!(s.kittens.len(), 1);
}

// @spec SIM-KIT-004
#[test]
fn arrival_chance_scales_with_stock_and_caps() {
    let mut s = GameState::new(6);
    give(&mut s, ResourceId::Catnip, 50.0);
    assert!((s.arrival_chance() - 0.15).abs() < 1e-9);
    give(&mut s, ResourceId::Catnip, 250.0);
    assert!((s.arrival_chance() - 0.25).abs() < 1e-9);
    give(&mut s, ResourceId::Catnip, 2000.0);
    assert!((s.arrival_chance() - 0.60).abs() < 1e-9, "capped at 60%");
    s.ideas.insert(IdeaId::NeighborhoodWatch);
    assert!((s.arrival_chance() - 0.75).abs() < 1e-9, "watch bonus lands on top of the cap");
}

// @spec SIM-KIT-005
#[test]
fn pity_bounds_dry_streaks_and_pauses_when_ineligible() {
    // For every seed: an always-eligible meadow sees a kitten within 5 days.
    for seed in 0..40u64 {
        let mut s = GameState::new(seed);
        grant(&mut s, StructureId::CozyCushion, 1);
        let mut days = 0;
        while s.kittens.is_empty() {
            give(&mut s, ResourceId::Catnip, 50.0); // hold chance at the 15% floor
            run_days(&mut s, 1);
            days += 1;
            assert!(days <= 5, "seed {seed}: pity must force arrival by day 5");
        }
    }

    // Ineligible days pause the counter rather than resetting it.
    let mut s = GameState::new(7);
    grant(&mut s, StructureId::CozyCushion, 1);
    s.pity = 3;
    give(&mut s, ResourceId::Catnip, 10.0); // ineligible: stock < 50
    run_days(&mut s, 4);
    assert_eq!(s.pity, 3, "ineligible days neither extend nor reset pity");
}

// @spec SIM-KIT-006
#[test]
fn away_kittens_keep_their_cushions_warm() {
    let mut s = GameState::new(8);
    adopt(&mut s, 1);
    assert_eq!(s.free_slots(), 0);
    s.kittens[0].location = Location::NeighborsYard;
    s.kittens[0].away_seq = Some(1);
    assert_eq!(s.free_slots(), 0, "an absent kitten still holds its slot");
    give(&mut s, ResourceId::Catnip, 200.0);
    s.pity = 4;
    run_days(&mut s, 3);
    assert_eq!(s.kittens.len(), 1, "no arrival may displace an absentee");
}

// @spec SIM-KIT-007
#[test]
fn arrivals_come_home_loafing_with_a_named_event() {
    let mut s = GameState::new(9);
    grant(&mut s, StructureId::CozyCushion, 1);
    give(&mut s, ResourceId::Catnip, 100.0);
    s.pity = 4;
    let events = run_days(&mut s, 1);
    let arrived = names_of(&events);
    assert_eq!(arrived.len(), 1);
    assert_eq!(s.kittens[0].name, arrived[0]);
    assert_eq!(s.kittens[0].location, Location::Home);
    assert_eq!(s.kittens[0].job, None);
    assert_eq!(s.pity, 0, "arrival resets pity");
}

// @spec SIM-KIT-008
#[test]
fn assignment_picks_the_keenest_newcomer_and_names_it() {
    let mut s = GameState::new(10);
    adopt(&mut s, 2);
    let newest = s.kittens[1].name.clone();
    let events = s.apply(Command::AssignJob(JobId::Gardener)).unwrap();
    assert!(events.contains(&Event::JobAssigned { name: newest.clone(), job: JobId::Gardener }));
    assert_eq!(s.kittens[1].job, Some(JobId::Gardener));
    assert!(s.kittens[1].job_seq.is_some());
    assert_eq!(s.kittens[0].job, None);

    s.apply(Command::AssignJob(JobId::Gardener)).unwrap();
    assert_eq!(
        s.apply(Command::AssignJob(JobId::Gardener)).unwrap_err(),
        CommandError::NoIdleKittens
    );
}

// @spec SIM-KIT-009
#[test]
fn unassignment_is_lifo_among_home_kittens() {
    let mut s = GameState::new(11);
    adopt(&mut s, 2);
    s.apply(Command::AssignJob(JobId::Gardener)).unwrap(); // newest first
    s.apply(Command::AssignJob(JobId::Gardener)).unwrap(); // then the elder
    let last_assigned = s.kittens[0].name.clone(); // elder assigned second
    let events = s.apply(Command::UnassignJob(JobId::Gardener)).unwrap();
    assert!(events.contains(&Event::JobUnassigned { name: last_assigned, job: JobId::Gardener }));
    assert_eq!(s.kittens[0].job, None);
    assert_eq!(s.kittens[1].job, Some(JobId::Gardener));

    // The remaining holder walks off to the neighbor's: nobody Home holds it.
    s.kittens[1].location = Location::NeighborsYard;
    s.kittens[1].away_seq = Some(1);
    assert_eq!(
        s.apply(Command::UnassignJob(JobId::Gardener)).unwrap_err(),
        CommandError::NoWorkersAssigned
    );
}

// @spec SIM-KIT-010
#[test]
fn effective_workers_counts_only_home_kittens() {
    let mut s = GameState::new(12);
    adopt(&mut s, 2);
    s.apply(Command::AssignJob(JobId::Gardener)).unwrap();
    s.apply(Command::AssignJob(JobId::Gardener)).unwrap();
    assert_eq!(s.effective_workers(JobId::Gardener), 2);
    s.kittens[1].location = Location::NeighborsYard;
    s.kittens[1].away_seq = Some(1);
    assert_eq!(s.effective_workers(JobId::Gardener), 1, "absent workers keep the title, not the shift");
}

// @spec SIM-KIT-011
#[test]
fn happiness_formula() {
    let mut s = GameState::new(13);
    adopt(&mut s, 1);
    assert!((s.happiness() - 101.0).abs() < 1e-9, "one loafer: 100 + 1");

    s.apply(Command::AssignJob(JobId::Gardener)).unwrap();
    assert!((s.happiness() - 100.0).abs() < 1e-9, "no loafers: flat 100");

    adopt(&mut s, 5); // six kittens: crowding −3, loafers +5 (five loafing)
    assert!((s.happiness() - (100.0 - 3.0 + 5.0)).abs() < 1e-9);

    grant(&mut s, StructureId::ScratchingPost, 3);
    assert!((s.happiness() - 117.0).abs() < 1e-9, "+5 per post");

    s.ideas.insert(IdeaId::BiscuitMaking);
    assert!((s.happiness() - 127.0).abs() < 1e-9, "+10 for biscuits");

    // Deep crowding floors at 25.
    let mut crowded = GameState::new(14);
    adopt(&mut crowded, 40);
    for k in &mut crowded.kittens {
        k.job = Some(JobId::Gardener);
        k.job_seq = Some(1);
    }
    assert_eq!(crowded.happiness(), 25.0, "floor at 25: 100 − 105 + 0");
}

// @spec SIM-KIT-012
#[test]
fn mood_multiplier_tracks_happiness_and_grumpiness_quarters_it() {
    let mut s = GameState::new(15);
    adopt(&mut s, 1);
    assert!((s.mood_multiplier() - 1.01).abs() < 1e-9);
    s.grumpy = true;
    assert!((s.mood_multiplier() - 1.01 * 0.25).abs() < 1e-9);
}

// @spec SIM-KIT-013
#[test]
fn grumpiness_follows_shortfall_tick_by_tick() {
    let mut s = GameState::new(16);
    adopt(&mut s, 1);
    s.kittens[0].job = None;
    s.ticks += 1;
    give(&mut s, ResourceId::Catnip, 0.0);
    run_ticks(&mut s, 1);
    assert!(s.grumpy);
    give(&mut s, ResourceId::Catnip, 50.0);
    run_ticks(&mut s, 1);
    assert!(!s.grumpy, "grumpiness clears on the first fed tick");
}

// @spec SIM-KIT-014
#[test]
fn mood_tiers_map_happiness_and_grumpy_overrides() {
    let mut s = GameState::new(17);
    adopt(&mut s, 1); // happiness 101
    assert_eq!(s.mood_tier(), MoodTier::Content);
    grant(&mut s, StructureId::ScratchingPost, 4); // 121
    assert_eq!(s.mood_tier(), MoodTier::Blissful);
    s.grumpy = true;
    assert_eq!(s.mood_tier(), MoodTier::Grumpy, "famine outranks bliss");
    s.grumpy = false;

    let mut crowded = GameState::new(18);
    adopt(&mut crowded, 12); // 100 − 21 + 5 = 84
    assert_eq!(crowded.mood_tier(), MoodTier::Restless);
    adopt(&mut crowded, 8); // 100 − 45 + 5 = 60 → Restless edge; one more crowds to Fussy
    assert_eq!(crowded.mood_tier(), MoodTier::Restless);
    adopt(&mut crowded, 1); // 57
    assert_eq!(crowded.mood_tier(), MoodTier::Fussy);
}

// @spec SIM-KIT-015, SIM-KIT-018
#[test]
fn day_clocks_are_mutually_exclusive_and_read_the_shortfall_flag() {
    let mut s = GameState::new(19);
    adopt(&mut s, 1);
    give(&mut s, ResourceId::Catnip, 0.0);
    run_days(&mut s, 2);
    assert_eq!((s.famine_days, s.recovery_days), (2, 0));
    give(&mut s, ResourceId::Catnip, 100.0);
    run_days(&mut s, 1);
    assert_eq!((s.famine_days, s.recovery_days), (0, 1), "a fed day resets famine");

    // The clocks read the persisted shortfall flag, not a fresh stock reading:
    // a hand-made state that is grumpy with a full pantry still counts famine.
    let mut odd = GameState::new(20);
    adopt(&mut odd, 1);
    give(&mut odd, ResourceId::Catnip, 100.0);
    odd.grumpy = true;
    let saved_stock = odd.stock(ResourceId::Catnip);
    run_ticks(&mut odd, 10); // one day; first close sees grumpy=true
    assert!(odd.famine_days >= 1, "day close consumed the flag, not the stock");
    assert!(odd.stock(ResourceId::Catnip) < saved_stock, "the pantry was open the whole time");
}

// @spec SIM-KIT-016
#[test]
fn famine_wanders_newest_first_but_never_the_last_kitten() {
    let mut s = GameState::new(21);
    adopt(&mut s, 3);
    let (eldest, middle, newest) = (
        s.kittens[0].name.clone(),
        s.kittens[1].name.clone(),
        s.kittens[2].name.clone(),
    );
    give(&mut s, ResourceId::Catnip, 0.0);
    let events = run_days(&mut s, 2);
    assert!(!events.iter().any(|e| matches!(e, Event::KittenWanderedOff { .. })));

    let events = run_days(&mut s, 1); // famine day 3
    assert!(events.contains(&Event::KittenWanderedOff { name: newest.clone() }));
    assert_eq!(s.kittens[2].location, Location::NeighborsYard);
    assert!(s.kittens[2].away_seq.is_some());

    let events = run_days(&mut s, 1); // famine day 4
    assert!(events.contains(&Event::KittenWanderedOff { name: middle.clone() }));

    let events = run_days(&mut s, 10); // deep famine
    assert!(
        !events.iter().any(|e| matches!(e, Event::KittenWanderedOff { .. })),
        "the last Home kitten never wanders"
    );
    assert_eq!(s.kittens[0].location, Location::Home);
    assert_eq!(s.kittens[0].name, eldest);
}

// @spec SIM-KIT-017
#[test]
fn returns_are_first_out_first_back_and_need_food_in_the_bowl() {
    let mut s = GameState::new(23);
    adopt(&mut s, 3);
    let (middle, newest) = (s.kittens[1].name.clone(), s.kittens[2].name.clone());
    give(&mut s, ResourceId::Catnip, 0.0);
    run_days(&mut s, 4);
    assert_eq!(s.kittens.iter().filter(|k| k.location == Location::NeighborsYard).count(), 2);

    // Feed generously: first out (newest) comes back first, one per day.
    give(&mut s, ResourceId::Catnip, 150.0);
    let events = run_days(&mut s, 1);
    assert!(events.contains(&Event::KittenReturned { name: newest.clone() }));
    let events = run_days(&mut s, 1);
    assert!(events.contains(&Event::KittenReturned { name: middle.clone() }));
    assert!(s.kittens.iter().all(|k| k.location == Location::Home));
    assert!(s.kittens.iter().all(|k| k.away_seq.is_none()), "stamps clear on return");

    // The food gate: a recovery day close with an empty bowl returns nobody.
    let mut gated = GameState::new(24);
    adopt(&mut gated, 2);
    gated.kittens[1].location = Location::NeighborsYard;
    gated.kittens[1].away_seq = Some(5);
    gated.recovery_days = 1;
    gated.grumpy = false;
    give(&mut gated, ResourceId::Catnip, 0.0);
    // One home kitten with an empty bowl: the boundary's day close reads the
    // (false) flag from the pre-day tick, but the bowl is empty → no return.
    let events = run_ticks(&mut gated, 10);
    assert!(
        !events.iter().any(|e| matches!(e, Event::KittenReturned { .. })),
        "kittens come back when the pantry has actually refilled"
    );
    assert_eq!(gated.kittens[1].location, Location::NeighborsYard);
}
