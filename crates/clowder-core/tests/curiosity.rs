mod common;

use clowder_core::*;
use common::*;

// @spec SIM-CUR-001
#[test]
fn no_box_no_ideas() {
    let mut s = GameState::new(1);
    give(&mut s, ResourceId::Curiosity, 100.0); // hand-edited plenty changes nothing
    assert!(!s.idea_unlocked(IdeaId::PantryOrganization));
    assert_eq!(
        s.apply(Command::ResearchIdea(IdeaId::PantryOrganization))
            .unwrap_err(),
        CommandError::Locked
    );
    grant(&mut s, StructureId::CardboardBox, 1);
    assert!(
        s.idea_unlocked(IdeaId::PantryOrganization),
        "the box opens the mind"
    );
}

// @spec SIM-CUR-002
#[test]
fn prerequisites_gate_the_ladder() {
    let mut s = GameState::new(2);
    grant(&mut s, StructureId::CardboardBox, 5);
    give(&mut s, ResourceId::Curiosity, 250.0);
    assert!(!s.idea_unlocked(IdeaId::GroomedToPurrfection));
    assert_eq!(
        s.apply(Command::ResearchIdea(IdeaId::GroomedToPurrfection))
            .unwrap_err(),
        CommandError::Locked
    );
    s.apply(Command::ResearchIdea(IdeaId::PantryOrganization))
        .unwrap();
    assert!(s.idea_unlocked(IdeaId::GroomedToPurrfection));
    s.apply(Command::ResearchIdea(IdeaId::GroomedToPurrfection))
        .unwrap();
    assert!(s.ideas.contains(&IdeaId::GroomedToPurrfection));
}

// @spec SIM-CUR-003
#[test]
fn research_spends_in_full_once() {
    let mut s = GameState::new(3);
    grant(&mut s, StructureId::CardboardBox, 1);
    give(&mut s, ResourceId::Curiosity, 20.0);
    let events = s
        .apply(Command::ResearchIdea(IdeaId::PantryOrganization))
        .unwrap();
    assert!(events.contains(&Event::IdeaResearched {
        id: IdeaId::PantryOrganization
    }));
    assert!((s.stock(ResourceId::Curiosity) - 5.0).abs() < 1e-9);
    assert_eq!(
        s.apply(Command::ResearchIdea(IdeaId::PantryOrganization))
            .unwrap_err(),
        CommandError::AlreadyResearched
    );
    // Unlocked but unaffordable reads as InsufficientResources.
    let err = s
        .apply(Command::ResearchIdea(IdeaId::ScratchingArts))
        .unwrap_err();
    assert!(matches!(err, CommandError::InsufficientResources { .. }));
}

// @spec SIM-CUR-004
#[test]
fn ideas_are_forever() {
    let mut s = GameState::new(4);
    grant(&mut s, StructureId::CardboardBox, 1);
    give(&mut s, ResourceId::Curiosity, 50.0);
    s.apply(Command::ResearchIdea(IdeaId::PantryOrganization))
        .unwrap();
    run_days(&mut s, 400); // seasons turn, famines come and go
    assert!(s.ideas.contains(&IdeaId::PantryOrganization));
    // And they survive the save file.
    let json = save_string(&s, 1_000_000);
    let loaded = load_string(&json, 1_000_000).unwrap();
    assert!(loaded.state.ideas.contains(&IdeaId::PantryOrganization));
}

// @spec SIM-CUR-005
#[test]
fn the_idea_tree_matches_the_design_table() {
    // The founding nine keep their tabled costs and prerequisites; the rest
    // of the 57-idea banded tree is pinned by tests/economy_depth.rs.
    let costs: Vec<f64> = IdeaId::ALL[..9]
        .iter()
        .map(|i| content::idea_def(*i).cost)
        .collect();
    assert_eq!(
        costs,
        vec![15.0, 25.0, 40.0, 60.0, 80.0, 120.0, 150.0, 200.0, 250.0]
    );

    use IdeaId::*;
    let prereqs: Vec<&[IdeaId]> = IdeaId::ALL[..9]
        .iter()
        .map(|i| content::idea_def(*i).prereqs)
        .collect();
    let expected: Vec<&[IdeaId]> = vec![
        &[],
        &[],
        &[PantryOrganization],
        &[ScratchingArts],
        &[PantryOrganization],
        &[GroomedToPurrfection],
        &[TwigArchitecture],
        &[BiscuitMaking],
        &[SeasonalAwareness],
    ];
    assert_eq!(prereqs, expected);
    for i in IdeaId::ALL {
        assert!(!content::idea_def(i).name.is_empty());
        assert!(
            !content::idea_def(i).blurb.is_empty(),
            "whimsy with clarity: every idea explains itself"
        );
    }
}

// @spec SIM-CUR-006
#[test]
fn research_touches_only_curiosity_and_the_idea_set() {
    let mut s = GameState::new(5);
    grant(&mut s, StructureId::CardboardBox, 1);
    give(&mut s, ResourceId::Curiosity, 30.0);
    adopt(&mut s, 1);
    let before = s.clone();
    s.apply(Command::ResearchIdea(IdeaId::PantryOrganization))
        .unwrap();
    assert_eq!(s.ticks, before.ticks);
    assert_eq!(s.rng, before.rng);
    assert_eq!(s.structures, before.structures);
    assert_eq!(s.kittens, before.kittens);
    assert_eq!(
        s.stock(ResourceId::Catnip),
        before.stock(ResourceId::Catnip)
    );
    assert_eq!(s.stock(ResourceId::Twigs), before.stock(ResourceId::Twigs));
    assert!(
        (before.stock(ResourceId::Curiosity) - s.stock(ResourceId::Curiosity) - 15.0).abs() < 1e-9
    );
    assert_eq!(s.ideas.len(), before.ideas.len() + 1);
}
