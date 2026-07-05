mod common;

use clowder_core::*;
use common::*;

// @spec SIM-ECO-001
#[test]
fn base_caps() {
    let s = GameState::new(1);
    assert_eq!(s.cap(ResourceId::Catnip), 200.0);
    assert_eq!(s.cap(ResourceId::Twigs), 50.0);
    assert_eq!(s.cap(ResourceId::Fluff), 25.0);
    assert_eq!(s.cap(ResourceId::Curiosity), 0.0);
}

// @spec SIM-ECO-002
#[test]
fn structures_extend_caps() {
    let mut s = GameState::new(2);
    grant(&mut s, StructureId::Pantry, 2);
    grant(&mut s, StructureId::TwigPile, 1);
    grant(&mut s, StructureId::CardboardBox, 3);
    grant(&mut s, StructureId::CatTree, 2);
    assert_eq!(s.cap(ResourceId::Catnip), 200.0 + 500.0);
    assert_eq!(s.cap(ResourceId::Twigs), 50.0 + 40.0);
    assert_eq!(s.cap(ResourceId::Curiosity), 150.0);
    assert_eq!(s.cap(ResourceId::Fluff), 25.0 + 20.0);
}

// @spec SIM-ECO-003
#[test]
fn clamp_trims_to_cap_after_consumption_eats_the_transient() {
    let mut s = GameState::new(3);
    adopt(&mut s, 1);
    s.kittens[0].job = None;
    grant(&mut s, StructureId::CatnipPatch, 10); // 1.25/tick × spring 1.5 = 1.875
    s.ticks = 1001; // spring, mid-day
    give(&mut s, ResourceId::Catnip, 199.99);
    run_ticks(&mut s, 1);
    // prod → 201.865, eat 0.085 → 201.78, clamp → 200. A clamp-before-consume
    // implementation would land at 199.915.
    assert_eq!(s.stock(ResourceId::Catnip), 200.0);
}

// @spec SIM-ECO-004
#[test]
fn gather_yields_one_or_two_with_the_idea() {
    let mut s = GameState::new(4);
    s.apply(Command::GatherCatnip).unwrap();
    assert_eq!(s.stock(ResourceId::Catnip), 1.0);
    s.ideas.insert(IdeaId::SelectiveHearing);
    s.apply(Command::GatherCatnip).unwrap();
    assert_eq!(s.stock(ResourceId::Catnip), 3.0);
}

// @spec SIM-ECO-005
#[test]
fn forage_trades_ten_catnip_for_a_twig() {
    let mut s = GameState::new(5);
    give(&mut s, ResourceId::Catnip, 15.0);
    s.apply(Command::Forage).unwrap();
    assert_eq!(s.stock(ResourceId::Catnip), 5.0);
    assert_eq!(s.stock(ResourceId::Twigs), 1.0);
    let err = s.apply(Command::Forage).unwrap_err();
    assert!(matches!(err, CommandError::InsufficientResources { .. }));
}

// @spec SIM-ECO-006
#[test]
fn manual_actions_reject_when_yield_is_capped() {
    let mut s = GameState::new(6);
    give(&mut s, ResourceId::Catnip, 200.0);
    assert_eq!(s.apply(Command::GatherCatnip).unwrap_err(), CommandError::StorageFull);
    assert_eq!(s.stock(ResourceId::Catnip), 200.0);

    give(&mut s, ResourceId::Catnip, 100.0);
    give(&mut s, ResourceId::Twigs, 50.0);
    assert_eq!(s.apply(Command::Forage).unwrap_err(), CommandError::StorageFull);
    assert_eq!(s.stock(ResourceId::Catnip), 100.0, "no catnip spent on a trimmed yield");
}

// @spec SIM-ECO-007
#[test]
fn build_costs_ramp_exponentially_with_ceil() {
    let mut s = GameState::new(7);
    give(&mut s, ResourceId::Catnip, 200.0);
    assert_eq!(s.next_cost(StructureId::CatnipPatch), vec![(ResourceId::Catnip, 10.0)]);
    let events = s.apply(Command::Build(StructureId::CatnipPatch)).unwrap();
    assert!(events.contains(&Event::StructureBuilt { id: StructureId::CatnipPatch, count: 1 }));
    assert_eq!(s.stock(ResourceId::Catnip), 190.0);
    assert_eq!(s.next_cost(StructureId::CatnipPatch), vec![(ResourceId::Catnip, 12.0)]); // ceil(11.2)
    s.apply(Command::Build(StructureId::CatnipPatch)).unwrap();
    assert_eq!(s.next_cost(StructureId::CatnipPatch), vec![(ResourceId::Catnip, 13.0)]); // ceil(12.544)
}

// @spec SIM-ECO-008
#[test]
fn catnip_patch_definition_and_seasonal_production() {
    let mut s = GameState::new(8);
    assert!(s.structure_unlocked(StructureId::CatnipPatch));
    grant(&mut s, StructureId::CatnipPatch, 4); // 0.5/tick before season
    s.ticks = 1001; // spring
    give(&mut s, ResourceId::Catnip, 0.0);
    run_ticks(&mut s, 1);
    assert!((s.stock(ResourceId::Catnip) - 0.5 * 1.5).abs() < 1e-9);
}

// @spec SIM-ECO-009
#[test]
fn cozy_cushion_houses_one() {
    let mut s = GameState::new(9);
    assert!(s.structure_unlocked(StructureId::CozyCushion));
    grant(&mut s, StructureId::CozyCushion, 3);
    assert_eq!(s.housing_capacity(), 3);
    let def = content::structure_def(StructureId::CozyCushion);
    assert_eq!(def.base_costs, &[(ResourceId::Catnip, 25.0), (ResourceId::Twigs, 5.0)]);
    assert_eq!(def.ratio, 1.15);
}

// @spec SIM-ECO-010
#[test]
fn cardboard_box_radiates_mystery() {
    let mut s = GameState::new(10);
    assert!(s.structure_unlocked(StructureId::CardboardBox));
    grant(&mut s, StructureId::CardboardBox, 2);
    s.ticks = 1;
    run_ticks(&mut s, 1);
    assert!((s.stock(ResourceId::Curiosity) - 0.01).abs() < 1e-12);
    assert_eq!(s.cap(ResourceId::Curiosity), 100.0);
}

// @spec SIM-ECO-011
#[test]
fn pantry_is_idea_gated_storage() {
    let mut s = GameState::new(11);
    assert!(!s.structure_unlocked(StructureId::Pantry));
    s.ideas.insert(IdeaId::PantryOrganization);
    assert!(s.structure_unlocked(StructureId::Pantry));
    grant(&mut s, StructureId::Pantry, 1);
    assert_eq!(s.cap(ResourceId::Catnip), 450.0);
}

// @spec SIM-ECO-012
#[test]
fn twig_pile_is_idea_gated_storage() {
    let mut s = GameState::new(12);
    assert!(!s.structure_unlocked(StructureId::TwigPile));
    s.ideas.insert(IdeaId::TwigArchitecture);
    assert!(s.structure_unlocked(StructureId::TwigPile));
    let def = content::structure_def(StructureId::TwigPile);
    assert_eq!(def.base_costs, &[(ResourceId::Catnip, 50.0), (ResourceId::Twigs, 10.0)]);
}

// @spec SIM-ECO-013
#[test]
fn scratching_post_is_idea_gated_happiness() {
    let mut s = GameState::new(13);
    assert!(!s.structure_unlocked(StructureId::ScratchingPost));
    s.ideas.insert(IdeaId::ScratchingArts);
    assert!(s.structure_unlocked(StructureId::ScratchingPost));
    adopt(&mut s, 1);
    let before = s.happiness();
    grant(&mut s, StructureId::ScratchingPost, 2);
    assert!((s.happiness() - before - 10.0).abs() < 1e-9);
}

// @spec SIM-ECO-014
#[test]
fn cat_tree_houses_three_and_stores_fluff() {
    let mut s = GameState::new(14);
    assert!(!s.structure_unlocked(StructureId::CatTree));
    s.ideas.insert(IdeaId::TwigArchitecture);
    assert!(s.structure_unlocked(StructureId::CatTree));
    grant(&mut s, StructureId::CatTree, 2);
    assert_eq!(s.housing_capacity(), 6);
    assert_eq!(s.cap(ResourceId::Fluff), 45.0);
}

// @spec SIM-ECO-015
#[test]
fn gardener_yield_is_season_independent() {
    let mut s = GameState::new(15);
    adopt(&mut s, 1);
    s.apply(Command::AssignJob(JobId::Gardener)).unwrap();
    s.ticks = 751; // winter, mid-day
    give(&mut s, ResourceId::Catnip, 10.0);
    run_ticks(&mut s, 1);
    // 0.15 × mood 1.0 − appetite 0.085 = +0.065, unaffected by winter.
    assert!((s.stock(ResourceId::Catnip) - 10.065).abs() < 1e-9);
}

// @spec SIM-ECO-016
#[test]
fn twig_collector_yield() {
    let mut s = GameState::new(16);
    adopt(&mut s, 1);
    s.apply(Command::AssignJob(JobId::TwigCollector)).unwrap();
    s.ticks += 1;
    s.zoomies_until = 0; // isolate the base yield from any adoption-day zoomies
    give(&mut s, ResourceId::Catnip, 10.0);
    run_ticks(&mut s, 1);
    assert!((s.stock(ResourceId::Twigs) - 0.03).abs() < 1e-9);
}

// @spec SIM-ECO-017
#[test]
fn groomer_is_idea_gated_and_yields_fluff() {
    let mut s = GameState::new(17);
    adopt(&mut s, 1);
    assert_eq!(s.apply(Command::AssignJob(JobId::Groomer)).unwrap_err(), CommandError::Locked);
    s.ideas.insert(IdeaId::GroomedToPurrfection);
    s.apply(Command::AssignJob(JobId::Groomer)).unwrap();
    s.ticks += 1;
    s.zoomies_until = 0; // isolate the base yield from any adoption-day zoomies
    give(&mut s, ResourceId::Catnip, 10.0);
    run_ticks(&mut s, 1);
    assert!((s.stock(ResourceId::Fluff) - 0.02).abs() < 1e-9);
}

// @spec SIM-ECO-018
#[test]
fn observer_requires_a_box_and_yields_curiosity() {
    let mut s = GameState::new(18);
    adopt(&mut s, 1);
    assert_eq!(s.apply(Command::AssignJob(JobId::Observer)).unwrap_err(), CommandError::Locked);
    grant(&mut s, StructureId::CardboardBox, 1);
    s.apply(Command::AssignJob(JobId::Observer)).unwrap();
    s.ticks += 1;
    s.zoomies_until = 0; // isolate the base yield from any adoption-day zoomies
    give(&mut s, ResourceId::Catnip, 10.0);
    run_ticks(&mut s, 1);
    assert!((s.stock(ResourceId::Curiosity) - (0.025 + 0.005)).abs() < 1e-9);
}

// @spec SIM-ECO-019
#[test]
fn seasons_modify_patch_production_only() {
    let season_output = |ticks: u64, eased: bool| {
        let mut s = GameState::new(19);
        grant(&mut s, StructureId::CatnipPatch, 8); // 1.0/tick base
        if eased {
            s.ideas.insert(IdeaId::SeasonalAwareness);
        }
        s.ticks = ticks;
        run_ticks(&mut s, 1);
        s.stock(ResourceId::Catnip)
    };
    assert!((season_output(1001, false) - 1.5).abs() < 1e-9); // spring
    assert!((season_output(1251, false) - 1.0).abs() < 1e-9); // summer
    assert!((season_output(1501, false) - 0.85).abs() < 1e-9); // autumn
    assert!((season_output(1751, false) - 0.25).abs() < 1e-9); // winter
    assert!((season_output(1751, true) - 0.40).abs() < 1e-9); // eased winter
}

// @spec SIM-ECO-020
#[test]
fn consumption_is_pooled_and_away_kittens_eat_nothing() {
    let mut s = GameState::new(20);
    adopt(&mut s, 3);
    for k in &mut s.kittens {
        k.job = None;
    }
    s.kittens[2].location = Location::NeighborsYard;
    s.kittens[2].away_seq = Some(1);
    s.ticks += 1;
    give(&mut s, ResourceId::Catnip, 10.0);
    run_ticks(&mut s, 1);
    // Two Home kittens eat 0.17; the away one eats nothing.
    assert!((s.stock(ResourceId::Catnip) - (10.0 - 0.17)).abs() < 1e-9);
}

// @spec SIM-ECO-021
#[test]
fn shortfall_zeroes_stock_and_exact_cover_is_thrift() {
    let mut s = GameState::new(21);
    adopt(&mut s, 1);
    s.kittens[0].job = None;
    s.ticks += 1;
    give(&mut s, ResourceId::Catnip, 0.05); // less than one meal
    run_ticks(&mut s, 1);
    assert_eq!(s.stock(ResourceId::Catnip), 0.0);
    assert!(s.grumpy, "shortfall makes the clowder grumpy");

    give(&mut s, ResourceId::Catnip, 0.085); // exactly one meal
    run_ticks(&mut s, 1);
    assert!(!s.grumpy, "a licked-clean bowl is thrift, not famine");
}

// @spec SIM-ECO-022
#[test]
fn affordability_tolerates_float_dust() {
    let mut s = GameState::new(22);
    give(&mut s, ResourceId::Catnip, 10.0 - 1e-10); // displays as 10
    s.apply(Command::Forage).unwrap();
    assert_eq!(s.stock(ResourceId::Twigs), 1.0);
    assert!(s.stock(ResourceId::Catnip) >= 0.0, "deduction floors at zero");
}

// @spec SIM-ECO-023
#[test]
fn structures_never_feel_moods_or_zoomies() {
    let mut base = GameState::new(23);
    grant(&mut base, StructureId::CatnipPatch, 4);
    base.ticks = 1251; // summer, ×1.0
    let mut grumpy = base.clone();
    grumpy.grumpy = true;
    let mut zoomy = base.clone();
    zoomy.zoomies_until = zoomy.ticks + 50;
    run_ticks(&mut base, 1);
    run_ticks(&mut grumpy, 1);
    run_ticks(&mut zoomy, 1);
    assert_eq!(base.stock(ResourceId::Catnip), grumpy.stock(ResourceId::Catnip));
    assert_eq!(base.stock(ResourceId::Catnip), zoomy.stock(ResourceId::Catnip));
}

// SIM-ECO-024's reachability property moved to tests/economy_depth.rs, which
// enforces the revised *banded* horizons (bands 0–2: 20 copies; band 3 and
// declared walls: 5; band 4 and monuments: first copy).
