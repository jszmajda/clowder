//! Depth Wave 3 tests: society (skills, ranks, the Elder, traits, jamborees),
//! pouncing expeditions, Tinkerer background crafting, refinements, and house
//! rules. Written tests-first from the specs — they compile against the Depth
//! Wave 3 API contract and fail until the content lands.

mod common;

use clowder_core::*;
use common::*;

// ---- Local band map (transcribed from the curiosity design's tree tables) ----
fn idea_band(i: IdeaId) -> u32 {
    match i as u32 {
        0..=17 | 56 => 0,
        18..=27 => 1,
        28..=35 => 2,
        36..=43 => 3,
        44..=55 => 4,
        other => panic!("unmapped idea id {other}"),
    }
}

/// A structure's band is the band of its gate idea; ungated = band 0.
fn structure_band(s: StructureId) -> u32 {
    content::structure_def(s).gate.map(idea_band).unwrap_or(0)
}

/// Step off the day boundary and quiet any adoption-day zoomies so a single
/// tick measures exactly one production/consumption step.
fn off_boundary(s: &mut GameState) {
    s.ticks += 1;
    s.zoomies_until = 0;
}

/// Run ticks with zoomies suppressed before every step, so long measured
/// spans never pick up a surprise multiplier at a day boundary.
fn run_calm_ticks(s: &mut GameState, n: u64) -> Vec<Event> {
    (0..n)
        .flat_map(|_| {
            s.zoomies_until = 0;
            s.tick()
        })
        .collect()
}

fn skill(s: &GameState, kitten: usize, j: JobId) -> f64 {
    s.kittens[kitten].skills[j as usize]
}

/// A meadow ready for Tinkerer work: ideas researched, workshops built,
/// kittens adopted and put on the Tinkerer job.
fn tinker_world(seed: u64, workshops: u32, tinkerers: usize) -> GameState {
    let mut s = GameState::new(seed);
    s.ideas.insert(IdeaId::Toymaking); // Tinkerer job + Toy Workshop
    s.ideas.insert(IdeaId::Tinkering); // Wands/Tiles recipes
    adopt(&mut s, tinkerers);
    grant(&mut s, StructureId::ToyWorkshop, workshops);
    for _ in 0..tinkerers {
        s.apply(Command::AssignJob(JobId::Tinkerer)).unwrap();
    }
    s
}

// =====================================================================
// KIT — Skills (SIM-KIT-019)
// =====================================================================

// @spec SIM-KIT-019
#[test]
fn employed_home_kittens_accrue_skill_and_mirror_it_into_exp() {
    let mut s = GameState::new(201);
    adopt(&mut s, 1);
    s.apply(Command::AssignJob(JobId::Gardener)).unwrap();
    off_boundary(&mut s);
    give(&mut s, ResourceId::Catnip, 100.0);
    run_calm_ticks(&mut s, 10);
    assert!(
        (skill(&s, 0, JobId::Gardener) - 0.1).abs() < 1e-9,
        "0.01 XP per tick"
    );
    assert!(
        (s.kittens[0].exp - 0.1).abs() < 1e-9,
        "the exp pool mirrors accrual"
    );
    // Untouched skills stay at zero — decay floors there, never below.
    assert_eq!(skill(&s, 0, JobId::TwigCollector), 0.0);
}

// @spec SIM-KIT-019
#[test]
fn other_job_skills_decay_while_working_with_a_floor_at_zero() {
    let mut s = GameState::new(202);
    adopt(&mut s, 1);
    s.apply(Command::AssignJob(JobId::Gardener)).unwrap();
    s.kittens[0].skills[JobId::TwigCollector as usize] = 1.0;
    s.kittens[0].skills[JobId::Groomer as usize] = 0.005; // less than 10 ticks of decay
    off_boundary(&mut s);
    give(&mut s, ResourceId::Catnip, 100.0);
    run_calm_ticks(&mut s, 10);
    assert!(
        (skill(&s, 0, JobId::TwigCollector) - 0.99).abs() < 1e-9,
        "0.001/tick decay on every other job's skill"
    );
    assert_eq!(skill(&s, 0, JobId::Groomer), 0.0, "decay floors at zero");
    // Decay never touches the lifetime pool: exp only mirrors accrual.
    assert!((s.kittens[0].exp - 0.1).abs() < 1e-9);
}

// @spec SIM-KIT-019
#[test]
fn loafers_and_away_kittens_neither_accrue_nor_decay() {
    let mut s = GameState::new(203);
    adopt(&mut s, 2);
    s.apply(Command::AssignJob(JobId::Gardener)).unwrap(); // newest (index 1)
    s.kittens[0].skills[JobId::Gardener as usize] = 50.0; // loafing kitten
    s.kittens[1].skills[JobId::TwigCollector as usize] = 50.0; // employed kitten decays this
    off_boundary(&mut s);
    give(&mut s, ResourceId::Catnip, 100.0);
    run_calm_ticks(&mut s, 10);
    assert_eq!(skill(&s, 0, JobId::Gardener), 50.0, "loafers are frozen");
    assert_eq!(s.kittens[0].exp, 0.0);
    assert!((skill(&s, 1, JobId::TwigCollector) - 49.99).abs() < 1e-9);

    // Away kittens are frozen entirely, job title or not. A lean meadow
    // (empty bowl) keeps the wanderer at the neighbor's — a well-fed one would
    // recover and come home at the next day close, which is a different test.
    let mut away = GameState::new(204);
    adopt(&mut away, 2);
    away.apply(Command::AssignJob(JobId::Gardener)).unwrap();
    away.kittens[1].location = Location::NeighborsYard;
    away.kittens[1].away_seq = Some(1);
    away.kittens[1].skills[JobId::Gardener as usize] = 10.0;
    away.kittens[1].skills[JobId::Groomer as usize] = 10.0;
    off_boundary(&mut away);
    give(&mut away, ResourceId::Catnip, 0.0);
    run_calm_ticks(&mut away, 10);
    assert_eq!(
        skill(&away, 1, JobId::Gardener),
        10.0,
        "no accrual while away"
    );
    assert_eq!(skill(&away, 1, JobId::Groomer), 10.0, "no decay while away");
    assert_eq!(away.kittens[1].exp, 0.0);
}

// @spec SIM-KIT-019
#[test]
fn grumpiness_slows_paws_not_learning() {
    let mut s = GameState::new(205);
    adopt(&mut s, 1);
    // A Twig Collector, not a Gardener: it makes no catnip, so the bowl stays
    // empty and the clowder stays Grumpy while the kitten keeps studying twigs.
    // (A lone Gardener would out-produce its own appetite and never go hungry.)
    s.apply(Command::AssignJob(JobId::TwigCollector)).unwrap();
    off_boundary(&mut s);
    give(&mut s, ResourceId::Catnip, 0.0); // shortfall every tick: Grumpy
    run_calm_ticks(&mut s, 10);
    assert!(s.grumpy, "an empty bowl keeps the clowder Grumpy");
    assert!(
        (skill(&s, 0, JobId::TwigCollector) - 0.1).abs() < 1e-9,
        "learning is never slowed"
    );
    assert!((s.kittens[0].exp - 0.1).abs() < 1e-9);
}

// @spec SIM-KIT-019, SIM-CUR-010
#[test]
fn skill_xp_ratio_and_decay_removal_refinements_change_the_flows() {
    // Encouraging Mews: skill XP +50% ⇒ 0.015/tick, mirrored into exp.
    let mut s = GameState::new(206);
    adopt(&mut s, 1);
    s.apply(Command::AssignJob(JobId::Gardener)).unwrap();
    s.refinements.insert(RefinementId::EncouragingMews);
    s.kittens[0].skills[JobId::TwigCollector as usize] = 1.0;
    off_boundary(&mut s);
    give(&mut s, ResourceId::Catnip, 100.0);
    run_calm_ticks(&mut s, 10);
    assert!(
        (skill(&s, 0, JobId::Gardener) - 0.15).abs() < 1e-9,
        "XP rides the ratio"
    );
    assert!((s.kittens[0].exp - 0.15).abs() < 1e-9);
    assert!(
        (skill(&s, 0, JobId::TwigCollector) - 0.99).abs() < 1e-9,
        "decay is unchanged"
    );

    // Group Lessons removes the decay entirely.
    let mut lessons = GameState::new(207);
    adopt(&mut lessons, 1);
    lessons.apply(Command::AssignJob(JobId::Gardener)).unwrap();
    lessons.refinements.insert(RefinementId::GroupLessons);
    lessons.kittens[0].skills[JobId::TwigCollector as usize] = 1.0;
    off_boundary(&mut lessons);
    give(&mut lessons, ResourceId::Catnip, 100.0);
    run_calm_ticks(&mut lessons, 10);
    assert_eq!(
        skill(&lessons, 0, JobId::TwigCollector),
        1.0,
        "no decay with Group Lessons"
    );
    assert!((skill(&lessons, 0, JobId::Gardener) - 0.1).abs() < 1e-9);
}

// =====================================================================
// KIT — Skill tiers, promotion, the Elder (SIM-KIT-020..022)
// =====================================================================

/// One measured production tick for a lone Gardener with the given skill
/// XP: returns the catnip delta (production − the kitten's 0.085 appetite).
fn gardener_tick_delta(seed: u64, xp: f64, rank: u8, elder: bool) -> f64 {
    let mut s = GameState::new(seed);
    adopt(&mut s, 1);
    s.apply(Command::AssignJob(JobId::Gardener)).unwrap();
    s.kittens[0].skills[JobId::Gardener as usize] = xp;
    s.kittens[0].rank = rank;
    if elder {
        s.elder = Some(0);
    }
    off_boundary(&mut s);
    give(&mut s, ResourceId::Catnip, 100.0);
    run_ticks(&mut s, 1);
    s.stock(ResourceId::Catnip) - 100.0 + 0.085
}

// @spec SIM-KIT-020
#[test]
fn skill_tiers_multiply_the_kittens_own_job_yield() {
    // A lone employed kitten: happiness 100, mood ×1.0, gardener base 0.15.
    let tiers = [
        (0.0, 1.0),
        (99.9, 1.0), // just under the first tier boundary
        (100.0, 1.0125),
        (500.0, 1.025),
        (1200.0, 1.045),
        (2500.0, 1.075),
        (5000.0, 1.125),
        (9000.0, 1.1875),
    ];
    for (i, (xp, mult)) in tiers.iter().enumerate() {
        let delta = gardener_tick_delta(210 + i as u64, *xp, 0, false);
        assert!(
            (delta - 0.15 * mult).abs() < 1e-9,
            "at {xp} XP the gardener yields 0.15 × {mult}, measured {delta}"
        );
    }
}

// @spec SIM-KIT-021
#[test]
fn promote_spends_exp_and_shinies_and_ranks_up() {
    let mut s = GameState::new(220);
    adopt(&mut s, 1);
    let name = s.kittens[0].name.clone();
    s.kittens[0].exp = 600.0;
    give(&mut s, ResourceId::Shinies, 25.0);
    let events = s.apply(Command::Promote(0)).unwrap();
    assert!(events.contains(&Event::KittenPromoted {
        name: name.clone(),
        rank: 1
    }));
    assert_eq!(s.kittens[0].rank, 1);
    assert!(
        (s.kittens[0].exp - 100.0).abs() < 1e-9,
        "500 exp spent from the pool"
    );
    assert_eq!(s.stock(ResourceId::Shinies), 0.0, "25 shinies spent");

    // Rank 1 → 2 costs 500 × 1.75 = 875 exp and 25 × 2 = 50 shinies; short on
    // either rejects with InsufficientResources and spends nothing.
    s.kittens[0].exp = 875.0;
    give(&mut s, ResourceId::Shinies, 49.0);
    let err = s.apply(Command::Promote(0)).unwrap_err();
    assert!(matches!(err, CommandError::InsufficientResources { .. }));
    assert_eq!(s.kittens[0].rank, 1);
    assert!(
        (s.kittens[0].exp - 875.0).abs() < 1e-9,
        "nothing spent on a refusal"
    );
    assert_eq!(s.stock(ResourceId::Shinies), 49.0);

    s.kittens[0].exp = 874.0; // exp short, shinies plenty
    give(&mut s, ResourceId::Shinies, 50.0);
    let err = s.apply(Command::Promote(0)).unwrap_err();
    assert!(matches!(err, CommandError::InsufficientResources { .. }));
    assert_eq!(s.stock(ResourceId::Shinies), 50.0);

    s.kittens[0].exp = 875.0;
    let events = s.apply(Command::Promote(0)).unwrap();
    assert!(events.contains(&Event::KittenPromoted { name, rank: 2 }));
    assert_eq!(s.kittens[0].rank, 2);
    assert!(s.kittens[0].exp.abs() < 1e-9);
    assert_eq!(s.stock(ResourceId::Shinies), 0.0);
}

// @spec SIM-KIT-022
#[test]
fn appoint_elder_replaces_freely_and_scales_own_job_yield_by_rank() {
    let mut s = GameState::new(221);
    adopt(&mut s, 2);
    let (first, second) = (s.kittens[0].name.clone(), s.kittens[1].name.clone());
    let events = s.apply(Command::AppointElder(0)).unwrap();
    assert!(events.contains(&Event::ElderAppointed { name: first }));
    assert_eq!(s.elder, Some(0));
    // Replacing is free and unceremonious; the game never removes an Elder.
    let events = s.apply(Command::AppointElder(1)).unwrap();
    assert!(events.contains(&Event::ElderAppointed { name: second }));
    assert_eq!(s.elder, Some(1));

    // Own-job multiplier max(1, (rank+1)/1.4): rank 0 gives ×1 (min applies),
    // rank 3 gives ×4/1.4.
    let plain = gardener_tick_delta(222, 0.0, 0, false);
    let rank0_elder = gardener_tick_delta(223, 0.0, 0, true);
    let rank3_elder = gardener_tick_delta(224, 0.0, 3, true);
    let rank3_not_elder = gardener_tick_delta(225, 0.0, 3, false);
    assert!((plain - 0.15).abs() < 1e-9);
    assert!(
        (rank0_elder - 0.15).abs() < 1e-9,
        "a rank-0 Elder never slows its own paws"
    );
    assert!((rank3_elder - 0.15 * (4.0 / 1.4)).abs() < 1e-9);
    assert!(
        (rank3_not_elder - 0.15).abs() < 1e-9,
        "rank alone multiplies nothing"
    );
}

// =====================================================================
// KIT — Traits (SIM-KIT-023, SIM-KIT-024)
// =====================================================================

// @spec SIM-KIT-023
#[test]
fn arrivals_roll_a_trait_uniformly_over_the_eight() {
    let mut s = GameState::new(230);
    adopt(&mut s, 40);
    let traits: Vec<KittenTrait> = s.kittens.iter().map(|k| k.trait_id).collect();
    let distinct: std::collections::BTreeSet<_> = traits.iter().map(|t| format!("{t:?}")).collect();
    // 40 uniform draws over 8 traits: missing 4+ of them is astronomically
    // unlikely; the bound stays generous against seed luck.
    assert!(
        distinct.len() >= 5,
        "40 arrivals drew only {} distinct traits: {distinct:?}",
        distinct.len()
    );
}

// @spec SIM-KIT-023, SIM-KIT-024
#[test]
fn trait_effects_apply_only_while_that_kitten_is_the_elder() {
    // Crafty: +5% craft ratio, but only from the Elder's chair.
    let mut crafty = GameState::new(231);
    adopt(&mut crafty, 1);
    crafty.kittens[0].trait_id = KittenTrait::Crafty;
    crafty.ideas.insert(IdeaId::Tinkering);
    give(&mut crafty, ResourceId::Twigs, 30.0);
    crafty.apply(Command::Craft(CraftId::Wands)).unwrap();
    assert!(
        (crafty.stock(ResourceId::Wands) - 1.0).abs() < 1e-9,
        "a non-Elder trait is inert"
    );
    crafty.apply(Command::AppointElder(0)).unwrap();
    crafty.apply(Command::Craft(CraftId::Wands)).unwrap();
    assert!(
        (crafty.stock(ResourceId::Wands) - 2.05).abs() < 1e-9,
        "the Crafty Elder adds 5%"
    );

    // Brassy: +10% on metal crafts (badges/brass/cogs/moonsteel) only.
    let mut brassy = GameState::new(232);
    adopt(&mut brassy, 1);
    brassy.kittens[0].trait_id = KittenTrait::Brassy;
    brassy.apply(Command::AppointElder(0)).unwrap();
    brassy.ideas.insert(IdeaId::Excavation);
    brassy.ideas.insert(IdeaId::Tinkering);
    give(&mut brassy, ResourceId::Buttons, 10.0);
    give(&mut brassy, ResourceId::Embers, 10.0);
    give(&mut brassy, ResourceId::Twigs, 15.0);
    brassy.apply(Command::Craft(CraftId::Brass)).unwrap();
    assert!(
        (brassy.stock(ResourceId::Brass) - 1.1).abs() < 1e-9,
        "brass is a metal craft"
    );
    brassy.apply(Command::Craft(CraftId::Wands)).unwrap();
    assert!(
        (brassy.stock(ResourceId::Wands) - 1.0).abs() < 1e-9,
        "wands are not"
    );

    // Curious: idea curiosity costs −5% — 15-cost Pantry Organization
    // becomes affordable (and is spent) at 14.25.
    let mut curious = GameState::new(233);
    adopt(&mut curious, 1);
    curious.kittens[0].trait_id = KittenTrait::Curious;
    grant(&mut curious, StructureId::CardboardBox, 1);
    give(&mut curious, ResourceId::Curiosity, 14.25);
    let err = curious
        .apply(Command::ResearchIdea(IdeaId::PantryOrganization))
        .unwrap_err();
    assert!(
        matches!(err, CommandError::InsufficientResources { .. }),
        "inert off the chair"
    );
    curious.apply(Command::AppointElder(0)).unwrap();
    curious
        .apply(Command::ResearchIdea(IdeaId::PantryOrganization))
        .unwrap();
    assert!(
        curious.stock(ResourceId::Curiosity).abs() < 1e-9,
        "exactly 14.25 spent"
    );

    // Bold: +0.5 pounce ratio through the effect-key system (SIM-EXP-004).
    let mut bold = GameState::new(234);
    adopt(&mut bold, 1);
    bold.kittens[0].trait_id = KittenTrait::Bold;
    assert!((bold.effect(Effect::PounceRatio) - 0.0).abs() < 1e-9);
    bold.apply(Command::AppointElder(0)).unwrap();
    assert!((bold.effect(Effect::PounceRatio) - 0.5).abs() < 1e-9);

    // None: some cats simply are. An Elder with no trait changes nothing.
    let mut none = GameState::new(235);
    adopt(&mut none, 1);
    none.kittens[0].trait_id = KittenTrait::None;
    none.apply(Command::AppointElder(0)).unwrap();
    none.ideas.insert(IdeaId::Tinkering);
    give(&mut none, ResourceId::Twigs, 15.0);
    none.apply(Command::Craft(CraftId::Wands)).unwrap();
    assert!((none.stock(ResourceId::Wands) - 1.0).abs() < 1e-9);
    assert!((none.effect(Effect::PounceRatio) - 0.0).abs() < 1e-9);
}

// @spec SIM-KIT-024
#[test]
fn replacing_the_elder_swaps_the_active_trait() {
    let mut s = GameState::new(236);
    adopt(&mut s, 2);
    s.kittens[0].trait_id = KittenTrait::Crafty;
    s.kittens[1].trait_id = KittenTrait::None;
    s.ideas.insert(IdeaId::Tinkering);
    s.apply(Command::AppointElder(0)).unwrap();
    give(&mut s, ResourceId::Twigs, 30.0);
    s.apply(Command::Craft(CraftId::Wands)).unwrap();
    assert!((s.stock(ResourceId::Wands) - 1.05).abs() < 1e-9);
    s.apply(Command::AppointElder(1)).unwrap();
    s.apply(Command::Craft(CraftId::Wands)).unwrap();
    assert!(
        (s.stock(ResourceId::Wands) - 2.05).abs() < 1e-9,
        "the old Elder's trait deactivated with the handover"
    );
}

// =====================================================================
// KIT — Jamborees (SIM-KIT-025) and the revised luxury drain (SIM-ECO-026)
// =====================================================================

// @spec SIM-KIT-025
#[test]
fn jamboree_is_gated_costed_and_runs_a_hundred_days() {
    // Locked before Tea Ceremonies, even with the full bill in paw.
    let mut s = GameState::new(240);
    give(&mut s, ResourceId::Pep, 800.0);
    give(&mut s, ResourceId::Charm, 1500.0);
    give(&mut s, ResourceId::Quills, 40.0);
    assert_eq!(
        s.apply(Command::HostJamboree).unwrap_err(),
        CommandError::Locked
    );
    assert_eq!(s.stock(ResourceId::Pep), 800.0, "a refusal spends nothing");

    // Short on quills: all-or-nothing.
    s.ideas.insert(IdeaId::TeaCeremonies);
    give(&mut s, ResourceId::Quills, 39.0);
    let err = s.apply(Command::HostJamboree).unwrap_err();
    assert!(matches!(err, CommandError::InsufficientResources { .. }));
    assert_eq!(s.stock(ResourceId::Charm), 1500.0);

    // Fully affordable: 800 pep + 1500 charm + 40 quills buy 100 days.
    give(&mut s, ResourceId::Quills, 40.0);
    let start = s.ticks;
    let events = s.apply(Command::HostJamboree).unwrap();
    assert!(events.contains(&Event::JamboreeStarted {
        until_tick: start + 1000
    }));
    assert_eq!(s.jamboree_until, start + 1000, "100 days × 10 ticks");
    assert_eq!(s.stock(ResourceId::Pep), 0.0);
    assert_eq!(s.stock(ResourceId::Charm), 0.0);
    assert_eq!(s.stock(ResourceId::Quills), 0.0);

    // Double-booking while one runs is Locked ("already celebrating").
    give(&mut s, ResourceId::Pep, 800.0);
    give(&mut s, ResourceId::Charm, 1500.0);
    give(&mut s, ResourceId::Quills, 40.0);
    assert_eq!(
        s.apply(Command::HostJamboree).unwrap_err(),
        CommandError::Locked
    );

    // It ends at 100 days, announced, and the window resets to none.
    let events = run_calm_ticks(&mut s, 999);
    assert!(!events.contains(&Event::JamboreeEnded), "not a day early");
    let events = run_calm_ticks(&mut s, 11);
    assert!(events.contains(&Event::JamboreeEnded));
    assert_eq!(s.jamboree_until, 0, "0 = no jamboree");

    // And a fresh one may be hosted after (restocked: the ticks in between
    // clamped pep and charm back to their caps).
    give(&mut s, ResourceId::Pep, 800.0);
    give(&mut s, ResourceId::Charm, 1500.0);
    give(&mut s, ResourceId::Quills, 40.0);
    s.apply(Command::HostJamboree).unwrap();
}

// @spec SIM-KIT-025, SIM-KIT-011
#[test]
fn jamboree_adds_thirty_happiness_scaled_by_the_jamboree_ratio() {
    let mut s = GameState::new(241);
    adopt(&mut s, 1); // one loafer: happiness 101
    let base = s.happiness();
    assert!((base - 101.0).abs() < 1e-9);
    s.jamboree_until = s.ticks + 10_000;
    assert!(
        (s.happiness() - (base + 30.0)).abs() < 1e-9,
        "+30 points while celebrating"
    );
    // Tea Houses raise the jamboree ratio by 1% each: +30 × 1.02.
    grant(&mut s, StructureId::TeaHouse, 2);
    assert!((s.happiness() - (base + 30.0 * 1.02)).abs() < 1e-9);
    // Over: the bonus lapses cleanly.
    s.jamboree_until = 0;
    assert!((s.happiness() - base).abs() < 1e-9);
}

// @spec SIM-KIT-025, SIM-KIT-004
#[test]
fn jamboree_doubles_arrival_chance_plus_tea_house_arrival_effects() {
    let mut s = GameState::new(242);
    give(&mut s, ResourceId::Catnip, 50.0); // base 15%
    assert!((s.arrival_chance() - 0.15).abs() < 1e-9);
    s.jamboree_until = s.ticks + 10_000;
    assert!(
        (s.arrival_chance() - 0.30).abs() < 1e-9,
        "×2 while a jamboree runs"
    );
    // Tea Houses add +0.1% each to the jamboree arrival factor: ×(2 + 0.002).
    grant(&mut s, StructureId::TeaHouse, 2);
    assert!((s.arrival_chance() - 0.15 * 2.002).abs() < 1e-9);
}

// @spec SIM-ECO-026, SIM-KIT-025
#[test]
fn luxury_drain_scales_by_one_plus_effects_and_floors_at_zero() {
    // A jamboree adds +0.2 to the drain sum; a Gift Stand eases −0.04:
    // one Home kitten drains feathers at 0.001 × (1 − 0.04 + 0.2).
    let mut s = GameState::new(243);
    adopt(&mut s, 1);
    s.kittens[0].job = None;
    grant(&mut s, StructureId::GiftStand, 1);
    s.jamboree_until = s.ticks + 10_000;
    off_boundary(&mut s);
    give(&mut s, ResourceId::Catnip, 150.0);
    give(&mut s, ResourceId::Feathers, 2.0);
    give(&mut s, ResourceId::Bells, 2.0);
    run_ticks(&mut s, 1);
    assert!((s.stock(ResourceId::Feathers) - (2.0 - 0.001 * 1.16)).abs() < 1e-9);
    assert!((s.stock(ResourceId::Bells) - (2.0 - 0.0007 * 1.16)).abs() < 1e-9);

    // Enough Gift Stands push the multiplier to (and below) zero: the scale
    // is max(0, 1 + Σ effects) — luxuries stop draining, nothing refills.
    let mut still = GameState::new(244);
    adopt(&mut still, 1);
    still.kittens[0].job = None;
    grant(&mut still, StructureId::GiftStand, 25); // 1 − 1.00 = 0 exactly
    off_boundary(&mut still);
    give(&mut still, ResourceId::Catnip, 150.0);
    give(&mut still, ResourceId::Feathers, 2.0);
    run_ticks(&mut still, 1);
    assert_eq!(
        still.stock(ResourceId::Feathers),
        2.0,
        "a zero multiplier stops the drain"
    );

    let mut negative = GameState::new(245);
    adopt(&mut negative, 1);
    negative.kittens[0].job = None;
    grant(&mut negative, StructureId::GiftStand, 30); // 1 − 1.2 < 0 clamps to 0
    off_boundary(&mut negative);
    give(&mut negative, ResourceId::Catnip, 150.0);
    give(&mut negative, ResourceId::Feathers, 2.0);
    run_ticks(&mut negative, 1);
    assert_eq!(
        negative.stock(ResourceId::Feathers),
        2.0,
        "never negative, never a faucet"
    );
}

// =====================================================================
// KIT — Happiness revised (SIM-KIT-011) and arrivals revised (SIM-KIT-004)
// =====================================================================

// @spec SIM-KIT-011
#[test]
fn each_distinct_stocked_luxury_adds_ten_points_dust_guarded() {
    let mut s = GameState::new(250);
    adopt(&mut s, 1); // one loafer: 101
    assert!((s.happiness() - 101.0).abs() < 1e-9);
    give(&mut s, ResourceId::Feathers, 2.0);
    assert!(
        (s.happiness() - 111.0).abs() < 1e-9,
        "+10 per distinct luxury"
    );
    give(&mut s, ResourceId::Bells, 0.5);
    assert!((s.happiness() - 121.0).abs() < 1e-9);
    give(&mut s, ResourceId::Mint, 1e-10); // dust, not stock
    assert!(
        (s.happiness() - 121.0).abs() < 1e-9,
        "the dust guard applies"
    );
    give(&mut s, ResourceId::Mint, 1.0);
    assert!((s.happiness() - 131.0).abs() < 1e-9);
    // Quantity is irrelevant; distinctness is the whole bonus.
    give(&mut s, ResourceId::Feathers, 1e9);
    assert!((s.happiness() - 131.0).abs() < 1e-9);
}

// @spec SIM-KIT-011
#[test]
fn story_circles_and_moonlight_stages_relieve_crowding() {
    // 15 Home kittens, all employed (no loafer points): crowding 3 × 10 = 30.
    let mut s = GameState::new(251);
    adopt(&mut s, 15);
    for k in &mut s.kittens {
        k.job = Some(JobId::Gardener);
        k.job_seq = Some(1);
    }
    assert!((s.happiness() - 70.0).abs() < 1e-9);

    // 5 Story Circles: relief = limited_dr(0.048 × 5, 0.9) = 0.24 (linear).
    grant(&mut s, StructureId::StoryCircle, 5);
    assert!((s.happiness() - (100.0 - 30.0 * (1.0 - 0.24))).abs() < 1e-9);

    // Moonlight Stages pool into the same knob: 0.24 + 0.075 × 4 = 0.54.
    grant(&mut s, StructureId::MoonlightStage, 4);
    assert!((s.happiness() - (100.0 - 30.0 * (1.0 - 0.54))).abs() < 1e-9);

    // The pooled relief is DR-limited below 0.9: crowding never vanishes.
    let mut packed = GameState::new(252);
    adopt(&mut packed, 15);
    for k in &mut packed.kittens {
        k.job = Some(JobId::Gardener);
        k.job_seq = Some(1);
    }
    grant(&mut packed, StructureId::StoryCircle, 100);
    let relief = limited_dr(0.048 * 100.0, 0.9);
    assert!(relief < 0.9, "the DR limit is never reached");
    assert!((packed.happiness() - (100.0 - 30.0 * (1.0 - relief))).abs() < 1e-9);
    assert!(
        packed.happiness() < 97.0 + 1e-9,
        "at least 10% of the crowding always bites"
    );
}

// @spec SIM-KIT-011
#[test]
fn the_full_happiness_formula_composes_exactly() {
    // 7 Home kittens (5 employed, 2 loafing), 5 Story Circles, 2 Scratching
    // Posts, feathers + bells stocked, a jamboree with 2 Tea Houses:
    // 100 − 3×2×(1 − 0.24) + 2 + 10 + 20 + 30×1.02 = 158.04.
    let mut s = GameState::new(253);
    adopt(&mut s, 7);
    for k in &mut s.kittens[..5] {
        k.job = Some(JobId::Gardener);
        k.job_seq = Some(1);
    }
    grant(&mut s, StructureId::StoryCircle, 5);
    grant(&mut s, StructureId::ScratchingPost, 2);
    grant(&mut s, StructureId::TeaHouse, 2);
    give(&mut s, ResourceId::Feathers, 1.0);
    give(&mut s, ResourceId::Bells, 1.0);
    s.jamboree_until = s.ticks + 10_000;
    let expected = 100.0 - 6.0 * (1.0 - 0.24) + 2.0 + 10.0 + 20.0 + 30.0 * 1.02;
    assert!(
        (s.happiness() - expected).abs() < 1e-9,
        "expected {expected}, measured {}",
        s.happiness()
    );
}

// @spec SIM-KIT-004
#[test]
fn arrival_chance_pipeline_composes_in_the_specified_order() {
    // Inside-cap effects: the Warm Welcome Mat's +5% joins the base sum
    // before the 60% cap...
    let mut s = GameState::new(254);
    give(&mut s, ResourceId::Catnip, 50.0);
    assert!((s.arrival_chance() - 0.15).abs() < 1e-9);
    s.refinements.insert(RefinementId::WarmWelcomeMat);
    assert!(
        (s.arrival_chance() - 0.20).abs() < 1e-9,
        "+5% inside the cap"
    );
    give(&mut s, ResourceId::Catnip, 2000.0);
    assert!(
        (s.arrival_chance() - 0.60).abs() < 1e-9,
        "the cap swallows inside effects"
    );

    // ...the Watch's +15% lands on top of the cap...
    s.ideas.insert(IdeaId::NeighborhoodWatch);
    assert!((s.arrival_chance() - 0.75).abs() < 1e-9);

    // ...mess (level 1+) multiplies the whole sum, cap and bonus included...
    s.mess = 1e5;
    assert!(
        (s.arrival_chance() - 0.75 * 0.75).abs() < 1e-9,
        "(0.60 + 0.15) × 0.75"
    );

    // ...a jamboree doubles it, and the result clamps to 1.
    s.jamboree_until = s.ticks + 10_000;
    assert!(
        (s.arrival_chance() - 1.0).abs() < 1e-9,
        "0.5625 × 2 = 1.125 clamps to 1"
    );
    s.mess = 0.0;
    assert!(
        (s.arrival_chance() - 1.0).abs() < 1e-9,
        "0.75 × 2 clamps to 1"
    );
    s.jamboree_until = 0;
    assert!((s.arrival_chance() - 0.75).abs() < 1e-9);
}

// =====================================================================
// EXP — Pouncing expeditions (SIM-EXP-001..006)
// =====================================================================

// @spec SIM-EXP-001
#[test]
fn pounce_is_idea_gated_and_spends_pep_per_squad() {
    // Locked before Pouncing Practice, a perfect no-op (RNG included).
    let mut s = GameState::new(260);
    give(&mut s, ResourceId::Pep, 500.0);
    let snapshot = s.clone();
    assert_eq!(
        s.apply(Command::Pounce(2)).unwrap_err(),
        CommandError::Locked
    );
    assert_eq!(s, snapshot);

    // Unaffordable squads reject with InsufficientResources, mutating nothing.
    s.ideas.insert(IdeaId::PouncingPractice);
    give(&mut s, ResourceId::Pep, 199.0);
    let snapshot = s.clone();
    let err = s.apply(Command::Pounce(2)).unwrap_err();
    assert!(matches!(err, CommandError::InsufficientResources { .. }));
    assert_eq!(s, snapshot, "a rejected pounce leaves the RNG untouched");

    // 100 pep per squad, spent in full.
    give(&mut s, ResourceId::Pep, 250.0);
    s.apply(Command::Pounce(2)).unwrap();
    assert!((s.stock(ResourceId::Pep) - 50.0).abs() < 1e-9);
}

// @spec SIM-EXP-005, SIM-EXP-006
#[test]
fn pounce_returns_instantly_with_an_event_matching_the_stock_deltas() {
    let mut s = GameState::new(261);
    s.ideas.insert(IdeaId::PouncingPractice);
    give(&mut s, ResourceId::Pep, 300.0);
    let before: Vec<f64> = [
        ResourceId::Feathers,
        ResourceId::Bells,
        ResourceId::Shinies,
        ResourceId::Unicorns,
    ]
    .iter()
    .map(|r| s.stock(*r))
    .collect();
    let events = s.apply(Command::Pounce(3)).unwrap();
    let returned = events
        .iter()
        .find_map(|e| match e {
            Event::PounceReturned {
                squads,
                feathers,
                bells,
                shinies,
                unicorns,
            } => Some((*squads, *feathers, *bells, *shinies, *unicorns)),
            _ => None,
        })
        .expect("one PounceReturned event per pounce");
    let (squads, feathers, bells, shinies, unicorns) = returned;
    assert_eq!(squads, 3);
    assert!((s.stock(ResourceId::Feathers) - before[0] - feathers).abs() < 1e-9);
    assert!((s.stock(ResourceId::Bells) - before[1] - bells).abs() < 1e-9);
    assert!((s.stock(ResourceId::Shinies) - before[2] - shinies).abs() < 1e-9);
    assert!((s.stock(ResourceId::Unicorns) - before[3] - unicorns as f64).abs() < 1e-9);
    assert!(feathers > 0.0, "every squad finds some feathers");

    // Instant: the yields landed at command time; nothing trickles in later
    // (no kittens, no structures — nothing else moves these stocks).
    let held: Vec<f64> = [
        ResourceId::Feathers,
        ResourceId::Bells,
        ResourceId::Unicorns,
    ]
    .iter()
    .map(|r| s.stock(*r))
    .collect();
    run_ticks(&mut s, 25);
    assert_eq!(s.stock(ResourceId::Feathers), held[0], "no return timers");
    assert_eq!(s.stock(ResourceId::Bells), held[1]);
    assert_eq!(s.stock(ResourceId::Unicorns), held[2]);
}

// @spec SIM-EXP-002, SIM-EXP-003
#[test]
fn pounce_yields_match_their_distributions_over_many_squads() {
    // 400 pounces × 5 squads at pounce_ratio 0. Expected per squad:
    // feathers 8 × 1/2 = 4, bells 5 × 0.45/2 = 1.125, shinies 0.5 × 0.25/2 =
    // 0.0625, unicorns 0.05. Bounds are generous against seed luck.
    let mut s = GameState::new(262);
    s.ideas.insert(IdeaId::PouncingPractice);
    let (mut feathers, mut bells, mut shinies, mut unicorns) = (0.0, 0.0, 0.0, 0u32);
    for _ in 0..400 {
        give(&mut s, ResourceId::Pep, 500.0);
        give(&mut s, ResourceId::Shinies, 0.0); // keep shinies clear of the cap
        let events = s.apply(Command::Pounce(5)).unwrap();
        for e in &events {
            if let Event::PounceReturned {
                feathers: f,
                bells: b,
                shinies: sh,
                unicorns: u,
                ..
            } = e
            {
                feathers += f;
                bells += b;
                shinies += sh;
                unicorns += u;
            }
        }
    }
    let squads = 2000.0;
    assert!(
        (3.4..=4.6).contains(&(feathers / squads)),
        "feathers/squad: {}",
        feathers / squads
    );
    assert!(
        (0.85..=1.4).contains(&(bells / squads)),
        "bells/squad: {}",
        bells / squads
    );
    assert!(
        (0.04..=0.09).contains(&(shinies / squads)),
        "shinies/squad: {}",
        shinies / squads
    );
    assert!(
        (50..=170).contains(&unicorns),
        "unicorns over 2000 squads: {unicorns} (each squad ~5%)"
    );
}

// @spec SIM-EXP-002, SIM-EXP-004
#[test]
fn pounce_ratio_raises_the_haul_and_shinies_clamp_at_cap() {
    // pounce_ratio 3 (Bolas +1, Pounce Pads +2): feathers per squad mean
    // (8 + 19.5)/2 = 13.75 — far from the ratio-0 mean of 4.
    let mut s = GameState::new(263);
    s.ideas.insert(IdeaId::PouncingPractice);
    s.refinements.insert(RefinementId::BolasOfYarn);
    s.refinements.insert(RefinementId::PouncePads);
    assert!(
        (s.effect(Effect::PounceRatio) - 3.0).abs() < 1e-9,
        "SIM-EXP-004: Σ effects"
    );
    let mut feathers = 0.0;
    for _ in 0..100 {
        give(&mut s, ResourceId::Pep, 400.0);
        give(&mut s, ResourceId::Shinies, 0.0);
        let events = s.apply(Command::Pounce(4)).unwrap();
        for e in &events {
            if let Event::PounceReturned { feathers: f, .. } = e {
                feathers += f;
            }
        }
    }
    let per_squad = feathers / 400.0;
    assert!(
        (11.0..=16.5).contains(&per_squad),
        "boosted feathers/squad: {per_squad}"
    );

    // Shinies clamp to cap at yield time: pounding away at a full shiny box
    // never overflows it (base cap 15).
    let mut full = GameState::new(264);
    full.ideas.insert(IdeaId::PouncingPractice);
    give(&mut full, ResourceId::Shinies, 15.0);
    for _ in 0..50 {
        give(&mut full, ResourceId::Pep, 500.0);
        full.apply(Command::Pounce(5)).unwrap();
        assert!(
            full.stock(ResourceId::Shinies) <= 15.0 + 1e-9,
            "clamped to cap"
        );
    }
    // Feathers are uncapped: the same pounces piled them up freely.
    assert!(full.stock(ResourceId::Feathers) > 100.0);
}

// =====================================================================
// ECO/KIT — Tinkerer background crafting (SIM-ECO-043, SIM-KIT-026)
// =====================================================================

// @spec SIM-ECO-043
#[test]
fn craft_recipes_carry_their_tabled_tiers() {
    use CraftId::*;
    let tier = |c: CraftId| content::craft_def(c).tier;
    for c in [Wands, Tiles, Badges, Quills, Cairns] {
        assert_eq!(tier(c), 1, "{c:?} is tier 1");
    }
    for c in [Brass, Lattices, Stories] {
        assert_eq!(tier(c), 2, "{c:?} is tier 2");
    }
    for c in [Cogs, Pavers, Anthologies, Kites] {
        assert_eq!(tier(c), 3, "{c:?} is tier 3");
    }
    for c in [Moonsteel, Schemes, Moonmilk] {
        assert_eq!(tier(c), 4, "{c:?} is tier 4");
    }
    for c in [Balloons, Dreamweave, Cometsilk] {
        assert_eq!(tier(c), 5, "{c:?} is tier 5");
    }
}

// @spec SIM-ECO-043
#[test]
fn assign_tinkerer_binds_lifo_and_unassign_releases_highest_job_seq() {
    let mut s = tinker_world(270, 3, 3);
    // Binding is Locked while the recipe is (Cogs needs Cogwheels).
    assert_eq!(
        s.apply(Command::AssignTinkerer(CraftId::Cogs)).unwrap_err(),
        CommandError::Locked
    );

    // The most recently arrived unbound Home Tinkerer binds first.
    s.apply(Command::AssignTinkerer(CraftId::Wands)).unwrap();
    assert_eq!(s.kittens[2].tinkering, Some(CraftId::Wands));
    assert_eq!(s.kittens[1].tinkering, None);
    s.apply(Command::AssignTinkerer(CraftId::Wands)).unwrap();
    assert_eq!(s.kittens[1].tinkering, Some(CraftId::Wands));
    s.apply(Command::AssignTinkerer(CraftId::Tiles)).unwrap();
    assert_eq!(s.kittens[0].tinkering, Some(CraftId::Tiles));

    // All three bound: no unbound Home Tinkerer remains.
    assert_eq!(
        s.apply(Command::AssignTinkerer(CraftId::Wands))
            .unwrap_err(),
        CommandError::NoIdleKittens
    );

    // Unassign releases the highest-job_seq bound holder of that recipe.
    // Assignment ran newest-first, so job_seq: kittens[2]=0, [1]=1, [0]=2.
    s.apply(Command::UnassignTinkerer(CraftId::Wands)).unwrap();
    assert_eq!(
        s.kittens[1].tinkering, None,
        "the later assignee unbinds first"
    );
    assert_eq!(s.kittens[2].tinkering, Some(CraftId::Wands));
    s.apply(Command::UnassignTinkerer(CraftId::Wands)).unwrap();
    assert_eq!(s.kittens[2].tinkering, None);
    assert_eq!(
        s.apply(Command::UnassignTinkerer(CraftId::Wands))
            .unwrap_err(),
        CommandError::NoWorkersAssigned
    );
}

// @spec SIM-ECO-043, SIM-ENG-006
#[test]
fn tinkerer_seats_come_from_toy_workshops_with_noseats_in_precedence() {
    // No workshops at all: an idle Tinkerer has nowhere to sit.
    let mut s = tinker_world(271, 0, 1);
    assert_eq!(
        s.apply(Command::AssignTinkerer(CraftId::Wands))
            .unwrap_err(),
        CommandError::NoSeats
    );

    // One seat, two Tinkerers: the second binding would exceed the seats.
    let mut s = tinker_world(272, 1, 2);
    s.apply(Command::AssignTinkerer(CraftId::Wands)).unwrap();
    assert_eq!(
        s.apply(Command::AssignTinkerer(CraftId::Tiles))
            .unwrap_err(),
        CommandError::NoSeats
    );

    // Precedence: Locked beats NoSeats (locked recipe, no seats)...
    let mut s = tinker_world(273, 0, 1);
    assert_eq!(
        s.apply(Command::AssignTinkerer(CraftId::Cogs)).unwrap_err(),
        CommandError::Locked
    );
    // ...and NoIdleKittens beats NoSeats (everyone bound AND seats full).
    let mut s = tinker_world(274, 1, 1);
    s.apply(Command::AssignTinkerer(CraftId::Wands)).unwrap();
    assert_eq!(
        s.apply(Command::AssignTinkerer(CraftId::Tiles))
            .unwrap_err(),
        CommandError::NoIdleKittens
    );
}

/// Run a lone bound Tinkerer for `ticks` and return (output stock, input stock).
fn tinker_run(
    mut s: GameState,
    recipe: CraftId,
    input: ResourceId,
    stock: f64,
    output: ResourceId,
    ticks: u64,
) -> (f64, f64) {
    s.apply(Command::AssignTinkerer(recipe)).unwrap();
    off_boundary(&mut s);
    give(&mut s, ResourceId::Catnip, 100.0);
    give(&mut s, input, stock);
    run_calm_ticks(&mut s, ticks);
    (s.stock(output), s.stock(input))
}

// @spec SIM-ECO-043, SIM-KIT-026
#[test]
fn bound_tinkerers_craft_at_the_base_rate_with_craft_ratio_boosted_output() {
    // Rank 0 on a tier-1 recipe: effectiveness 1 − 0.15 × max(0, 1−1) = 100%.
    // 200 ticks × 0.0017 batches: 5.1 twigs in, 0.34 × 1.05 wands out (the
    // one Toy Workshop's +5% craft ratio boosts output, never input).
    let s = tinker_world(275, 1, 1);
    let (wands, twigs) = tinker_run(
        s,
        CraftId::Wands,
        ResourceId::Twigs,
        50.0,
        ResourceId::Wands,
        200,
    );
    assert!(
        (wands - 0.34 * 1.05).abs() < 1e-9,
        "wands after 200 ticks: {wands}"
    );
    assert!(
        (twigs - (50.0 - 5.1)).abs() < 1e-9,
        "inputs drawn at the unboosted rate"
    );
}

// @spec SIM-KIT-026, SIM-ECO-043
#[test]
fn recipe_tiers_above_rank_plus_one_reduce_effectiveness() {
    // Tier 3 at rank 0: 1 − 0.15 × 2 = ×0.7.
    let mut s = tinker_world(276, 1, 1);
    s.ideas.insert(IdeaId::Cogwheels);
    let (cogs, brass) = tinker_run(
        s,
        CraftId::Cogs,
        ResourceId::Brass,
        100.0,
        ResourceId::Cogs,
        200,
    );
    assert!((cogs - 0.34 * 0.7 * 1.05).abs() < 1e-9, "cogs: {cogs}");
    assert!((brass - (100.0 - 0.34 * 0.7 * 8.0)).abs() < 1e-9);

    // Tier 5 at rank 0: 1 − 0.15 × 4 = ×0.4.
    let mut s = tinker_world(278, 1, 1);
    s.ideas.insert(IdeaId::BalloonCraft);
    give(&mut s, ResourceId::Kites, 100.0);
    give(&mut s, ResourceId::Schemes, 100.0);
    let (balloons, _) = tinker_run(
        s,
        CraftId::Balloons,
        ResourceId::Moonsteel,
        1000.0,
        ResourceId::Balloons,
        200,
    );
    assert!(
        (balloons - 0.34 * 0.4 * 1.05).abs() < 1e-9,
        "balloons: {balloons}"
    );

    // A rank matching the tier works cleanly: tier 4 at rank 3 is 100%.
    let mut s = tinker_world(279, 1, 1);
    s.ideas.insert(IdeaId::Moonmetallurgy);
    s.kittens[0].rank = 3;
    give(&mut s, ResourceId::Moonstone, 8.0);
    let (steel, _) = tinker_run(
        s,
        CraftId::Moonsteel,
        ResourceId::Brass,
        100.0,
        ResourceId::Moonsteel,
        200,
    );
    assert!((steel - 0.34 * 1.05).abs() < 1e-9, "moonsteel: {steel}");
}

// @spec SIM-ECO-043, SIM-CUR-010
#[test]
fn tinkerer_speed_refinements_multiply_the_batch_rate() {
    // Nimble Paws: Tinkerer speed +100% ⇒ 0.0034 batches/tick.
    let mut s = tinker_world(280, 1, 1);
    s.refinements.insert(RefinementId::NimblePaws);
    let (wands, twigs) = tinker_run(
        s,
        CraftId::Wands,
        ResourceId::Twigs,
        50.0,
        ResourceId::Wands,
        200,
    );
    assert!((wands - 0.68 * 1.05).abs() < 1e-9, "doubled rate: {wands}");
    assert!((twigs - (50.0 - 10.2)).abs() < 1e-9);

    // The paws ladder stacks additively on one key: +100% +100% +200% = ×5.
    let mut s = tinker_world(281, 1, 1);
    s.refinements.insert(RefinementId::NimblePaws);
    s.refinements.insert(RefinementId::PatientPaws);
    s.refinements.insert(RefinementId::ClockworkPaws);
    let (wands, _) = tinker_run(
        s,
        CraftId::Wands,
        ResourceId::Twigs,
        50.0,
        ResourceId::Wands,
        200,
    );
    assert!(
        (wands - 0.34 * 5.0 * 1.05).abs() < 1e-9,
        "the ladder stacks: {wands}"
    );
}

// @spec SIM-ECO-043
#[test]
fn starved_tinkerers_throttle_quietly_and_away_tinkerers_idle_bound() {
    // No inputs at all: nothing crafts, nothing errors, the binding holds.
    let mut s = tinker_world(282, 1, 1);
    s.apply(Command::AssignTinkerer(CraftId::Wands)).unwrap();
    off_boundary(&mut s);
    give(&mut s, ResourceId::Catnip, 100.0);
    give(&mut s, ResourceId::Twigs, 0.0);
    run_calm_ticks(&mut s, 50);
    assert_eq!(s.stock(ResourceId::Wands), 0.0, "a starved Tinkerer idles");
    assert_eq!(
        s.kittens[0].tinkering,
        Some(CraftId::Wands),
        "the binding persists"
    );

    // A finite twig pile is drawn down through the throttle and converted in
    // full — output tracks exactly the inputs actually consumed.
    let mut s = tinker_world(283, 1, 1);
    s.apply(Command::AssignTinkerer(CraftId::Wands)).unwrap();
    off_boundary(&mut s);
    give(&mut s, ResourceId::Catnip, 100.0);
    give(&mut s, ResourceId::Twigs, 2.55); // exactly 100 ticks of demand
    run_calm_ticks(&mut s, 200);
    assert!(s.stock(ResourceId::Twigs) < 1e-9, "the pile is used up");
    assert!(
        (s.stock(ResourceId::Wands) - (2.55 / 15.0) * 1.05).abs() < 1e-6,
        "output matches the inputs actually drawn: {}",
        s.stock(ResourceId::Wands)
    );

    // An Away Tinkerer's recipe idles entirely, but the binding (and seat)
    // persist for its return.
    let mut s = tinker_world(284, 1, 2);
    s.apply(Command::AssignTinkerer(CraftId::Wands)).unwrap();
    s.kittens[1].location = Location::NeighborsYard;
    s.kittens[1].away_seq = Some(9);
    off_boundary(&mut s);
    // Lean meadow keeps the bound Tinkerer at the neighbor's for the whole
    // window (a full bowl would recover it home at a day close).
    give(&mut s, ResourceId::Catnip, 0.0);
    give(&mut s, ResourceId::Twigs, 50.0);
    run_calm_ticks(&mut s, 100);
    assert_eq!(s.stock(ResourceId::Wands), 0.0, "no output while away");
    assert_eq!(
        s.stock(ResourceId::Twigs),
        50.0,
        "no inputs drawn while away"
    );
    assert_eq!(s.kittens[1].tinkering, Some(CraftId::Wands));
    assert_eq!(
        s.apply(Command::AssignTinkerer(CraftId::Tiles))
            .unwrap_err(),
        CommandError::NoSeats,
        "the absent Tinkerer still holds its seat"
    );
}

// =====================================================================
// ECO — Pep caps from housing (SIM-ECO-044)
// =====================================================================

// @spec SIM-ECO-044
#[test]
fn housing_extends_the_pep_cap() {
    let mut s = GameState::new(285);
    assert!(
        (s.cap(ResourceId::Pep) - 100.0).abs() < 1e-9,
        "base pep cap"
    );
    grant(&mut s, StructureId::CozyCushion, 2); // +25 each
    grant(&mut s, StructureId::WindowPerch, 3); // +25 each
    grant(&mut s, StructureId::CatTree, 2); // +75 each
    assert!(
        (s.cap(ResourceId::Pep) - (100.0 + 50.0 + 75.0 + 150.0)).abs() < 1e-9,
        "cushions and perches +25, trees +75"
    );
    // The cap is real: stock clamps to it during the tick.
    s.ticks = 3;
    give(&mut s, ResourceId::Pep, 10_000.0);
    run_ticks(&mut s, 1);
    assert!((s.stock(ResourceId::Pep) - 375.0).abs() < 1e-9);
}

// =====================================================================
// ECO — The Cogworks year-close auto-craft (SIM-ECO-039)
// =====================================================================

// @spec SIM-ECO-039
#[test]
fn cogworks_auto_craft_capped_raws_at_year_close() {
    // One Cogworks, twigs and pebbles sitting at cap, recipes unlocked:
    // year close converts 2% of each stock through its recipe.
    let mut s = GameState::new(290);
    s.ideas.insert(IdeaId::Tinkering); // Wands + Tiles recipes
    grant(&mut s, StructureId::Cogworks, 1);
    s.ticks = 998; // two ticks before the year-1 boundary at tick 1000
    give(&mut s, ResourceId::Twigs, 50.0); // 100% of the base cap
    give(&mut s, ResourceId::Pebbles, 60.0);
    give(&mut s, ResourceId::Catnip, 200.0); // at cap, but catnip has no crafted form
    run_ticks(&mut s, 1); // tick 999: not yet the year close
    assert_eq!(
        s.stock(ResourceId::Wands),
        0.0,
        "nothing before the year boundary"
    );
    let events = run_ticks(&mut s, 1); // tick 1000: NewYear + auto-craft
    assert!(events.contains(&Event::NewYear { year: 1 }));
    // 2% × 50 twigs = 1.0 twigs → 1/15 batches; no craft-ratio sources here.
    assert!(
        (s.stock(ResourceId::Wands) - 1.0 / 15.0).abs() < 1e-9,
        "wands: {}",
        s.stock(ResourceId::Wands)
    );
    assert!(
        (s.stock(ResourceId::Twigs) - 49.0).abs() < 1e-9,
        "the inputs left the pile"
    );
    // 2% × 60 pebbles = 1.2 → 1.2/25 tiles.
    assert!((s.stock(ResourceId::Tiles) - 1.2 / 25.0).abs() < 1e-9);
    assert!((s.stock(ResourceId::Pebbles) - 58.8).abs() < 1e-9);
    // Catnip is not on the single-input list: skipped entirely.
    assert!((s.stock(ResourceId::Catnip) - 200.0).abs() < 1e-9);

    // Once per year: refilled stocks wait for the next year close.
    give(&mut s, ResourceId::Twigs, 50.0);
    run_ticks(&mut s, 200); // 20 days, same year
    assert!(
        (s.stock(ResourceId::Wands) - 1.0 / 15.0).abs() < 1e-9,
        "no mid-year crafts"
    );
}

// @spec SIM-ECO-039
#[test]
fn cogworks_auto_craft_needs_98_percent_scales_with_count_and_caps_at_half() {
    // 97% of cap: below the threshold, skipped.
    let mut low = GameState::new(291);
    low.ideas.insert(IdeaId::Tinkering);
    grant(&mut low, StructureId::Cogworks, 1);
    low.ticks = 999;
    give(&mut low, ResourceId::Twigs, 48.5); // 97% of 50
    run_ticks(&mut low, 1);
    assert_eq!(
        low.stock(ResourceId::Wands),
        0.0,
        "below 98% of cap nothing converts"
    );
    assert!((low.stock(ResourceId::Twigs) - 48.5).abs() < 1e-9);

    // 98% exactly qualifies, and 5 Cogworks convert 10%.
    let mut five = GameState::new(292);
    five.ideas.insert(IdeaId::Tinkering);
    grant(&mut five, StructureId::Cogworks, 5);
    five.ticks = 999;
    give(&mut five, ResourceId::Twigs, 49.0); // 98% of 50
    run_ticks(&mut five, 1);
    assert!((five.stock(ResourceId::Twigs) - 49.0 * 0.9).abs() < 1e-9);
    assert!((five.stock(ResourceId::Wands) - 49.0 * 0.1 / 15.0).abs() < 1e-9);

    // 30 Cogworks would be 60%: the conversion caps at 50%.
    let mut many = GameState::new(293);
    many.ideas.insert(IdeaId::Tinkering);
    grant(&mut many, StructureId::Cogworks, 30);
    many.ticks = 999;
    give(&mut many, ResourceId::Twigs, 50.0);
    run_ticks(&mut many, 1);
    assert!(
        (many.stock(ResourceId::Twigs) - 25.0).abs() < 1e-9,
        "capped at half the stock"
    );
    assert!((many.stock(ResourceId::Wands) - 25.0 / 15.0).abs() < 1e-9);
}

// @spec SIM-ECO-039, SIM-ECO-033
#[test]
fn cogworks_auto_craft_output_rides_the_craft_ratio() {
    let mut s = GameState::new(294);
    s.ideas.insert(IdeaId::Tinkering);
    grant(&mut s, StructureId::Cogworks, 1);
    grant(&mut s, StructureId::CraftCorner, 2); // craft ratio +12%
    s.ticks = 999;
    give(&mut s, ResourceId::Twigs, 50.0);
    run_ticks(&mut s, 1);
    assert!(
        (s.stock(ResourceId::Wands) - (1.0 / 15.0) * 1.12).abs() < 1e-9,
        "overflow becomes ratio-boosted goods: {}",
        s.stock(ResourceId::Wands)
    );
    assert!(
        (s.stock(ResourceId::Twigs) - 49.0).abs() < 1e-9,
        "inputs are never boosted"
    );
}

// =====================================================================
// CUR — Refinements (SIM-CUR-010, SIM-CUR-012)
// =====================================================================

// @spec SIM-CUR-010
#[test]
fn refinement_table_has_35_appended_ids_with_tabled_costs_and_gates() {
    assert_eq!(RefinementId::ALL.len(), 35, "exactly the Refinements table");
    assert_eq!(RefinementId::PebbleCombs as usize, 0);
    assert_eq!(RefinementId::BolasOfYarn as usize, 10);
    assert_eq!(RefinementId::DeepShelves as usize, 14);
    assert_eq!(RefinementId::Bellows as usize, 19);
    assert_eq!(RefinementId::TidyUpRota as usize, 26);
    assert_eq!(RefinementId::GroupLessons as usize, 27);
    assert_eq!(RefinementId::EncouragingMews as usize, 28);
    assert_eq!(RefinementId::WarmWelcomeMat as usize, 32);
    assert_eq!(RefinementId::DewFunnels as usize, 34);

    let d = content::refinement_def(RefinementId::PebbleCombs);
    assert_eq!(d.cost, 100.0);
    assert_eq!(d.materials, &[(ResourceId::Pebbles, 275.0)]);
    assert_eq!(d.gate, IdeaId::CouchSpelunking);

    let d = content::refinement_def(RefinementId::DeepShelves);
    assert_eq!(d.cost, 500.0);
    assert_eq!(
        d.materials,
        &[
            (ResourceId::Twigs, 1000.0),
            (ResourceId::Pebbles, 750.0),
            (ResourceId::Buttons, 50.0)
        ]
    );
    assert_eq!(d.gate, IdeaId::PackRattery);

    let d = content::refinement_def(RefinementId::MoonsteelMittens);
    assert_eq!(d.cost, 50_000.0);
    assert_eq!(d.materials, &[(ResourceId::Moonsteel, 25.0)]);
    assert_eq!(d.gate, IdeaId::Moonmetallurgy);

    let d = content::refinement_def(RefinementId::WarmWelcomeMat);
    assert_eq!(d.cost, 4000.0);
    assert_eq!(d.materials, &[(ResourceId::Stories, 20.0)]);
    assert_eq!(d.gate, IdeaId::TeaCeremonies);

    let d = content::refinement_def(RefinementId::DewFunnels);
    assert_eq!(d.cost, 30_000.0);
    assert_eq!(d.materials, &[(ResourceId::Lattices, 40.0)]);
    assert_eq!(d.gate, IdeaId::DewHarvest);

    for r in RefinementId::ALL {
        assert!(
            !content::refinement_def(r).name.is_empty(),
            "{r:?} names itself"
        );
    }
}

// @spec SIM-CUR-010
#[test]
fn buy_refinement_is_gated_all_or_nothing_and_once_only() {
    // Locked before the gate idea, even fully funded.
    let mut s = GameState::new(300);
    give(&mut s, ResourceId::Curiosity, 100.0);
    give(&mut s, ResourceId::Pebbles, 275.0);
    assert_eq!(
        s.apply(Command::BuyRefinement(RefinementId::PebbleCombs))
            .unwrap_err(),
        CommandError::Locked
    );
    assert_eq!(
        s.stock(ResourceId::Curiosity),
        100.0,
        "a refusal spends nothing"
    );

    // Short on materials: all-or-nothing.
    s.ideas.insert(IdeaId::CouchSpelunking);
    give(&mut s, ResourceId::Pebbles, 200.0);
    let err = s
        .apply(Command::BuyRefinement(RefinementId::PebbleCombs))
        .unwrap_err();
    assert!(matches!(err, CommandError::InsufficientResources { .. }));
    assert_eq!(s.stock(ResourceId::Curiosity), 100.0);
    assert_eq!(s.stock(ResourceId::Pebbles), 200.0);

    // Funded: curiosity and materials spend together, permanently.
    give(&mut s, ResourceId::Pebbles, 275.0);
    let events = s
        .apply(Command::BuyRefinement(RefinementId::PebbleCombs))
        .unwrap();
    assert!(events.contains(&Event::RefinementBought {
        id: RefinementId::PebbleCombs
    }));
    assert!(s.refinements.contains(&RefinementId::PebbleCombs));
    assert!(s.stock(ResourceId::Curiosity).abs() < 1e-9);
    assert!(s.stock(ResourceId::Pebbles).abs() < 1e-9);

    // Purchasable once each: buying again is refused without mutation.
    give(&mut s, ResourceId::Curiosity, 100.0);
    give(&mut s, ResourceId::Pebbles, 275.0);
    let snapshot = s.clone();
    assert_eq!(
        s.apply(Command::BuyRefinement(RefinementId::PebbleCombs))
            .unwrap_err(),
        CommandError::AlreadyResearched
    );
    assert_eq!(s, snapshot);
}

// @spec SIM-CUR-010, SIM-CUR-012
#[test]
fn representative_refinement_effects_land() {
    // Pebble Combs: Gardener +50% ⇒ 0.15 × 1.5 per tick.
    let mut s = GameState::new(301);
    adopt(&mut s, 1);
    s.apply(Command::AssignJob(JobId::Gardener)).unwrap();
    s.refinements.insert(RefinementId::PebbleCombs);
    off_boundary(&mut s);
    give(&mut s, ResourceId::Catnip, 100.0);
    run_ticks(&mut s, 1);
    assert!((s.stock(ResourceId::Catnip) - 100.0 + 0.085 - 0.225).abs() < 1e-9);

    // Bellows: Hearth embers +0.002/t (one fully supplied Hearth: 0.003).
    let mut s = GameState::new(302);
    s.ticks = 3;
    grant(&mut s, StructureId::Hearth, 1);
    s.refinements.insert(RefinementId::Bellows);
    give(&mut s, ResourceId::Twigs, 10.0);
    give(&mut s, ResourceId::Pebbles, 20.0);
    run_ticks(&mut s, 1);
    assert!((s.stock(ResourceId::Embers) - 0.003).abs() < 1e-9);
    assert!(
        (s.stock(ResourceId::Buttons) - 0.02).abs() < 1e-9,
        "buttons are untouched"
    );

    // Tidy-Up Rota flips the Toy Workshop's mess from +2 to −2 per tick.
    let mut messy = GameState::new(303);
    messy.ticks = 3;
    grant(&mut messy, StructureId::ToyWorkshop, 1);
    messy.mess = 1000.0;
    let mut tidy = messy.clone();
    tidy.refinements.insert(RefinementId::TidyUpRota);
    run_ticks(&mut messy, 1);
    run_ticks(&mut tidy, 1);
    assert!(
        (messy.mess - (1000.0 + 2.0 - 0.1)).abs() < 1e-2,
        "mess: {}",
        messy.mess
    );
    assert!(
        (tidy.mess - (1000.0 - 2.0 - 0.1)).abs() < 1e-2,
        "tidied: {}",
        tidy.mess
    );
    // Tidying sweeps toward zero, never below it.
    tidy.mess = 0.5;
    run_ticks(&mut tidy, 5);
    assert!(tidy.mess >= 0.0, "mess never goes negative");
    assert!(tidy.mess < 1e-9);

    // Deep Shelves: +100 twig cap, +150 pebble cap.
    let mut s = GameState::new(304);
    s.refinements.insert(RefinementId::DeepShelves);
    assert!((s.cap(ResourceId::Twigs) - 150.0).abs() < 1e-9);
    assert!((s.cap(ResourceId::Pebbles) - 210.0).abs() < 1e-9);
}

// @spec SIM-CUR-010
#[test]
fn every_refinements_capped_materials_fit_achievable_caps_at_gate_band_plus_one() {
    // Achievable caps one band past the gate: every cap-extending structure
    // of band ≤ gate band + 1 at 20 copies, with the Porch kite ratchet once
    // kites exist (band 1). Uncapped materials accumulate and are exempt;
    // the curiosity price is storage-gated by design (not part of this rule).
    for r in RefinementId::ALL {
        let def = content::refinement_def(r);
        let band = (idea_band(def.gate) + 1).min(4);
        let mut world = GameState::new(305);
        for st in StructureId::ALL {
            if structure_band(st) <= band {
                grant(&mut world, st, 20);
            }
        }
        if band >= 1 {
            world.kites_high_water = 1e9;
        }
        for (res, amount) in def.materials {
            let cap = world.cap(*res);
            if !cap.is_finite() {
                continue;
            }
            assert!(
                *amount <= cap,
                "{r:?} needs {amount} {res:?}, but band {band} caps out at {cap}"
            );
        }
    }
}

// =====================================================================
// CUR — House Rules (SIM-CUR-011, SIM-CUR-012)
// =====================================================================

// @spec SIM-CUR-011
#[test]
fn house_rules_come_in_four_charm_priced_blocking_pairs() {
    assert_eq!(RuleId::ALL.len(), 8);
    assert_eq!(RuleId::EarlyBirds as usize, 0);
    assert_eq!(RuleId::QuietContemplation as usize, 7);
    let pairs = [
        (RuleId::EarlyBirds, RuleId::NightOwls, 150.0),
        (RuleId::OpenMeadow, RuleId::CozyBurrows, 1500.0),
        (RuleId::SharingCircle, RuleId::PersonalStashes, 4000.0),
        (RuleId::LoudPurring, RuleId::QuietContemplation, 15_000.0),
    ];
    for (a, b, cost) in pairs {
        assert_eq!(content::rule_def(a).cost_charm, cost, "{a:?}");
        assert_eq!(content::rule_def(b).cost_charm, cost, "{b:?}");
        assert_eq!(content::rule_def(a).blocks, b, "{a:?} blocks its sibling");
        assert_eq!(content::rule_def(b).blocks, a, "{b:?} blocks back");
    }
}

// @spec SIM-CUR-011
#[test]
fn adopt_rule_spends_charm_and_blocks_the_sibling_both_ways() {
    let mut s = GameState::new(310);
    give(&mut s, ResourceId::Charm, 149.0);
    let err = s.apply(Command::AdoptRule(RuleId::EarlyBirds)).unwrap_err();
    assert!(matches!(err, CommandError::InsufficientResources { .. }));
    assert_eq!(s.stock(ResourceId::Charm), 149.0);

    give(&mut s, ResourceId::Charm, 200.0);
    let events = s.apply(Command::AdoptRule(RuleId::EarlyBirds)).unwrap();
    assert!(events.contains(&Event::RuleAdopted {
        id: RuleId::EarlyBirds
    }));
    assert!(s.rules.contains(&RuleId::EarlyBirds));
    assert!(
        (s.stock(ResourceId::Charm) - 50.0).abs() < 1e-9,
        "150 charm spent"
    );

    // The sibling is permanently Locked for this meadow.
    give(&mut s, ResourceId::Charm, 500.0);
    assert_eq!(
        s.apply(Command::AdoptRule(RuleId::NightOwls)).unwrap_err(),
        CommandError::Locked
    );
    assert!(!s.rules.contains(&RuleId::NightOwls));

    // Blocking runs the other way too, from a fresh meadow.
    let mut owls = GameState::new(311);
    give(&mut owls, ResourceId::Charm, 150.0);
    owls.apply(Command::AdoptRule(RuleId::NightOwls)).unwrap();
    give(&mut owls, ResourceId::Charm, 150.0);
    assert_eq!(
        owls.apply(Command::AdoptRule(RuleId::EarlyBirds))
            .unwrap_err(),
        CommandError::Locked
    );

    // Unrelated pairs are unaffected by the choice.
    give(&mut s, ResourceId::Charm, 1500.0);
    s.apply(Command::AdoptRule(RuleId::CozyBurrows)).unwrap();
    assert!(s.rules.contains(&RuleId::CozyBurrows));

    // Re-adopting an owned rule is refused without mutation.
    give(&mut s, ResourceId::Charm, 500.0);
    let snapshot = s.clone();
    assert_eq!(
        s.apply(Command::AdoptRule(RuleId::EarlyBirds)).unwrap_err(),
        CommandError::AlreadyResearched
    );
    assert_eq!(s, snapshot);
}

// @spec SIM-CUR-011, SIM-CUR-012
#[test]
fn rule_effects_land_in_their_owning_leaves() {
    // Early Birds: catnip production +10% (a patch in summer: 0.125 × 1.1).
    let mut s = GameState::new(312);
    grant(&mut s, StructureId::CatnipPatch, 1);
    s.ticks = 1251; // summer, mid-day
    s.rules.insert(RuleId::EarlyBirds);
    run_ticks(&mut s, 1);
    assert!((s.stock(ResourceId::Catnip) - 0.125 * 1.1).abs() < 1e-9);

    // Sharing Circle: luxury drain −25%.
    let mut s = GameState::new(313);
    adopt(&mut s, 1);
    s.kittens[0].job = None;
    s.rules.insert(RuleId::SharingCircle);
    off_boundary(&mut s);
    give(&mut s, ResourceId::Catnip, 150.0);
    give(&mut s, ResourceId::Feathers, 2.0);
    run_ticks(&mut s, 1);
    assert!((s.stock(ResourceId::Feathers) - (2.0 - 0.001 * 0.75)).abs() < 1e-9);

    // Cozy Burrows: +5 happiness points.
    let mut s = GameState::new(314);
    adopt(&mut s, 1);
    let base = s.happiness();
    s.rules.insert(RuleId::CozyBurrows);
    assert!((s.happiness() - (base + 5.0)).abs() < 1e-9);

    // Open Meadow: arrival +10% inside the cap.
    let mut s = GameState::new(315);
    give(&mut s, ResourceId::Catnip, 50.0);
    s.rules.insert(RuleId::OpenMeadow);
    assert!((s.arrival_chance() - 0.25).abs() < 1e-9);
    give(&mut s, ResourceId::Catnip, 2000.0);
    assert!(
        (s.arrival_chance() - 0.60).abs() < 1e-9,
        "inside the cap, swallowed by it"
    );

    // Loud Purring: charm production +20% (a Story Circle: 0.005 × 1.2).
    let mut s = GameState::new(316);
    grant(&mut s, StructureId::StoryCircle, 1);
    s.ticks = 3;
    s.rules.insert(RuleId::LoudPurring);
    run_ticks(&mut s, 1);
    assert!((s.stock(ResourceId::Charm) - 0.005 * 1.2).abs() < 1e-9);
}

// @spec SIM-CUR-011
#[test]
fn personal_stashes_multiplies_caps_but_never_opens_zero_or_infinite_ones() {
    let mut s = GameState::new(317);
    grant(&mut s, StructureId::Pantry, 1);
    s.rules.insert(RuleId::PersonalStashes);
    // cap = (base + flat extensions) × (1 + cap-ratio effects).
    assert!((s.cap(ResourceId::Catnip) - (200.0 + 250.0) * 1.1).abs() < 1e-9);
    assert!((s.cap(ResourceId::Twigs) - 50.0 * 1.1).abs() < 1e-9);
    // A zero cap stays zero: the Cardboard-Box gate never opens by percentage.
    assert_eq!(s.cap(ResourceId::Curiosity), 0.0);
    assert!(
        !s.idea_unlocked(IdeaId::PantryOrganization),
        "still boxless, still Locked"
    );
    // Infinite caps stay infinite.
    assert!(s.cap(ResourceId::Feathers).is_infinite());
}

// @spec SIM-CUR-011
#[test]
fn adopted_rules_and_their_blocks_survive_save_load() {
    let mut s = GameState::new(318);
    give(&mut s, ResourceId::Charm, 150.0);
    s.apply(Command::AdoptRule(RuleId::EarlyBirds)).unwrap();
    let json = save_string(&s, 42_000);
    let mut loaded = load_string(&json, 42_000).unwrap().state;
    assert!(loaded.rules.contains(&RuleId::EarlyBirds));
    give(&mut loaded, ResourceId::Charm, 150.0);
    assert_eq!(
        loaded
            .apply(Command::AdoptRule(RuleId::NightOwls))
            .unwrap_err(),
        CommandError::Locked,
        "the block survives the save file"
    );
}

// @spec SIM-CUR-012, SIM-ENG-019
#[test]
fn refinement_and_rule_effects_sum_through_the_effect_key_system() {
    // Every PounceRatio source in one meadow: Bolas 1 + Pads 2 + Whisker
    // Guards 0.5 + Mittens 0.5 + the Bold Elder 0.5 = 4.5.
    let mut s = GameState::new(319);
    adopt(&mut s, 1);
    s.kittens[0].trait_id = KittenTrait::Bold;
    s.apply(Command::AppointElder(0)).unwrap();
    s.refinements.insert(RefinementId::BolasOfYarn);
    s.refinements.insert(RefinementId::PouncePads);
    s.refinements.insert(RefinementId::WhiskerGuards);
    s.refinements.insert(RefinementId::MoonsteelMittens);
    assert!((s.effect(Effect::PounceRatio) - 4.5).abs() < 1e-9);
    let a = s.effect(Effect::PounceRatio);
    let b = s.effect(Effect::PounceRatio);
    assert_eq!(a.to_bits(), b.to_bits(), "bit-identical resummation");

    // Sources mix per key: Herb Gardens (+6%) and Early Birds (+10%) pool on
    // catnip production, measured through a summer patch.
    let mut mixed = GameState::new(320);
    grant(&mut mixed, StructureId::CatnipPatch, 1);
    grant(&mut mixed, StructureId::HerbGarden, 2);
    mixed.rules.insert(RuleId::EarlyBirds);
    mixed.ticks = 1251; // summer, mid-day
    run_ticks(&mut mixed, 1);
    assert!((mixed.stock(ResourceId::Catnip) - 0.125 * 1.16).abs() < 1e-9);
}

// =====================================================================
// KIT — Persistence (SIM-KIT-027) and determinism (SIM-ENG-018)
// =====================================================================

// @spec SIM-KIT-027
#[test]
fn skills_exp_ranks_and_traits_round_trip_through_the_save() {
    let mut s = GameState::new(330);
    adopt(&mut s, 2);
    s.kittens[0].skills[JobId::Gardener as usize] = 123.456;
    s.kittens[0].exp = 987.5;
    s.kittens[0].rank = 3;
    s.kittens[0].trait_id = KittenTrait::Serene;
    s.kittens[1].trait_id = KittenTrait::Charming;
    s.kittens[1].tinkering = Some(CraftId::Wands);
    s.elder = Some(0);
    s.jamboree_until = 4321;
    s.refinements.insert(RefinementId::PebbleCombs);
    s.rules.insert(RuleId::EarlyBirds);

    let json = save_string(&s, 42_000);
    let loaded = load_string(&json, 42_000).unwrap().state;
    assert_eq!(loaded.kittens[0].skills[JobId::Gardener as usize], 123.456);
    assert_eq!(loaded.kittens[0].exp, 987.5);
    assert_eq!(loaded.kittens[0].rank, 3);
    assert_eq!(loaded.kittens[0].trait_id, KittenTrait::Serene);
    assert_eq!(loaded.kittens[1].trait_id, KittenTrait::Charming);
    assert_eq!(loaded.kittens[1].tinkering, Some(CraftId::Wands));
    assert_eq!(loaded.elder, Some(0));
    assert_eq!(loaded.jamboree_until, 4321);
    assert!(loaded.refinements.contains(&RefinementId::PebbleCombs));
    assert!(loaded.rules.contains(&RuleId::EarlyBirds));
    // And the round trip stays byte-stable (SAVE-010 extended to new fields).
    assert_eq!(save_string(&loaded, 42_000), json);
}

// @spec SIM-KIT-027, SAVE-013
#[test]
fn v1_save_kittens_get_society_defaults() {
    let fixture = include_str!("fixtures/save_v1.json");
    let loaded = load_string(fixture, 1_751_700_000).expect("v1 fixture still loads");
    let s = &loaded.state;
    assert_eq!(s.kittens.len(), 2);
    for k in &s.kittens {
        assert_eq!(k.skills, [0.0; 8], "{}: skills zero-fill", k.name);
        assert_eq!(k.exp, 0.0);
        assert_eq!(k.rank, 0);
        assert_eq!(
            k.trait_id,
            KittenTrait::None,
            "v1 kittens default to no trait"
        );
        assert_eq!(k.tinkering, None);
    }
    assert_eq!(s.elder, None);
    assert_eq!(s.jamboree_until, 0);
    assert!(s.refinements.is_empty());
    assert!(s.rules.is_empty());
}

// @spec SIM-KIT-027
#[test]
fn no_mechanic_confiscates_a_kittens_learning() {
    // A famine that drives a kitten to the neighbor's yard takes nothing it
    // has learned: skills, exp, rank, and trait ride along and come home.
    let mut s = GameState::new(331);
    adopt(&mut s, 2);
    s.kittens[1].skills[JobId::Gardener as usize] = 500.0;
    s.kittens[1].exp = 800.0;
    s.kittens[1].rank = 2;
    s.kittens[1].trait_id = KittenTrait::Serene;
    give(&mut s, ResourceId::Catnip, 0.0);
    run_days(&mut s, 4); // famine day 3 wanders the newest kitten
    assert_eq!(s.kittens[1].location, Location::NeighborsYard);
    assert_eq!(s.kittens[1].skills[JobId::Gardener as usize], 500.0);
    assert_eq!(s.kittens[1].exp, 800.0);
    assert_eq!(s.kittens[1].rank, 2);
    assert_eq!(s.kittens[1].trait_id, KittenTrait::Serene);
    give(&mut s, ResourceId::Catnip, 150.0);
    run_days(&mut s, 2);
    assert_eq!(
        s.kittens[1].location,
        Location::Home,
        "and comes home intact"
    );
    assert_eq!(s.kittens[1].exp, 800.0);
}

// @spec SIM-ENG-018, SIM-ENG-007
#[test]
fn determinism_holds_with_arrivals_traits_and_pounces() {
    // The name and trait draws share the fixed boundary schedule; a scripted
    // month with adoptions and a pounce must replay bit-identically.
    let script = |seed: u64| {
        let mut s = GameState::new(seed);
        grant(&mut s, StructureId::CozyCushion, 8);
        s.ideas.insert(IdeaId::PouncingPractice);
        give(&mut s, ResourceId::Catnip, 150.0);
        run_days(&mut s, 30);
        give(&mut s, ResourceId::Pep, 300.0);
        s.apply(Command::Pounce(3)).unwrap();
        run_days(&mut s, 5);
        s
    };
    let a = script(42);
    let b = script(42);
    assert!(!a.kittens.is_empty(), "a month of plenty adopts somebody");
    assert_eq!(
        save_string(&a, 0),
        save_string(&b, 0),
        "bit-identical replay"
    );
    let c = script(43);
    assert_ne!(
        save_string(&a, 0),
        save_string(&c, 0),
        "different seed, different world"
    );
}

