//! The curiosity design's tables, read from the design itself: every row of the
//! idea tree and of the refinements against the content tables, and each rule
//! that row implies. A number that drifts on either side fails here.
//!
//! @spec SIM-CUR-001, SIM-CUR-005, SIM-CUR-009, SIM-CUR-010, SIM-CUR-017

use crate::common;

use clowder_core::content::JobGate;
use clowder_core::*;
use common::*;
use std::collections::BTreeSet;

const DESIGN: &str = include_str!("../../../../docs/intent/sim/curiosity/curiosity-design.md");

fn idea_rows() -> Vec<Vec<String>> {
    table_rows(DESIGN, "## The Idea Tree", "## Refinements")
}

fn refinement_rows() -> Vec<Vec<String>> {
    table_rows(DESIGN, "## Refinements", "## House Rules")
}

/// An amount and its resource: `15 schemes`.
fn amount_of(text: &str) -> (ResourceId, f64) {
    let (figure, name) = text.trim().split_once(' ').expect("an amount and a name");
    (
        resource_named(name),
        figure
            .parse()
            .unwrap_or_else(|_| panic!("a figure: {text}")),
    )
}

fn sorted(mut list: Vec<(ResourceId, f64)>) -> Vec<(ResourceId, f64)> {
    list.sort_by_key(|(r, _)| r.ix());
    list
}

fn idea_named(name: &str) -> IdeaId {
    IdeaId::ALL
        .into_iter()
        .find(|i| content::idea_def(*i).name == name)
        .unwrap_or_else(|| panic!("no idea is named {name:?}"))
}

/// Every idea stands in its row: its place, its name, its curiosity price, its
/// gate craft, and its prerequisites.
///
/// @spec SIM-CUR-005
#[test]
fn every_idea_matches_its_row_of_the_tree() {
    let rows = idea_rows();
    assert_eq!(rows.len(), 59, "the tree's rows");
    assert_eq!(rows.len(), IdeaId::ALL.len(), "one idea per row");
    for (n, row) in rows.iter().enumerate() {
        assert_eq!(
            row[0].parse::<usize>().unwrap(),
            n,
            "the rows number from 0"
        );
        let def = content::idea_def(IdeaId::ALL[n]);
        assert_eq!(def.name, row[1], "idea {n}'s name");

        let mut cost = row[2].split(" + ");
        let curiosity: f64 = cost.next().unwrap().parse().unwrap_or_else(|_| {
            panic!("{}: a curiosity price, not {:?}", row[1], row[2]);
        });
        assert_eq!(def.cost, curiosity, "{}'s curiosity price", row[1]);
        assert_eq!(
            sorted(def.gate.to_vec()),
            // A gate of none is worded `+ 0 stories`, where the idea opens the
            // recipe that makes them.
            sorted(cost.map(amount_of).filter(|(_, n)| *n > 0.0).collect()),
            "{}'s gate craft",
            row[1]
        );

        let wanted: BTreeSet<IdeaId> = if row[3] == "—" {
            BTreeSet::new()
        } else {
            row[3].split(", ").map(idea_named).collect()
        };
        let held: BTreeSet<IdeaId> = def.prereqs.iter().copied().collect();
        assert_eq!(held, wanted, "{}'s prerequisites", row[1]);
    }
}

/// A row's closing cell, whose unlocks are named in it.
fn unlocks_of(i: IdeaId) -> String {
    idea_rows()[i as usize][4].clone()
}

/// Every meadow short of one idea, then holding it: the rule reads that idea.
fn opens_only_with(i: IdeaId, what: &str, unlocked: impl Fn(&GameState) -> bool) {
    let mut s = GameState::new(31);
    s.ideas = IdeaId::ALL.into_iter().filter(|x| *x != i).collect();
    assert!(!unlocked(&s), "{what} is open without {i:?}");
    s.ideas.insert(i);
    assert!(unlocked(&s), "{what} stays shut with {i:?}");
}

/// Each structure, job, and recipe an idea opens is named in that idea's row, and
/// a rule reads that idea for it.
///
/// @spec SIM-CUR-009
#[test]
fn every_structure_job_and_recipe_an_idea_opens_is_in_its_row_and_read_by_a_rule() {
    let mut named = 0;
    for st in StructureId::ALL {
        let def = content::structure_def(st);
        if let StructureGate::Idea(i) = def.gate {
            named += 1;
            assert!(
                unlocks_of(i).contains(def.name),
                "{}'s row does not name {}: {:?}",
                content::idea_def(i).name,
                def.name,
                unlocks_of(i)
            );
            opens_only_with(i, def.name, |s| s.structure_unlocked(st));
        }
    }
    for j in JobId::ALL {
        let def = content::job_def(j);
        if let JobGate::Idea(i) = def.gate {
            named += 1;
            assert!(
                unlocks_of(i).contains(def.name),
                "{}'s row does not name {}: {:?}",
                content::idea_def(i).name,
                def.name,
                unlocks_of(i)
            );
            opens_only_with(i, def.name, |s| s.job_unlocked(j));
        }
    }
    for c in CraftId::ALL {
        let def = content::craft_def(c);
        if let Some(i) = def.gate {
            named += 1;
            opens_only_with(i, def.name, |s| s.craft_unlocked(c));
        }
    }
    assert!(named > 40, "fixture: most things wait on an idea ({named})");
}

/// A name in a row's unlocks is opened by that row's idea, and no other: read the
/// other way, so a row cannot promise what another idea opens.
///
/// @spec SIM-CUR-009
#[test]
fn a_structure_or_job_a_row_names_as_its_unlock_waits_on_that_idea() {
    for i in IdeaId::ALL {
        let row = unlocks_of(i);
        for piece in row.split([',', ';']).map(str::trim) {
            for st in StructureId::ALL {
                let def = content::structure_def(st);
                if piece == def.name {
                    assert!(
                        matches!(
                            def.gate,
                            StructureGate::Idea(gate) | StructureGate::SiteAndIdea(_, gate)
                                if gate == i
                        ),
                        "{}'s row names {} as its unlock",
                        content::idea_def(i).name,
                        def.name
                    );
                }
            }
            for j in JobId::ALL {
                let def = content::job_def(j);
                if piece == format!("{} job", def.name) {
                    assert!(
                        matches!(def.gate, JobGate::Idea(gate) if gate == i),
                        "{}'s row names {} as its unlock",
                        content::idea_def(i).name,
                        def.name
                    );
                }
            }
        }
    }
}

/// A refinement's price as its row words it: `100c, 50 twigs + 250 pebbles`.
fn price_of(text: &str) -> (f64, Vec<(ResourceId, f64)>) {
    let (curiosity, materials) = text.split_once(", ").expect("curiosity and materials");
    (
        curiosity
            .strip_suffix('c')
            .and_then(|c| c.parse().ok())
            .unwrap_or_else(|| panic!("a curiosity price: {curiosity}")),
        sorted(materials.split(" + ").map(amount_of).collect()),
    )
}

/// A row's words for what a refinement does, the effect keys that say it, and the
/// structure they scale by.
type RowEffects = (&'static str, Vec<(Effect, f64)>, Option<StructureId>);

/// What each row of the design says a refinement does, and the effect keys that
/// say it: the row's own words, so neither can change without the other.
fn effects_by_row() -> Vec<RowEffects> {
    use Effect as E;
    use JobId as J;
    use ResourceId as R;
    use StructureId as S;
    let one = |e: Effect, v: f64| vec![(e, v)];
    vec![
        ("Gardener +50%", one(E::JobRatio(J::Gardener), 0.5), None),
        ("Gardener +30%", one(E::JobRatio(J::Gardener), 0.3), None),
        ("Gardener +50%", one(E::JobRatio(J::Gardener), 0.5), None),
        ("Gardener +50%", one(E::JobRatio(J::Gardener), 0.5), None),
        (
            "TwigCollector +70%",
            one(E::JobRatio(J::TwigCollector), 0.7),
            None,
        ),
        (
            "TwigCollector +50%",
            one(E::JobRatio(J::TwigCollector), 0.5),
            None,
        ),
        (
            "TwigCollector +50%",
            one(E::JobRatio(J::TwigCollector), 0.5),
            None,
        ),
        (
            "TwigCollector +50%",
            one(E::JobRatio(J::TwigCollector), 0.5),
            None,
        ),
        ("Rummager +50%", one(E::JobRatio(J::Rummager), 0.5), None),
        ("Rummager +50%", one(E::JobRatio(J::Rummager), 0.5), None),
        ("pounce ratio +1", one(E::PounceRatio, 1.0), None),
        ("pounce ratio +2", one(E::PounceRatio, 2.0), None),
        ("pounce ratio +0.5", one(E::PounceRatio, 0.5), None),
        ("pounce ratio +0.5", one(E::PounceRatio, 0.5), None),
        (
            "+100 twig cap, +150 pebble cap",
            vec![(E::MaxCap(R::Twigs), 100.0), (E::MaxCap(R::Pebbles), 150.0)],
            None,
        ),
        (
            "+100 twig cap, +150 pebble cap, +25 button cap",
            vec![
                (E::MaxCap(R::Twigs), 100.0),
                (E::MaxCap(R::Pebbles), 150.0),
                (E::MaxCap(R::Buttons), 25.0),
            ],
            None,
        ),
        (
            "+50 button cap, +30 ember cap, +10 moonstone cap",
            vec![
                (E::MaxCap(R::Buttons), 50.0),
                (E::MaxCap(R::Embers), 30.0),
                (E::MaxCap(R::Moonstone), 10.0),
            ],
            None,
        ),
        (
            "Pantries also hold +125 catnip each",
            one(E::MaxCap(R::Catnip), 125.0),
            Some(S::Pantry),
        ),
        (
            "+200 pebble cap, +50 button cap",
            vec![
                (E::MaxCap(R::Pebbles), 200.0),
                (E::MaxCap(R::Buttons), 50.0),
            ],
            None,
        ),
        (
            "Hearth embers +0.002/t",
            one(E::StructureOutputAdd(S::Hearth, R::Embers), 0.002),
            None,
        ),
        (
            "Hearth embers +0.003/t",
            one(E::StructureOutputAdd(S::Hearth, R::Embers), 0.003),
            None,
        ),
        (
            "Cogworks +0.005 stories/t each",
            one(E::StructureOutputAdd(S::Cogworks, R::Stories), 0.005),
            None,
        ),
        (
            "Stories/Anthologies/Schemes craft +25%",
            vec![
                (E::CraftRatioFor(CraftId::Stories), 0.25),
                (E::CraftRatioFor(CraftId::Anthologies), 0.25),
                (E::CraftRatioFor(CraftId::Schemes), 0.25),
            ],
            None,
        ),
        ("Observer +30%", one(E::JobRatio(J::Observer), 0.3), None),
        (
            "sky-event chance ×2",
            one(E::SkyEventChanceRatio, 1.0),
            None,
        ),
        (
            "auto-observe chance ×2",
            one(E::AutoObserveChanceRatio, 1.0),
            None,
        ),
        (
            "Toy Workshop mess 2 → −2",
            one(E::StructureMessDelta(S::ToyWorkshop), -4.0),
            None,
        ),
        ("skill decay removed", vec![], None),
        ("skill XP +50%", one(E::SkillXpRatio, 0.5), None),
        ("Tinkerer speed +100%", one(E::TinkererSpeed, 1.0), None),
        ("Tinkerer speed +100%", one(E::TinkererSpeed, 1.0), None),
        ("Tinkerer speed +200%", one(E::TinkererSpeed, 2.0), None),
        (
            "arrival chance +5% (inside the cap)",
            one(E::ArrivalChanceInCap, 0.05),
            None,
        ),
        (
            "+1 happiness point per Hearth",
            one(E::HappinessPoints, 1.0),
            Some(S::Hearth),
        ),
        (
            "Dew Collector +50%",
            one(E::StructureRatio(S::DewCollector), 0.5),
            None,
        ),
    ]
}

/// Every refinement stands in its row: its place, name, price, materials, gate,
/// and effects.
///
/// @spec SIM-CUR-010
#[test]
fn every_refinement_matches_its_row() {
    let rows = refinement_rows();
    let effects = effects_by_row();
    assert_eq!(rows.len(), 35, "the table's rows");
    assert_eq!(
        rows.len(),
        RefinementId::ALL.len(),
        "one refinement per row"
    );
    assert_eq!(rows.len(), effects.len(), "fixture: one line per row");
    for (n, (row, (words, keys, per))) in rows.iter().zip(effects).enumerate() {
        assert_eq!(
            row[0].parse::<usize>().unwrap(),
            n,
            "the rows number from 0"
        );
        let def = content::refinement_def(RefinementId::ALL[n]);
        assert_eq!(def.name, row[1], "refinement {n}'s name");
        let (curiosity, materials) = price_of(&row[2]);
        assert_eq!(def.cost, curiosity, "{}'s curiosity price", row[1]);
        assert_eq!(
            sorted(def.materials.to_vec()),
            materials,
            "{}'s materials",
            row[1]
        );
        assert_eq!(format!("{:?}", def.gate), row[3], "{}'s gate", row[1]);
        assert_eq!(row[4], words, "{}: the design's row changed", row[1]);
        assert_eq!(def.effects.to_vec(), keys, "{}'s effects", row[1]);
        assert_eq!(def.per_structure, per, "{}'s per-structure scaling", row[1]);
    }
}

/// Every refinement is Locked before its idea, spends all or nothing, is bought
/// once, and adds its effects; those that scale do so by the structure's copies.
///
/// @spec SIM-CUR-010
#[test]
fn every_refinement_is_gated_spent_whole_bought_once_and_contributes() {
    for r in RefinementId::ALL {
        let def = content::refinement_def(r);
        let mut s = GameState::new(32);
        grant(&mut s, StructureId::CardboardBox, 1);
        if let Some(st) = def.per_structure {
            grant(&mut s, st, 3);
        }
        give(&mut s, ResourceId::Curiosity, def.cost);
        for (m, amount) in def.materials {
            give(&mut s, *m, *amount);
        }
        let before = s.clone();
        assert_eq!(
            s.apply(Command::BuyRefinement(r)).unwrap_err(),
            CommandError::Locked,
            "{} before its idea",
            def.name
        );
        assert_eq!(s, before, "{} changed the meadow in refusing", def.name);

        s.ideas.insert(def.gate);
        let (short, _) = def.materials[0];
        let held = s.stock(short);
        s.resources[short.ix()] = held / 2.0;
        let shorted = s.clone();
        assert!(
            matches!(
                s.apply(Command::BuyRefinement(r)),
                Err(CommandError::InsufficientResources { .. })
            ),
            "{} short of {short:?}",
            def.name
        );
        assert_eq!(s, shorted, "{} spent something in refusing", def.name);
        s.resources[short.ix()] = held;

        let copies = def
            .per_structure
            .map_or(1.0, |st| s.structures[st.ix()] as f64);
        let effects_before: Vec<f64> = def.effects.iter().map(|(k, _)| s.effect(*k)).collect();
        s.apply(Command::BuyRefinement(r)).unwrap();
        assert_eq!(s.stock(ResourceId::Curiosity), 0.0, "{}'s price", def.name);
        for (m, _) in def.materials {
            assert_eq!(s.stock(*m), 0.0, "{}'s {m:?}", def.name);
        }
        for ((key, value), was) in def.effects.iter().zip(effects_before) {
            assert!(
                (s.effect(*key) - was - value * copies).abs() < 1e-9,
                "{} should add {} to {key:?}",
                def.name,
                value * copies
            );
        }
        assert_eq!(
            s.apply(Command::BuyRefinement(r)).unwrap_err(),
            CommandError::AlreadyResearched,
            "{} a second time",
            def.name
        );
    }
}

/// With no box, every idea is Locked, however many prerequisites are held and
/// however much is in the stores; a box opens each that has its prerequisites.
///
/// @spec SIM-CUR-001
#[test]
fn with_no_box_every_idea_is_locked() {
    for i in IdeaId::ALL {
        let mut s = GameState::new(33);
        s.ideas = IdeaId::ALL.into_iter().filter(|x| *x != i).collect();
        for r in ResourceId::ALL {
            s.resources[r.ix()] = 1.0e12;
        }
        assert!(!s.idea_unlocked(i), "{i:?} is open with no box");
        assert_ne!(
            s.availability(ContentId::Idea(i)),
            Availability::Available,
            "{i:?} is reported open with no box"
        );
        assert_eq!(
            s.clone().apply(Command::ResearchIdea(i)).unwrap_err(),
            CommandError::Locked,
            "{i:?} took research with no box"
        );
        grant(&mut s, StructureId::CardboardBox, 1);
        assert!(s.idea_unlocked(i), "{i:?} stays shut with a box");
    }
}

/// A share of a store's production scales the stack's flows and leaves a flat add
/// at its stated figure: what a rite adds is the same with the rule and without.
///
/// @spec SIM-CUR-017
#[test]
fn a_production_share_leaves_a_flat_add_at_its_figure() {
    let per_tick = |rule: bool, rite: bool| {
        let mut s = GameState::new(34);
        grant(&mut s, StructureId::SunbeamAltar, 3);
        assert!(s.cap(ResourceId::Charm) > 1.0, "fixture: charm has room");
        s.resources[ResourceId::Charm.ix()] = 0.0;
        if rule {
            s.rules.insert(RuleId::LoudPurring);
        }
        if rite {
            s.rites.insert(RiteId::StainedSunlight);
            s.rite_counts[RiteId::StainedSunlight.ix()] = 1;
        }
        s.tick();
        s.stock(ResourceId::Charm)
    };
    let add_alone = per_tick(false, true) - per_tick(false, false);
    let add_with_rule = per_tick(true, true) - per_tick(true, false);
    assert!(add_alone > 0.0, "fixture: the rite adds charm");
    assert!(
        per_tick(true, false) > per_tick(false, false),
        "fixture: the rule raises the stack's flow"
    );
    assert!(
        (add_with_rule - add_alone).abs() < 1e-12,
        "the rule scaled the flat add: {add_alone} alone, {add_with_rule} with it"
    );
}
