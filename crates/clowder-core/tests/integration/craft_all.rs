//! Making every batch the stocks cover, in one application. @spec SIM-ECO-064

use crate::common;

use clowder_core::*;
use common::*;

/// How far short of a cost the core still calls it covered.
const EPSILON: f64 = 1e-9;

/// A meadow with every recipe unlocked, a craft ratio standing, and nothing in
/// the stores.
fn empty_workshop() -> GameState {
    let mut s = GameState::new(64);
    s.ideas = IdeaId::ALL.iter().copied().collect();
    for st in StructureId::ALL {
        grant(&mut s, st, 3);
    }
    for r in ResourceId::ALL {
        s.resources[r.ix()] = 0.0;
    }
    s
}

/// As `empty_workshop`, with the input at position `scarce` covering `batches`
/// and every other input covering plenty more.
fn stocked(c: CraftId, scarce: usize, batches: f64) -> GameState {
    let mut s = empty_workshop();
    for (i, (r, amount)) in content::craft_def(c).inputs.iter().enumerate() {
        let covers = if i == scarce { batches } else { batches + 40.0 };
        s.resources[r.ix()] = amount * covers;
    }
    s
}

/// Every recipe, with each of its inputs in turn the scarce one.
fn every_scarcity() -> Vec<(CraftId, usize)> {
    CraftId::ALL
        .into_iter()
        .flat_map(|c| (0..content::craft_def(c).inputs.len()).map(move |i| (c, i)))
        .collect()
}

/// The count rests on what each recipe is: some input, each named once at a
/// positive amount, none of them the recipe's own output, whose making would
/// otherwise cover further batches as it went.
#[test]
fn every_recipe_has_the_shape_the_count_rests_on() {
    for c in CraftId::ALL {
        let inputs = content::craft_def(c).inputs;
        assert!(!inputs.is_empty(), "{c:?} takes no input");
        for (i, (r, amount)) in inputs.iter().enumerate() {
            assert!(*amount > 0.0, "{c:?} takes none of {r:?}");
            assert_ne!(*r, content::craft_output(c), "{c:?} takes its own output");
            assert!(
                inputs[..i].iter().all(|(earlier, _)| earlier != r),
                "{c:?} names {r:?} twice"
            );
        }
    }
}

/// One application leaves the meadow as calling one batch at a time until the
/// core refuses would, whichever input is the scarce one and with a craft ratio
/// standing.
///
/// @spec SIM-ECO-064
#[test]
fn one_application_leaves_what_a_batch_at_a_time_would() {
    let mut ratio_seen = false;
    for (c, scarce) in every_scarcity() {
        let before = stocked(c, scarce, 37.5);
        let mut by_batches = before.clone();
        let mut made = 0;
        while by_batches.apply(Command::Craft(c)).is_ok() {
            made += 1;
        }
        assert_eq!(made, 37, "fixture: {c:?} covers 37 whole batches");
        ratio_seen |= by_batches.stock(content::craft_output(c)) > 37.0 + 1e-6;

        let mut at_once = before.clone();
        at_once.apply(Command::CraftAll(c)).unwrap();

        for r in ResourceId::ALL {
            let (got, want) = (at_once.stock(r), by_batches.stock(r));
            assert!(
                (got - want).abs() <= 1e-6 * want.abs().max(1.0),
                "{c:?} (input {scarce} scarce) left {got} of {r:?}, batch by batch left {want}"
            );
        }
        // Nothing but the stores differs.
        at_once.resources = by_batches.resources;
        assert_eq!(at_once, by_batches, "{c:?} changed more than the stores");
    }
    assert!(ratio_seen, "fixture: no recipe had a craft ratio standing");
}

/// However high the stocks stand, the one application makes every batch: no
/// count short of them is built in, and one batch more is refused afterwards.
///
/// @spec SIM-ECO-064
#[test]
fn a_stock_far_past_any_run_of_calls_is_made_in_one_application() {
    for (c, scarce) in every_scarcity() {
        for batches in [1.0e5, 1.0e12] {
            let before = stocked(c, scarce, batches);
            let mut one = before.clone();
            one.apply(Command::Craft(c)).unwrap();
            let output = content::craft_output(c);
            let batch = one.stock(output) - before.stock(output);

            let mut s = before.clone();
            s.apply(Command::CraftAll(c)).unwrap();

            let want = before.stock(output) + batches * batch;
            assert!(
                (s.stock(output) - want).abs() <= 1e-6 * want,
                "{c:?} made {} of a wanted {want} at {batches} batches",
                s.stock(output)
            );
            assert!(
                matches!(
                    s.apply(Command::Craft(c)),
                    Err(CommandError::InsufficientResources { .. })
                ),
                "{c:?} still covers a batch after making them all at {batches} batches"
            );
        }
    }
}

/// What no whole batch takes stays in the store: the fraction of the scarce
/// input, and all the surplus of the rest.
///
/// @spec SIM-ECO-064
#[test]
fn what_no_whole_batch_covers_stays_in_the_stores() {
    for (c, scarce) in every_scarcity() {
        let mut s = stocked(c, scarce, 3.5);
        s.apply(Command::CraftAll(c)).unwrap();
        for (i, (r, amount)) in content::craft_def(c).inputs.iter().enumerate() {
            let want = amount * if i == scarce { 0.5 } else { 40.5 };
            assert!(
                (s.stock(*r) - want).abs() < 1e-9 * want.max(1.0),
                "{c:?} left {} of {r:?}, wanted {want}",
                s.stock(*r)
            );
        }
    }
}

/// Stocks a hair short of a whole batch cover it, as they do for one `Craft`.
///
/// @spec SIM-ECO-064
#[test]
fn the_count_is_as_tolerant_as_one_batch_is() {
    for (c, scarce) in every_scarcity() {
        let (r, amount) = content::craft_def(c).inputs[scarce];
        for batches in [1.0, 3.0] {
            let mut within = stocked(c, scarce, batches);
            within.resources[r.ix()] = amount * batches - EPSILON / 2.0;
            let mut by_batches = within.clone();
            let mut made = 0.0;
            while by_batches.apply(Command::Craft(c)).is_ok() {
                made += 1.0;
            }
            assert_eq!(made, batches, "fixture: {c:?} batch by batch");

            within.apply(Command::CraftAll(c)).unwrap();
            assert_eq!(
                within.stock(r),
                0.0,
                "{c:?} did not take the batch a hair of stock short"
            );
            let output = content::craft_output(c);
            assert!((within.stock(output) - by_batches.stock(output)).abs() < 1e-6);
        }

        // Further short than the tolerance, the batch is not covered.
        let mut short = stocked(c, scarce, 1.0);
        short.resources[r.ix()] = amount - 2.0 * EPSILON;
        assert!(short.apply(Command::CraftAll(c)).is_err());
    }
}

/// Stocks that cover no batch are refused with exactly what one `Craft` says,
/// shortfall included, and nothing is spent.
///
/// @spec SIM-ECO-064
#[test]
fn stocks_covering_no_batch_are_refused_as_one_batch_is() {
    for (c, scarce) in every_scarcity() {
        let before = stocked(c, scarce, 0.5);
        let single = before.clone().apply(Command::Craft(c)).unwrap_err();
        assert!(
            matches!(single, CommandError::InsufficientResources { .. }),
            "fixture: {c:?} refused for another reason: {single:?}"
        );

        let mut s = before.clone();
        assert_eq!(s.apply(Command::CraftAll(c)).unwrap_err(), single);
        assert_eq!(s, before, "{c:?} changed the meadow in refusing");
    }
}

/// A recipe not yet unlocked is refused as locked, however full the stores.
///
/// @spec SIM-ECO-064
#[test]
fn a_locked_recipe_is_refused_as_one_batch_is() {
    let mut seen = 0;
    for c in CraftId::ALL {
        let Some(gate) = content::craft_def(c).gate else {
            continue;
        };
        seen += 1;
        let mut before = stocked(c, 0, 20.0);
        before.ideas.remove(&gate);
        let mut s = before.clone();
        assert_eq!(
            s.apply(Command::CraftAll(c)).unwrap_err(),
            CommandError::Locked
        );
        assert_eq!(s, before, "{c:?} changed the meadow in refusing");
    }
    assert!(seen > 0, "fixture: no recipe is gated");
}
