mod common;

use clowder_core::*;
use common::*;

// @spec SIM-ENG-016
#[test]
fn new_game_is_an_empty_meadow() {
    let s = GameState::new(7);
    assert_eq!(s.ticks, 0);
    assert_eq!(s.resources, [0.0; 46]);
    assert_eq!(s.structures, [0; 53]);
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
        !early
            .iter()
            .any(|e| matches!(e, Event::SeasonChanged { .. } | Event::NewYear { .. })),
        "no season/year events before the first season boundary"
    );
    let cross = run_ticks(&mut s, 1); // tick 250 = day 25
    assert!(cross.contains(&Event::SeasonChanged {
        season: Season::Summer
    }));

    s.ticks = 990;
    let newyear = run_ticks(&mut s, 10); // tick 1000 = day 100 = year 1
    assert!(newyear.contains(&Event::NewYear { year: 1 }));
    assert!(newyear.contains(&Event::SeasonChanged {
        season: Season::Spring
    }));
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
    s.apply(Command::ResearchIdea(IdeaId::PantryOrganization))
        .unwrap();
    give(&mut s, ResourceId::Curiosity, 0.0);
    assert_eq!(
        s.apply(Command::ResearchIdea(IdeaId::PantryOrganization))
            .unwrap_err(),
        CommandError::AlreadyResearched
    );
    // StorageFull beats InsufficientResources: twigs at cap AND catnip < 10.
    let twig_cap = s.cap(ResourceId::Twigs);
    give(&mut s, ResourceId::Twigs, twig_cap);
    give(&mut s, ResourceId::Catnip, 3.0);
    assert_eq!(
        s.apply(Command::Forage).unwrap_err(),
        CommandError::StorageFull
    );
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
    assert_ne!(
        save_string(&a, 0),
        save_string(&c, 0),
        "different seed, different world"
    );
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
    assert_eq!(
        restored.next_u32(),
        b.next_u32(),
        "round-trip preserves position"
    );
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
    assert!(
        !name.is_empty(),
        "arrival event/kitten must carry a real name"
    );
}

// @spec SIM-ENG-010
#[test]
fn mood_events_fire_only_on_tier_transitions() {
    let mut s = GameState::new(9);
    adopt(&mut s, 1);
    give(&mut s, ResourceId::Catnip, 150.0);
    let steady = run_days(&mut s, 3);
    assert_eq!(
        steady
            .iter()
            .filter(|e| matches!(e, Event::KittenMood { .. }))
            .count(),
        0,
        "steady mood emits no mood events"
    );
    give(&mut s, ResourceId::Catnip, 0.0);
    let famine = run_ticks(&mut s, 3);
    let grumpy_events: Vec<_> = famine
        .iter()
        .filter(|e| {
            matches!(
                e,
                Event::KittenMood {
                    tier: MoodTier::Grumpy
                }
            )
        })
        .collect();
    assert_eq!(grumpy_events.len(), 1, "exactly one transition into Grumpy");
}

// @spec SIM-ENG-011
#[test]
fn zoomies_start_roughly_once_a_season() {
    let mut s = GameState::new(10);
    let events = run_days(&mut s, 2000);
    let starts = events
        .iter()
        .filter(|e| matches!(e, Event::ZoomiesStarted { .. }))
        .count();
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
        if let Some(Event::ZoomiesStarted { until_tick }) = events
            .iter()
            .find(|e| matches!(e, Event::ZoomiesStarted { .. }))
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
        if let Some(Event::ZoomiesStarted { until_tick }) = events
            .iter()
            .find(|e| matches!(e, Event::ZoomiesStarted { .. }))
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
    assert!(
        (50..=220).contains(&flavors.len()),
        "flavor count: {}",
        flavors.len()
    );
    // With zero kittens ever, only ambient templates may fire, kitten-less.
    for (template, kitten) in &flavors {
        assert!(!content::FLAVOR_TEMPLATES[*template].needs_kitten);
        assert!(kitten.is_none());
    }
    // And flavor must have no mechanical effect: an identical world with the
    // same seed but no flavor consumption can't be constructed externally, so
    // assert the observable: resources stayed exactly zero (no production
    // exists in this world) despite hundreds of events.
    assert_eq!(s.resources, [0.0; 46]);
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
    assert!(
        returned_at < arrived_at,
        "returns come home before newcomers arrive"
    );

    // Phase-1 arrivals precede phase-5 zoomies/flavor in the same tick's events.
    let mut hunted = None;
    for seed in 0..2000u64 {
        let mut s = GameState::new(seed);
        give(&mut s, ResourceId::Catnip, 100.0);
        grant(&mut s, StructureId::CozyCushion, 1);
        s.pity = 4;
        let events = run_days(&mut s, 1);
        let arrival = events
            .iter()
            .position(|e| matches!(e, Event::KittenArrived { .. }));
        let phase5 = events
            .iter()
            .position(|e| matches!(e, Event::ZoomiesStarted { .. } | Event::Flavor { .. }));
        if let (Some(a), Some(p)) = (arrival, phase5) {
            hunted = Some((a, p));
            break;
        }
    }
    let (arrival_ix, phase5_ix) =
        hunted.expect("some seed fires arrival + a phase-5 event same day");
    assert!(
        arrival_ix < phase5_ix,
        "daily processing precedes random events"
    );
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

// ---- Depth Wave 1 ----

// @spec SIM-ENG-020
#[test]
fn diminishing_returns_primitives() {
    // limited_dr: identity in the linear region, asymptotic above, never reaches.
    assert!((limited_dr(0.5, 1.0) - 0.5).abs() < 1e-12);
    assert!((limited_dr(0.75, 1.0) - 0.75).abs() < 1e-12);
    assert!(limited_dr(2.0, 1.0) > 0.75 && limited_dr(2.0, 1.0) < 1.0);
    assert!(limited_dr(1000.0, 1.0) < 1.0);
    assert!(limited_dr(3.0, 1.0) < limited_dr(4.0, 1.0), "monotone");

    // unlimited_dr known points at stripe 1: f(1)=1, f(3)=2, f(6)=3.
    assert!((unlimited_dr(0.0, 1.0) - 0.0).abs() < 1e-12);
    assert!((unlimited_dr(1.0, 1.0) - 1.0).abs() < 1e-12);
    assert!((unlimited_dr(3.0, 1.0) - 2.0).abs() < 1e-12);
    assert!((unlimited_dr(6.0, 1.0) - 3.0).abs() < 1e-12);
}

// @spec SIM-ENG-019
#[test]
fn effect_sums_are_deterministic_and_source_complete() {
    let mut s = GameState::new(30);
    grant(&mut s, StructureId::Pantry, 2);
    grant(&mut s, StructureId::CardboardBox, 3);
    assert!((s.effect(Effect::MaxCap(ResourceId::Catnip)) - 500.0).abs() < 1e-9);
    assert!((s.effect(Effect::MaxCap(ResourceId::Curiosity)) - 150.0).abs() < 1e-9);
    // cap() is effects-backed: base + summed extensions.
    assert!((s.cap(ResourceId::Catnip) - 700.0).abs() < 1e-9);
    let a = s.effect(Effect::MaxCap(ResourceId::Catnip));
    let b = s.effect(Effect::MaxCap(ResourceId::Catnip));
    assert_eq!(a.to_bits(), b.to_bits(), "bit-identical resummation");
}

// @spec SIM-ENG-022
#[test]
fn converter_throttle_scales_by_worst_input() {
    let mut s = GameState::new(31);
    give(&mut s, ResourceId::Catnip, 50.0);
    give(&mut s, ResourceId::Twigs, 1.0);
    // Demands: 10 catnip (plenty) and 4 twigs (only 25% supplied) → 0.25.
    let f = s.throttle_fraction(&[(ResourceId::Catnip, 10.0), (ResourceId::Twigs, 4.0)]);
    assert!((f - 0.25).abs() < 1e-9);
    // Fully starved input → 0.0, and that's a quiet idle, not an error.
    give(&mut s, ResourceId::Twigs, 0.0);
    let f = s.throttle_fraction(&[(ResourceId::Twigs, 4.0)]);
    assert_eq!(f, 0.0);
    // Fully supplied → 1.0.
    let f = s.throttle_fraction(&[(ResourceId::Catnip, 10.0)]);
    assert_eq!(f, 1.0);
}

// @spec SIM-ENG-023, SIM-ENG-061
#[test]
fn content_reveals_stick_and_availability_advises() {
    let mut s = GameState::new(32);
    // A fresh meadow has never seen 30% of a patch's price.
    assert_eq!(
        s.availability(ContentId::Structure(StructureId::CatnipPatch)),
        Availability::Hidden
    );
    // Gather up to the reveal fraction (30% of 10 = 3 catnip).
    for _ in 0..3 {
        s.apply(Command::GatherCatnip).unwrap();
    }
    let events = run_ticks(&mut s, 1);
    assert!(events.contains(&Event::Revealed {
        id: ContentId::Structure(StructureId::CatnipPatch)
    }));
    assert_eq!(
        s.availability(ContentId::Structure(StructureId::CatnipPatch)),
        Availability::Teased
    );
    for _ in 0..8 {
        s.apply(Command::GatherCatnip).unwrap();
    }
    assert_eq!(
        s.availability(ContentId::Structure(StructureId::CatnipPatch)),
        Availability::Available
    );
    // Sticky: spending back below the fraction keeps it revealed.
    s.apply(Command::Build(StructureId::CatnipPatch)).unwrap();
    give(&mut s, ResourceId::Catnip, 0.0);
    assert_ne!(
        s.availability(ContentId::Structure(StructureId::CatnipPatch)),
        Availability::Hidden
    );
    // Idea-gated structures stay Hidden until their idea exists.
    assert_eq!(
        s.availability(ContentId::Structure(StructureId::Pantry)),
        Availability::Hidden
    );
}

/// Every structure behind a gate reveals in the first scan after that gate
/// opens, on a meadow holding none of its price — on the hot path and on the
/// post-prestige path a Tradition switches to alike. The price comes into view
/// with the row, so every store it names is revealed too.
// @spec SIM-ENG-060, SIM-ENG-049
#[test]
fn a_gated_structure_reveals_when_its_gate_opens_whatever_the_stock() {
    let mut checked = 0;
    for prestiged in [false, true] {
        for st in StructureId::ALL {
            let mut s = GameState::new(60);
            if prestiged {
                s.traditions.insert(TraditionId::TravelingBasketsI);
            }
            match content::structure_def(st).gate {
                StructureGate::Always => continue,
                StructureGate::Idea(i) => {
                    s.ideas.insert(i);
                }
                StructureGate::Site(site) => {
                    s.sites_reached.insert(site);
                }
            }
            let id = ContentId::Structure(st);
            assert_eq!(
                s.availability(id),
                Availability::Hidden,
                "{st:?} before the scan"
            );

            let events = run_ticks(&mut s, 1);

            let price = s.cost_of_next(id, 1);
            assert!(
                price.iter().any(|(r, amount)| s.stock(*r) < 0.30 * amount),
                "fixture: the meadow must hold less than the reveal fraction of {st:?}'s price"
            );
            assert!(
                events.contains(&Event::Revealed { id }),
                "{st:?} (prestiged: {prestiged}) did not reveal when its gate opened"
            );
            assert_ne!(
                s.availability(id),
                Availability::Hidden,
                "{st:?} after the scan"
            );
            for (r, _) in &price {
                assert!(
                    s.resource_revealed(*r),
                    "{st:?}'s price names {r:?}, which stayed unrevealed"
                );
            }
            checked += 1;
        }
    }
    assert!(checked > 0, "fixture: some structures carry a gate");
}

/// The player's path: researching the idea that unlocks a structure brings the
/// structure's row on the very next tick, with nothing of its price in stock.
// @spec SIM-ENG-060
#[test]
fn researching_an_idea_reveals_the_structure_it_unlocks_on_the_next_tick() {
    let mut s = GameState::new(61);
    grant(&mut s, StructureId::CardboardBox, 1); // curiosity storage opens the idea tree
    let mut pending = vec![IdeaId::Tinkering];
    while let Some(i) = pending.pop() {
        for p in content::idea_def(i).prereqs {
            if s.ideas.insert(*p) {
                pending.push(*p);
            }
        }
    }
    for (r, amount) in s.cost_of_next(ContentId::Idea(IdeaId::Tinkering), 1) {
        give(&mut s, r, amount);
    }
    let gated = ContentId::Structure(StructureId::CraftCorner);
    assert_eq!(
        content::structure_def(StructureId::CraftCorner).gate,
        StructureGate::Idea(IdeaId::Tinkering),
        "fixture"
    );

    s.apply(Command::ResearchIdea(IdeaId::Tinkering)).unwrap();
    assert_eq!(
        s.availability(gated),
        Availability::Hidden,
        "the scan runs at the tick"
    );
    let events = run_ticks(&mut s, 1);

    assert!(events.contains(&Event::Revealed { id: gated }));
    assert_eq!(s.availability(gated), Availability::Teased);
}

// @spec SIM-ENG-024, SIM-ENG-043
#[test]
fn speed_is_clamped_state_that_never_touches_tick_rules() {
    let mut s = GameState::new(33);
    assert_eq!(s.speed, 1.0);
    let events = s.apply(Command::SetSpeed(3.0)).unwrap();
    assert!(events.contains(&Event::SpeedChanged { speed: 3.0 }));
    assert_eq!(s.speed, 3.0);
    s.apply(Command::SetSpeed(99.0)).unwrap();
    assert_eq!(s.speed, 8.0, "clamped high");
    s.apply(Command::SetSpeed(0.001)).unwrap();
    assert_eq!(s.speed, 0.25, "clamped low");

    // Two worlds differing only in speed tick identically.
    let mut slow = GameState::new(34);
    let mut fast = GameState::new(34);
    fast.apply(Command::SetSpeed(8.0)).unwrap();
    let ev_slow = run_ticks(&mut slow, 200);
    let ev_fast = run_ticks(&mut fast, 200);
    assert_eq!(
        ev_slow, ev_fast,
        "speed changes cadence at the boundary, never tick behavior"
    );

    // The speed field is the ONE permitted difference. The journals must match
    // outright: a speed change is never journaled, because the journal records
    // what happened in the meadow and speed is the pace it is watched at. This
    // assertion is that spec's only coverage, which is why it is annotated on
    // the test rather than on any code — a rule about what never happens has no
    // production sink to point at.
    assert_eq!(
        slow.journal, fast.journal,
        "a speed change leaves no trace in the journal"
    );

    fast.speed = slow.speed;
    assert_eq!(save_string(&slow, 0), save_string(&fast, 0));
}

/// A non-finite speed is replaced rather than clamped. `f64::clamp` propagates
/// NaN instead of bounding it, so without the substitution a NaN would reach
/// the speed field — where it is unserializable, survives no save, and poisons
/// every interval priced against it afterwards.
///
/// Nothing asserted this until the differential noticed the code doing
/// something the spec did not mention. The guard is right; it was undefended.
/// @spec SIM-ENG-024
#[test]
fn a_speed_that_is_not_a_number_is_replaced_rather_than_clamped() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut s = GameState::new(11);
        s.apply(Command::SetSpeed(bad))
            .expect("setting speed never fails");
        assert!(
            s.speed.is_finite(),
            "SetSpeed({bad}) left speed at {}, which no save can carry and no interval can \
             be priced against",
            s.speed
        );
        // The bounds the spec names, spelled out here because the constants are
        // crate-private and this test is deliberately outside the crate.
        assert!(
            (0.25..=8.0).contains(&s.speed),
            "SetSpeed({bad}) left speed at {} — outside the range every other value is held to",
            s.speed
        );
    }
}

/// Every command, not one of them.
///
/// SIM-ENG-005 is a claim about the whole command layer: each one either fully
/// applies or returns a typed error having touched nothing, RNG included. The
/// test above checks that for a single command in a single rejecting state,
/// which is how a new command that mutates before failing would pass the suite.
///
/// This walks every variant. A command that succeeds is not this spec's
/// business; a command that fails must leave the meadow bit-identical.
/// @spec SIM-ENG-005
#[test]
fn no_rejected_command_of_any_kind_leaves_a_mark() {
    let every_command = vec![
        Command::GatherCatnip,
        Command::Forage,
        Command::Rummage,
        Command::Build(StructureId::ALL[0]),
        Command::AssignJob(JobId::ALL[0]),
        Command::UnassignJob(JobId::ALL[0]),
        Command::ResearchIdea(IdeaId::ALL[0]),
        Command::Craft(CraftId::ALL[0]),
        Command::ObserveSky,
        Command::SetSpeed(2.0),
        Command::Pounce(1),
        Command::HostJamboree,
        Command::Promote(0),
        Command::AppointElder(0),
        Command::BuyRefinement(RefinementId::ALL[0]),
        Command::AdoptRule(RuleId::ALL[0]),
        Command::AssignTinkerer(CraftId::ALL[0]),
        Command::UnassignTinkerer(CraftId::ALL[0]),
        Command::Bask,
        Command::Reverie,
        Command::BuyRite(RiteId::ALL[0]),
        Command::Enlighten,
        Command::HoldParade,
        Command::BuyArch(ArchId::ALL[0]),
        Command::RefineStarshard,
        Command::RefineEcho,
        Command::CondenseJar,
        Command::MakePromise,
        Command::Trade(ColonyId::ALL[0], 1),
        Command::SendPenPal(ColonyId::ALL[0]),
        Command::ScoutPicnic,
        Command::GreatMeadowMove,
        Command::PassItDown,
        Command::BuyTradition(TraditionId::ALL[0]),
        Command::DeclareAdventure(None),
        Command::LaunchExpedition(SiteId::ALL[0]),
        Command::SummonStarWhale,
        Command::FeedWhale,
        Command::DeepNap(1),
        Command::BuyDreamLoom(DreamLoomId::ALL[0]),
    ];

    let mut rejected = 0;
    for cmd in every_command {
        let mut s = GameState::new(4242);
        // A little of everything, so the rejections that fire are the gates and
        // the prerequisites rather than only "you have nothing".
        give(&mut s, ResourceId::Catnip, 50.0);
        give(&mut s, ResourceId::Twigs, 5.0);
        let snapshot = s.clone();

        if s.apply(cmd.clone()).is_err() {
            rejected += 1;
            assert_eq!(
                s, snapshot,
                "{cmd:?} was rejected and still changed the meadow — a rejected command must \
                 be a perfect no-op, RNG state included"
            );
        }
    }

    assert!(
        rejected >= 30,
        "only {rejected} of the commands were rejected on a near-empty meadow; this test \
         proves nothing if most of them succeed"
    );
}

/// The RNG schedule, pinned.
///
/// SIM-ENG-018 fixes the order in which a day boundary consumes draws and says
/// new subsystems append to it, never reorder it. Nothing enforced that. Every
/// determinism test here compares two runs of the *same* build, which proves
/// the generator is deterministic and cannot see a schedule that changed
/// between builds — and a schedule that changes silently rewrites the future of
/// every save already on disk.
///
/// This pins the generator's position after a fixed run. An inserted draw, a
/// reordered one, or a changed probability that gates a later draw all move it.
///
/// **Ignored on purpose.** It fails for good reasons as well as bad ones: any
/// balance change that alters how often a roll fires moves this number too, and
/// during development that is often. Run it deliberately before shipping, with
/// `--ignored`. A failure is not a bug — it is notice that existing saves will
/// diverge from where they were heading, which is a thing to decide rather than
/// discover.
/// @spec SIM-ENG-018, SIM-ENG-007
#[test]
#[ignore = "golden: run before shipping — a change means existing saves diverge"]
fn the_draw_schedule_has_not_moved() {
    for (seed, expected) in [
        (7u64, 11_951_728_235_097_488_807u64),
        (99, 12_315_264_012_441_656_832),
    ] {
        let mut s = GameState::new(seed);
        s.resources[ResourceId::Catnip.ix()] += 5_000.0;
        for _ in 0..20_000 {
            s.tick();
        }
        let saved: serde_json::Value = serde_json::from_str(&save_string(&s, 0)).unwrap();
        let actual = saved["state"]["rng"]["state"]
            .as_u64()
            .expect("the rng records a position");
        assert_eq!(
            actual, expected,
            "after 20000 ticks on seed {seed} the generator sits at {actual}, not {expected}: \
             the draw schedule has moved, so every save already written will play out \
             differently from here than it would have"
        );
    }
}

/// The Forecaster tradition shows the coming season with no idea needed.
/// @spec SIM-ENG-017
#[test]
fn the_forecaster_knows_the_coming_season() {
    let mut s = GameState::new(1302);
    assert_eq!(s.season_forecast(), None);
    s.traditions.insert(TraditionId::Forecaster);
    assert!(s.season_forecast().is_some());
}
