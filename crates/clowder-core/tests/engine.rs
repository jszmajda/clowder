mod common;

use clowder_core::*;
use common::*;

// @spec SIM-ENG-016
#[test]
fn new_game_is_an_empty_meadow() {
    let s = GameState::new(7);
    assert_eq!(s.ticks, 0);
    assert_eq!(s.resources, [0.0; 4]);
    assert_eq!(s.structures, [0; 7]);
    assert!(s.kittens.is_empty());
    assert!(s.ideas.is_empty());
    assert_eq!((s.pity, s.famine_days, s.recovery_days), (0, 0, 0));
    assert!(!s.grumpy);
    assert_eq!(s.zoomies_until, 0);
}

// @spec SIM-ENG-001
#[test]
fn calendar_is_derived_from_ticks_alone() {
    let mut s = GameState::new(1);
    assert_eq!((s.day(), s.year()), (0, 0));
    assert_eq!(s.season(), Season::Spring);

    s.ticks = 10;
    assert_eq!(s.day(), 1);
    s.ticks = 249;
    assert_eq!(s.season(), Season::Spring);
    s.ticks = 250; // day 25
    assert_eq!(s.season(), Season::Summer);
    s.ticks = 750; // day 75
    assert_eq!(s.season(), Season::Winter);
    s.ticks = 999;
    assert_eq!(s.year(), 0);
    s.ticks = 1000; // day 100
    assert_eq!((s.season(), s.year()), (Season::Spring, 1));
}

// @spec SIM-ENG-003
#[test]
fn boundary_events_fire_on_crossings_and_not_at_start() {
    let mut s = GameState::new(2);
    let early = run_ticks(&mut s, 249);
    assert!(
        !early.iter().any(|e| matches!(e, Event::SeasonChanged { .. } | Event::NewYear { .. })),
        "no season/year events before the first season boundary"
    );
    let cross = run_ticks(&mut s, 1); // tick 250 = day 25
    assert!(cross.contains(&Event::SeasonChanged { season: Season::Summer }));

    s.ticks = 990;
    let newyear = run_ticks(&mut s, 10); // tick 1000 = day 100 = year 1
    assert!(newyear.contains(&Event::NewYear { year: 1 }));
    assert!(newyear.contains(&Event::SeasonChanged { season: Season::Spring }));
}

// @spec SIM-ENG-004
#[test]
fn production_precedes_consumption_within_a_tick() {
    let mut s = GameState::new(3);
    adopt(&mut s, 1);
    s.kittens[0].job = None;
    grant(&mut s, StructureId::CatnipPatch, 2); // 0.25/tick × 1.5 spring = 0.375
    s.ticks = 1002; // early spring, mid-day (no boundary interference)
    give(&mut s, ResourceId::Catnip, 0.0);
    run_ticks(&mut s, 1);
    // Production lands before consumption: 0.375 − 0.085 = 0.29, no shortfall.
    assert!(!s.grumpy, "same-tick production must feed the clowder");
    assert!((s.stock(ResourceId::Catnip) - 0.29).abs() < 1e-9);
}

// @spec SIM-ENG-005
#[test]
fn rejected_commands_mutate_nothing_rng_included() {
    let mut s = GameState::new(4);
    give(&mut s, ResourceId::Catnip, 100.0);
    let snapshot = s.clone();
    // Pantry is idea-gated and no idea is researched.
    let err = s.apply(Command::Build(StructureId::Pantry)).unwrap_err();
    assert_eq!(err, CommandError::Locked);
    assert_eq!(s, snapshot, "a rejected command must be a perfect no-op");
}

// @spec SIM-ENG-006
#[test]
fn error_precedence_reports_the_most_fundamental_blocker() {
    let mut s = GameState::new(5);
    // Locked beats InsufficientResources: Pantry gated, and we're broke.
    assert_eq!(
        s.apply(Command::Build(StructureId::Pantry)).unwrap_err(),
        CommandError::Locked
    );
    // Locked beats NoIdleKittens: Groomer gated, and there are no kittens.
    assert_eq!(
        s.apply(Command::AssignJob(JobId::Groomer)).unwrap_err(),
        CommandError::Locked
    );
    // AlreadyResearched beats InsufficientResources.
    grant(&mut s, StructureId::CardboardBox, 1);
    give(&mut s, ResourceId::Curiosity, 20.0);
    s.apply(Command::ResearchIdea(IdeaId::PantryOrganization)).unwrap();
    give(&mut s, ResourceId::Curiosity, 0.0);
    assert_eq!(
        s.apply(Command::ResearchIdea(IdeaId::PantryOrganization)).unwrap_err(),
        CommandError::AlreadyResearched
    );
    // StorageFull beats InsufficientResources: twigs at cap AND catnip < 10.
    let twig_cap = s.cap(ResourceId::Twigs);
    give(&mut s, ResourceId::Twigs, twig_cap);
    give(&mut s, ResourceId::Catnip, 3.0);
    assert_eq!(s.apply(Command::Forage).unwrap_err(), CommandError::StorageFull);
}

// @spec SIM-ENG-007
#[test]
fn same_seed_same_commands_bit_identical_state() {
    let script = |s: &mut GameState| {
        for i in 0..600u64 {
            if i == 5 || i == 17 {
                let _ = s.apply(Command::GatherCatnip);
            }
            if i == 30 {
                give(s, ResourceId::Catnip, 60.0);
                grant(s, StructureId::CozyCushion, 2);
                s.pity = 4;
            }
            if i == 55 {
                let _ = s.apply(Command::AssignJob(JobId::Gardener));
            }
            s.tick();
        }
    };
    let mut a = GameState::new(42);
    let mut b = GameState::new(42);
    script(&mut a);
    script(&mut b);
    assert_eq!(save_string(&a, 0), save_string(&b, 0));

    let mut c = GameState::new(43);
    script(&mut c);
    assert_ne!(save_string(&a, 0), save_string(&c, 0), "different seed, different world");
}

// @spec SIM-ENG-008
#[test]
fn rng_is_deterministic_and_serializable() {
    let mut a = Pcg32::new(99);
    let mut b = Pcg32::new(99);
    let seq_a: Vec<u32> = (0..8).map(|_| a.next_u32()).collect();
    let seq_b: Vec<u32> = (0..8).map(|_| b.next_u32()).collect();
    assert_eq!(seq_a, seq_b);

    let json = serde_json::to_string(&a).unwrap();
    let mut restored: Pcg32 = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.next_u32(), b.next_u32(), "round-trip preserves position");
}

// @spec SIM-ENG-015
#[test]
fn day_counters_saturate_instead_of_overflowing() {
    let mut s = GameState::new(6);
    adopt(&mut s, 1);
    // Endless plenty: recovery_days pinned at max, must not panic or wrap.
    s.recovery_days = 255;
    give(&mut s, ResourceId::Catnip, 200.0);
    run_days(&mut s, 2);
    assert_eq!(s.recovery_days, 255);

    // Endless famine: same for famine_days.
    s.famine_days = 255;
    s.recovery_days = 0;
    give(&mut s, ResourceId::Catnip, 0.0);
    run_days(&mut s, 2);
    assert_eq!(s.famine_days, 255);
}

// @spec SIM-ENG-009
#[test]
fn events_carry_data_not_prose() {
    let mut s = GameState::new(8);
    adopt(&mut s, 1);
    let name = &s.kittens[0].name;
    assert!(!name.is_empty(), "arrival event/kitten must carry a real name");
}

// @spec SIM-ENG-010
#[test]
fn mood_events_fire_only_on_tier_transitions() {
    let mut s = GameState::new(9);
    adopt(&mut s, 1);
    give(&mut s, ResourceId::Catnip, 150.0);
    let steady = run_days(&mut s, 3);
    assert_eq!(
        steady.iter().filter(|e| matches!(e, Event::KittenMood { .. })).count(),
        0,
        "steady mood emits no mood events"
    );
    give(&mut s, ResourceId::Catnip, 0.0);
    let famine = run_ticks(&mut s, 3);
    let grumpy_events: Vec<_> = famine
        .iter()
        .filter(|e| matches!(e, Event::KittenMood { tier: MoodTier::Grumpy }))
        .collect();
    assert_eq!(grumpy_events.len(), 1, "exactly one transition into Grumpy");
}

// @spec SIM-ENG-011
#[test]
fn zoomies_start_roughly_once_a_season() {
    let mut s = GameState::new(10);
    let events = run_days(&mut s, 2000);
    let starts = events.iter().filter(|e| matches!(e, Event::ZoomiesStarted { .. })).count();
    // 4%/day over 2000 days ⇒ ~80 expected; generous bounds against seed luck.
    assert!((30..=150).contains(&starts), "zoomies starts: {starts}");
}

// @spec SIM-ENG-012
#[test]
fn zoomies_multiplies_job_production_and_magnitude_is_live() {
    let mut base = GameState::new(11);
    adopt(&mut base, 1);
    base.apply(Command::AssignJob(JobId::Gardener)).unwrap();
    base.ticks += 1; // step off the day boundary
    give(&mut base, ResourceId::Catnip, 50.0);

    let mut plain = base.clone();
    let mut zoomy = base.clone();
    zoomy.zoomies_until = zoomy.ticks + 50;
    run_ticks(&mut plain, 1);
    run_ticks(&mut zoomy, 1);
    let plain_gain = plain.stock(ResourceId::Catnip);
    let zoomy_gain = zoomy.stock(ResourceId::Catnip);
    // Gardener yield 0.15 at mood 1.0; zoomies adds ×1.25 ⇒ +0.0375.
    assert!((zoomy_gain - plain_gain - 0.15 * 0.25).abs() < 1e-9);

    // Magnitude upgrades live mid-window once the idea is researched.
    let mut channeled = base.clone();
    channeled.zoomies_until = channeled.ticks + 50;
    channeled.ideas.insert(IdeaId::ZoomieChanneling);
    run_ticks(&mut channeled, 1);
    let channeled_gain = channeled.stock(ResourceId::Catnip);
    assert!((channeled_gain - plain_gain - 0.15 * 0.5).abs() < 1e-9);
}

// @spec SIM-ENG-012
#[test]
fn zoomies_window_covers_exactly_the_stated_duration() {
    // Hunt a seed whose first day-boundary starts zoomies (4% per boundary).
    let mut found = None;
    for seed in 0..1000u64 {
        let mut s = GameState::new(seed);
        let events = run_ticks(&mut s, 10);
        if let Some(Event::ZoomiesStarted { until_tick }) =
            events.iter().find(|e| matches!(e, Event::ZoomiesStarted { .. }))
        {
            found = Some((s.ticks, *until_tick, s));
            break;
        }
    }
    let (now, until, _s) = found.expect("some seed in 0..1000 starts zoomies on day 1");
    // Started in phase 5 of tick `now`: boosted production ticks are
    // now+1 ..= now+120 ⇒ until = now + 121.
    assert_eq!(until, now + 121);

    // With the idea already researched at start time, the window is 240.
    let mut found_long = None;
    for seed in 0..1000u64 {
        let mut s = GameState::new(seed);
        s.ideas.insert(IdeaId::ZoomieChanneling);
        let events = run_ticks(&mut s, 10);
        if let Some(Event::ZoomiesStarted { until_tick }) =
            events.iter().find(|e| matches!(e, Event::ZoomiesStarted { .. }))
        {
            found_long = Some((s.ticks, *until_tick));
            break;
        }
    }
    let (now, until) = found_long.expect("some seed starts zoomies with the idea researched");
    assert_eq!(until, now + 241);
}

// @spec SIM-ENG-013
#[test]
fn zoomies_expiry_is_checked_every_tick_not_just_boundaries() {
    let mut s = GameState::new(12);
    s.ticks = 11; // mid-day
    s.zoomies_until = 15; // expires on a non-boundary tick
    let events = run_ticks(&mut s, 4); // ticks 12..=15
    assert!(events.contains(&Event::ZoomiesEnded));
    assert_eq!(s.zoomies_until, 0);
    assert!(!s.zoomies_active());
}

// @spec SIM-ENG-014
#[test]
fn flavor_events_are_pure_delight() {
    let mut s = GameState::new(13);
    let events = run_days(&mut s, 1500);
    let flavors: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            Event::Flavor { template, kitten } => Some((*template, kitten.clone())),
            _ => None,
        })
        .collect();
    // 8%/day over 1500 days ⇒ ~120 expected.
    assert!((50..=220).contains(&flavors.len()), "flavor count: {}", flavors.len());
    // With zero kittens ever, only ambient templates may fire, kitten-less.
    for (template, kitten) in &flavors {
        assert!(!content::FLAVOR_TEMPLATES[*template].needs_kitten);
        assert!(kitten.is_none());
    }
    // And flavor must have no mechanical effect: an identical world with the
    // same seed but no flavor consumption can't be constructed externally, so
    // assert the observable: resources stayed exactly zero (no production
    // exists in this world) despite hundreds of events.
    assert_eq!(s.resources, [0.0; 4]);
}

// @spec SIM-ENG-002, SIM-ENG-018
#[test]
fn boundary_tick_processing_and_draws_run_in_fixed_order() {
    // Returns precede arrivals in the same boundary tick's event stream.
    let mut s = GameState::new(14);
    adopt(&mut s, 2);
    grant(&mut s, StructureId::CozyCushion, 2); // room for an arrival
    let wanderer = s.kittens[1].name.clone();
    s.kittens[1].location = Location::NeighborsYard;
    s.kittens[1].away_seq = Some(900);
    s.recovery_days = 1;
    s.grumpy = false;
    give(&mut s, ResourceId::Catnip, 120.0);
    s.pity = 4; // force the arrival with no RNG draw
    let events = run_days(&mut s, 1);
    let returned_at = events
        .iter()
        .position(|e| matches!(e, Event::KittenReturned { name } if *name == wanderer))
        .expect("wanderer returns");
    let arrived_at = events
        .iter()
        .position(|e| matches!(e, Event::KittenArrived { .. }))
        .expect("pity-forced arrival");
    assert!(returned_at < arrived_at, "returns come home before newcomers arrive");

    // Phase-1 arrivals precede phase-5 zoomies/flavor in the same tick's events.
    let mut hunted = None;
    for seed in 0..2000u64 {
        let mut s = GameState::new(seed);
        give(&mut s, ResourceId::Catnip, 100.0);
        grant(&mut s, StructureId::CozyCushion, 1);
        s.pity = 4;
        let events = run_days(&mut s, 1);
        let arrival = events.iter().position(|e| matches!(e, Event::KittenArrived { .. }));
        let phase5 = events.iter().position(|e| {
            matches!(e, Event::ZoomiesStarted { .. } | Event::Flavor { .. })
        });
        if let (Some(a), Some(p)) = (arrival, phase5) {
            hunted = Some((a, p));
            break;
        }
    }
    let (arrival_ix, phase5_ix) = hunted.expect("some seed fires arrival + a phase-5 event same day");
    assert!(arrival_ix < phase5_ix, "daily processing precedes random events");
}

// @spec SIM-ENG-017
#[test]
fn season_forecast_requires_the_idea() {
    let mut s = GameState::new(15);
    assert_eq!(s.season_forecast(), None);
    s.ideas.insert(IdeaId::SeasonalAwareness);
    assert_eq!(s.season_forecast(), Some(Season::Summer));
    s.ticks = 990; // day 99, Winter
    assert_eq!(s.season_forecast(), Some(Season::Spring));
}
