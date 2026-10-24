//! Tests-first for the gating queries — the half of the derived-query surface
//! that answers "why not, how much, and how many":
//!
//!   * SIM-ENG-031 — the reason a thing is not actionable, as typed data,
//!     decided by the same gate the command layer enforces.
//!   * SIM-ENG-036 — that reason resolves by the command layer's blocker
//!     precedence (SIM-ENG-006), so the reason shown for an item and the error
//!     returned by acting on it never name different causes.
//!   * SIM-ENG-037 — an unrevealed item's reason withholds its costs and
//!     prerequisites: it says only that it is not yet available.
//!   * SIM-ENG-038 — the total cost of taking a repeatable action n times from
//!     current state, escalation included.
//!   * SIM-ENG-039 — how far current stock covers the next cost, as a fraction.
//!   * SIM-ENG-033 — the closed-form affordable count, so a caller can name it
//!     in advance instead of attempting until rejected.
//!
//! These fail until `gating.rs` lands; the suite compiles against the
//! signatures already scaffolded there:
//!   `lock_reason(ContentId) -> Option<LockReason>`
//!   `max_affordable(ContentId) -> Option<u32>`
//!   `cost_of_next(ContentId, u32) -> Vec<(ResourceId, f64)>`
//!   `affordability(ContentId) -> f64`
//! with `LockReason::{NotYetReached, Prerequisite, AlreadyYours, NoOneFree,
//! NoRoom, ShortBy { missing }}` in precedence order.
//!
//! ---- What this suite deliberately does NOT decide ----
//!
//! Two readings are left open where the specs admit more than one, and the
//! assertions are written to hold under either (see the returned open
//! questions):
//!
//!   1. For a content kind with no reveal ladder (Job, Rule, Rite, Arch,
//!      Refinement, Craft, Tradition, DreamLoom), a closed gate could answer
//!      `NotYetReached` (SIM-ENG-032 calls those kinds "absent or present") or
//!      `Prerequisite` (whose doc comment says "revealed, but a prerequisite is
//!      unmet"). `names_the_same_cause` accepts either against `Locked`.
//!   2. `affordability` over a multi-component price could be the
//!      worst-supplied component (the SIM-ENG-022 throttle precedent) or a
//!      per-component-clamped mean. Every assertion here holds under both:
//!      uniform fractions, the empty case, and the "1 exactly when the gate
//!      stops rejecting" coupling the spec's rationale states.

mod common;

use clowder_core::*;
use common::*;

const TOL: f64 = 1e-9;

// ---- fixtures & helpers ----

/// A plain Home kitten (direct construction, as the other suites do).
fn kitten(name: &str) -> Kitten {
    Kitten {
        name: name.to_string(),
        job: None,
        location: Location::Home,
        arrived_day: 0,
        job_seq: None,
        away_seq: None,
        skills: [0.0; 8],
        exp: 0.0,
        rank: 0,
        trait_id: KittenTrait::None,
        tinkering: None,
        profile: None,
    }
}

/// A meadow whose two always-gated starter structures have crossed the reveal
/// fraction. Reveal is the precondition for a reason naming a price at all
/// (SIM-ENG-037), so every lock-reason test that expects a shortfall starts
/// here.
fn revealed_meadow(seed: u64) -> GameState {
    let mut s = GameState::new(seed);
    give(&mut s, ResourceId::Catnip, 8.0);
    give(&mut s, ResourceId::Twigs, 2.0);
    let events = run_ticks(&mut s, 1);
    for id in [
        ContentId::Structure(StructureId::CatnipPatch),
        ContentId::Structure(StructureId::CozyCushion),
    ] {
        assert!(
            events.contains(&Event::Revealed { id }),
            "fixture: {id:?} should reveal once 30% of its price is in store"
        );
    }
    s
}

/// The amount of one resource inside a cost list (0 when unnamed).
fn amount_of(costs: &[(ResourceId, f64)], r: ResourceId) -> f64 {
    costs
        .iter()
        .filter(|(cr, _)| *cr == r)
        .map(|(_, a)| *a)
        .sum()
}

/// The escalating price of `n` copies, observed one build at a time rather
/// than assumed — the very derivation SIM-ENG-038 exists to spare a caller.
fn walked_total(s: &GameState, id: StructureId, n: u32) -> Vec<(ResourceId, f64)> {
    let mut probe = s.clone();
    let mut total: Vec<(ResourceId, f64)> = Vec::new();
    for _ in 0..n {
        for (r, amount) in probe.next_cost(id) {
            match total.iter_mut().find(|(tr, _)| *tr == r) {
                Some((_, t)) => *t += amount,
                None => total.push((r, amount)),
            }
        }
        probe.structures[id.ix()] += 1;
    }
    total
}

/// How many copies a caller would get by attempting until rejected — the
/// discovery loop SIM-ENG-033 exists to replace.
fn builds_until_rejected(s: &GameState, id: StructureId) -> u32 {
    let mut probe = s.clone();
    let mut n = 0;
    while probe.apply(Command::Build(id)).is_ok() {
        n += 1;
        assert!(n < 10_000, "runaway build loop");
    }
    n
}

/// Compare two cost lists as totals: each resource named at most once, order
/// immaterial, zero-valued entries equivalent to absent ones.
fn assert_costs_eq(actual: &[(ResourceId, f64)], expected: &[(ResourceId, f64)], what: &str) {
    let mut seen: Vec<ResourceId> = Vec::new();
    for (r, _) in actual {
        assert!(
            !seen.contains(r),
            "{what}: {r:?} named twice — a total, not a per-step log"
        );
        seen.push(*r);
    }
    let mut a: Vec<(ResourceId, f64)> = actual
        .iter()
        .copied()
        .filter(|(_, v)| v.abs() > TOL)
        .collect();
    let mut e: Vec<(ResourceId, f64)> = expected
        .iter()
        .copied()
        .filter(|(_, v)| v.abs() > TOL)
        .collect();
    a.sort_by_key(|(r, _)| r.ix());
    e.sort_by_key(|(r, _)| r.ix());
    assert_eq!(a.len(), e.len(), "{what}: expected {e:?}, got {a:?}");
    for ((ra, va), (re, ve)) in a.iter().zip(e.iter()) {
        assert_eq!(ra, re, "{what}: expected {e:?}, got {a:?}");
        assert!((va - ve).abs() < TOL, "{what}: expected {e:?}, got {a:?}");
    }
}

/// The command layer's blocker precedence (SIM-ENG-006) mapped onto lock
/// reasons. `Locked` accepts either `NotYetReached` or `Prerequisite`: both
/// name the "cannot reach this yet" cause, and which one a kind without a
/// reveal ladder uses is left open (see the suite header).
fn names_the_same_cause(reason: &LockReason, err: &CommandError) -> bool {
    matches!(
        (reason, err),
        (
            LockReason::NotYetReached | LockReason::Prerequisite,
            CommandError::Locked
        ) | (LockReason::AlreadyYours, CommandError::AlreadyResearched)
            | (LockReason::NoOneFree, CommandError::NoIdleKittens)
            | (LockReason::NoRoom, CommandError::NoSeats)
            | (
                LockReason::ShortBy { .. },
                CommandError::InsufficientResources { .. }
            )
    )
}

/// Ask the query and act on the item, and insist the two answers agree: the
/// same gate, the same precedence, the same cause. Returns the reason so a
/// caller can pin the exact variant.
fn reason_and_error(s: &GameState, id: ContentId, cmd: Command) -> Option<LockReason> {
    let reason = s.lock_reason(id);
    let outcome = s.clone().apply(cmd);
    match (&reason, &outcome) {
        (None, Ok(_)) => {}
        (Some(r), Err(e)) => assert!(
            names_the_same_cause(r, e),
            "lock_reason({id:?}) = {r:?} but acting on it returns {e:?} — the reason shown \
             for an item and the error returned by acting on it name different causes"
        ),
        (None, Err(e)) => {
            panic!("lock_reason({id:?}) = None (actionable) but acting on it returns {e:?}")
        }
        (Some(r), Ok(_)) => panic!("lock_reason({id:?}) = {r:?} but acting on it succeeds"),
    }
    reason
}

// ---- SIM-ENG-031: the reason, as typed data, from the command layer's gate ----

// @spec SIM-ENG-031
#[test]
fn lock_reason_is_absent_exactly_when_the_command_would_be_accepted() {
    let mut s = revealed_meadow(11);
    let patch = StructureId::CatnipPatch;
    // Across the whole approach to a price — empty, part-way, a whisker short,
    // exactly enough, plenty — the query and the gate never disagree about
    // whether the thing can be done.
    for stock in [0.0, 3.0, 9.5, 10.0, 25.0] {
        give(&mut s, ResourceId::Catnip, stock);
        let reason = reason_and_error(&s, ContentId::Structure(patch), Command::Build(patch));
        assert_eq!(
            reason.is_none(),
            stock >= 10.0,
            "at {stock} catnip a 10-catnip patch should be actionable iff the stock covers it"
        );
    }
}

// @spec SIM-ENG-031
#[test]
fn a_short_lock_reason_carries_the_shortfall_the_command_reports() {
    let mut s = revealed_meadow(12);
    give(&mut s, ResourceId::Catnip, 4.0);
    let id = ContentId::Structure(StructureId::CatnipPatch);

    let reason = s
        .lock_reason(id)
        .expect("4 catnip does not buy a 10-catnip patch");
    let LockReason::ShortBy { missing } = reason else {
        panic!("a revealed, merely-unaffordable item is short by an amount, got {reason:?}")
    };
    // Data, not prose, and not a re-derivation: the same shortfall the command
    // layer hands back, because it comes from the same gate.
    let outcome = s.clone().apply(Command::Build(StructureId::CatnipPatch));
    let Err(CommandError::InsufficientResources {
        missing: from_command,
    }) = outcome
    else {
        panic!("fixture: the command should report a shortfall, got {outcome:?}")
    };
    assert_costs_eq(
        &missing,
        &from_command,
        "the query's shortfall vs the command's",
    );
    assert_costs_eq(
        &missing,
        &[(ResourceId::Catnip, 6.0)],
        "the shortfall itself",
    );
}

// ---- SIM-ENG-036: the command layer's precedence, item by item ----

// @spec SIM-ENG-036
#[test]
fn a_closed_gate_outranks_an_empty_roster_as_the_command_layer_does() {
    let mut s = GameState::new(21);
    let job = JobId::Groomer;
    let id = ContentId::Job(job);

    // Two blockers at once: the job's idea is unresearched *and* no kitten is
    // free. The command layer reports Locked, never NoIdleKittens.
    let reason = reason_and_error(&s, id, Command::AssignJob(job))
        .expect("an ungated-for job in an empty meadow is not actionable");
    assert!(
        matches!(reason, LockReason::NotYetReached | LockReason::Prerequisite),
        "the gate is the more fundamental blocker, got {reason:?}"
    );

    // Lift the gate and the next blocker in the order surfaces — the same one
    // the command layer would then name.
    s.ideas.insert(IdeaId::GroomedToPurrfection);
    let reason = reason_and_error(&s, id, Command::AssignJob(job))
        .expect("an unlocked job with nobody free is still not actionable");
    assert_eq!(reason, LockReason::NoOneFree);

    // Lift that one and nothing is left to report.
    s.kittens.push(kitten("Biscuit"));
    assert_eq!(reason_and_error(&s, id, Command::AssignJob(job)), None);
}

// @spec SIM-ENG-036
#[test]
fn what_is_already_yours_outranks_what_it_would_cost() {
    let mut s = GameState::new(22);
    // Adopting one house rule blocks its sibling for the meadow; neither is
    // affordable at zero charm, so both rows carry two blockers.
    s.rules.insert(RuleId::EarlyBirds);
    give(&mut s, ResourceId::Charm, 0.0);

    let blocked = reason_and_error(
        &s,
        ContentId::Rule(RuleId::NightOwls),
        Command::AdoptRule(RuleId::NightOwls),
    )
    .expect("a sibling-blocked rule is not actionable");
    assert!(
        matches!(
            blocked,
            LockReason::NotYetReached | LockReason::Prerequisite
        ),
        "the block outranks the empty purse, got {blocked:?}"
    );

    let owned = reason_and_error(
        &s,
        ContentId::Rule(RuleId::EarlyBirds),
        Command::AdoptRule(RuleId::EarlyBirds),
    )
    .expect("an adopted rule cannot be adopted again");
    assert_eq!(
        owned,
        LockReason::AlreadyYours,
        "already yours outranks the price"
    );

    // Wealth changes neither answer: the earlier blocker still wins.
    give(&mut s, ResourceId::Charm, 10_000.0);
    let blocked = reason_and_error(
        &s,
        ContentId::Rule(RuleId::NightOwls),
        Command::AdoptRule(RuleId::NightOwls),
    )
    .expect("a sibling-blocked rule stays blocked however rich the meadow");
    assert!(matches!(
        blocked,
        LockReason::NotYetReached | LockReason::Prerequisite
    ));
    assert_eq!(
        reason_and_error(
            &s,
            ContentId::Rule(RuleId::EarlyBirds),
            Command::AdoptRule(RuleId::EarlyBirds)
        ),
        Some(LockReason::AlreadyYours)
    );
}

// @spec SIM-ENG-036
#[test]
fn a_finished_idea_reports_as_done_rather_than_as_unaffordable() {
    let mut s = GameState::new(23);
    grant(&mut s, StructureId::CardboardBox, 1);
    run_ticks(&mut s, 1); // the founding ideas reveal once curiosity has a home
    let idea = IdeaId::PantryOrganization;
    s.ideas.insert(idea);
    give(&mut s, ResourceId::Curiosity, 0.0); // and broke, on top of done

    let reason = reason_and_error(&s, ContentId::Idea(idea), Command::ResearchIdea(idea))
        .expect("a researched idea is not actionable again");
    assert_eq!(reason, LockReason::AlreadyYours);
}

// ---- SIM-ENG-037: an unrevealed item names nothing ----

// @spec SIM-ENG-037
#[test]
fn an_unrevealed_items_reason_says_only_that_it_is_not_yet_reached() {
    let mut s = GameState::new(31);
    give(&mut s, ResourceId::Catnip, 1.0);

    // Hidden behind a price it has not approached: the price stays unstated.
    let patch = ContentId::Structure(StructureId::CatnipPatch);
    assert_eq!(s.availability(patch), Availability::Hidden, "fixture");
    assert_eq!(
        s.lock_reason(patch),
        Some(LockReason::NotYetReached),
        "an unrevealed item's reason names neither its cost nor its shortfall"
    );

    // Hidden behind a gate: the prerequisite stays unstated too.
    let pantry = ContentId::Structure(StructureId::Pantry);
    assert_eq!(s.availability(pantry), Availability::Hidden, "fixture");
    assert_eq!(s.lock_reason(pantry), Some(LockReason::NotYetReached));

    // And an unrevealed idea: its curiosity price is not the player's yet.
    let idea = ContentId::Idea(IdeaId::PantryOrganization);
    assert_eq!(s.availability(idea), Availability::Hidden, "fixture");
    assert_eq!(s.lock_reason(idea), Some(LockReason::NotYetReached));
}

// @spec SIM-ENG-037
#[test]
fn revealing_an_item_is_what_permits_naming_its_price() {
    let mut s = GameState::new(32);
    let patch = ContentId::Structure(StructureId::CatnipPatch);
    give(&mut s, ResourceId::Catnip, 3.0); // exactly the reveal fraction

    // The stock has reached the fraction, but the reveal scan has not run:
    // still unrevealed, so still nothing said about the price.
    assert_eq!(s.availability(patch), Availability::Hidden, "fixture");
    assert_eq!(s.lock_reason(patch), Some(LockReason::NotYetReached));

    run_ticks(&mut s, 1);

    // Same stock, same shortfall — only the reveal changed, and now the reason
    // may name what is missing.
    assert_eq!(s.availability(patch), Availability::Teased, "fixture");
    let reason = s
        .lock_reason(patch)
        .expect("3 catnip does not buy a 10-catnip patch");
    let LockReason::ShortBy { missing } = reason else {
        panic!("a revealed item may name its price, got {reason:?}")
    };
    assert_costs_eq(
        &missing,
        &[(ResourceId::Catnip, 7.0)],
        "the revealed shortfall",
    );
}

// ---- SIM-ENG-038: the total cost of n steps, escalation included ----

// @spec SIM-ENG-038
#[test]
fn cost_of_next_totals_the_escalating_price_over_n_steps() {
    let s = revealed_meadow(41);
    let patch = StructureId::CatnipPatch;
    let id = ContentId::Structure(patch);

    // One step is the price on the shelf.
    assert_costs_eq(&s.cost_of_next(id, 1), &s.next_cost(patch), "one step");

    // n steps are the n successive prices — not n copies of the first.
    for n in [2u32, 3, 5] {
        assert_costs_eq(
            &s.cost_of_next(id, n),
            &walked_total(&s, patch, n),
            &format!("{n} steps"),
        );
    }
    let one = amount_of(&s.next_cost(patch), ResourceId::Catnip);
    let three = amount_of(&s.cost_of_next(id, 3), ResourceId::Catnip);
    assert!(
        three > 3.0 * one + TOL,
        "the total must carry the escalation ({three} should exceed 3 × {one})"
    );

    // Multi-component prices total per component.
    let cushion = StructureId::CozyCushion;
    let total = s.cost_of_next(ContentId::Structure(cushion), 3);
    assert_costs_eq(&total, &walked_total(&s, cushion, 3), "three cushions");
    assert_eq!(
        total.iter().filter(|(_, a)| a.abs() > TOL).count(),
        2,
        "both components named"
    );

    // Taking the action no times costs nothing.
    assert!(
        s.cost_of_next(id, 0).iter().all(|(_, a)| a.abs() < TOL),
        "zero steps cost nothing"
    );
}

// @spec SIM-ENG-038
#[test]
fn cost_of_next_is_exactly_what_n_actions_consume() {
    let s = revealed_meadow(42);
    let patch = StructureId::CatnipPatch;
    let total = s.cost_of_next(ContentId::Structure(patch), 4);
    assert!(
        !total.is_empty(),
        "a four-patch preview names what it would spend"
    );

    // Stock exactly the preview, then spend it: four builds land, the purse
    // empties, and a fifth was never in the preview.
    let mut play = s.clone();
    for (r, amount) in &total {
        give(&mut play, *r, *amount);
    }
    for i in 1..=4 {
        play.apply(Command::Build(patch)).unwrap_or_else(|e| {
            panic!("build {i} of 4 should be covered by the preview, got {e:?}")
        });
    }
    for (r, _) in &total {
        assert!(
            play.stock(*r) < TOL,
            "the preview should be spent to the whisker, {r:?} has {} left",
            play.stock(*r)
        );
    }
    assert!(
        play.apply(Command::Build(patch)).is_err(),
        "a fifth build was never previewed"
    );
}

// @spec SIM-ENG-038
#[test]
fn cost_of_next_prices_from_current_state_not_from_an_empty_meadow() {
    let mut s = revealed_meadow(43);
    let patch = StructureId::CatnipPatch;
    let id = ContentId::Structure(patch);

    let from_scratch = amount_of(&s.cost_of_next(id, 2), ResourceId::Catnip);
    s.structures[patch.ix()] += 2;
    let later = s.cost_of_next(id, 2);
    assert_costs_eq(
        &later,
        &walked_total(&s, patch, 2),
        "two more once two are owned",
    );
    assert!(
        amount_of(&later, ResourceId::Catnip) > from_scratch + TOL,
        "the next two cost more than the first two did"
    );
}

// ---- SIM-ENG-039: how far the stock covers the next cost ----

// @spec SIM-ENG-039
#[test]
fn affordability_runs_from_empty_to_covered() {
    let mut s = revealed_meadow(51);
    let cushion = StructureId::CozyCushion;
    let id = ContentId::Structure(cushion);
    let cost = s.next_cost(cushion);

    // Nothing in store: none of the cost is covered.
    for (r, _) in &cost {
        give(&mut s, *r, 0.0);
    }
    assert!(
        s.affordability(id).abs() < TOL,
        "empty stores cover nothing, got {}",
        s.affordability(id)
    );

    // Half of every component: half covered.
    for (r, amount) in &cost {
        give(&mut s, *r, amount * 0.5);
    }
    assert!(
        (s.affordability(id) - 0.5).abs() < TOL,
        "half of every component is half the cost, got {}",
        s.affordability(id)
    );

    // Every component covered: the fraction is 1, and the gate agrees.
    for (r, amount) in &cost {
        give(&mut s, *r, *amount);
    }
    assert!(
        (s.affordability(id) - 1.0).abs() < TOL,
        "got {}",
        s.affordability(id)
    );
    assert!(
        s.clone().apply(Command::Build(cushion)).is_ok(),
        "fixture: 1.0 means affordable now"
    );

    // A pile beyond the price does not push the fraction past 1.
    for (r, amount) in &cost {
        give(&mut s, *r, amount * 10.0);
    }
    assert!(
        (s.affordability(id) - 1.0).abs() < TOL,
        "got {}",
        s.affordability(id)
    );
}

// @spec SIM-ENG-039
#[test]
fn affordability_reaches_one_only_when_every_component_is_covered() {
    let mut s = revealed_meadow(52);
    let cushion = StructureId::CozyCushion;
    let id = ContentId::Structure(cushion);
    let cost = s.next_cost(cushion);
    assert!(cost.len() > 1, "fixture: a two-component price");

    // A heap of one component cannot stand in for a shortfall in the other —
    // the fraction is drawn by the rules that gate the action, and the gate
    // rejects.
    give(&mut s, cost[0].0, cost[0].1 * 8.0);
    give(&mut s, cost[1].0, cost[1].1 * 0.5);
    let f = s.affordability(id);
    assert!(
        f > 0.0 && f < 1.0 - TOL,
        "partly affordable is neither 0 nor 1, got {f}"
    );
    assert!(
        matches!(s.lock_reason(id), Some(LockReason::ShortBy { .. })),
        "the same state the fraction calls short, the gate calls short"
    );

    // Cover the short component and both answers close together.
    give(&mut s, cost[1].0, cost[1].1);
    assert!(
        (s.affordability(id) - 1.0).abs() < TOL,
        "got {}",
        s.affordability(id)
    );
    assert_eq!(s.lock_reason(id), None);
}

// ---- SIM-ENG-033: the count, named in advance ----

// @spec SIM-ENG-033
#[test]
fn max_affordable_names_the_count_attempting_would_discover() {
    let mut s = revealed_meadow(61);
    let patch = StructureId::CatnipPatch;
    let id = ContentId::Structure(patch);

    for stock in [0.0, 9.0, 10.0, 22.0, 100.0, 200.0] {
        give(&mut s, ResourceId::Catnip, stock);
        let by_attempting = builds_until_rejected(&s, patch);
        assert_eq!(
            s.max_affordable(id),
            Some(by_attempting),
            "at {stock} catnip the count should be named, not discovered"
        );
    }
}

// @spec SIM-ENG-033
#[test]
fn max_affordable_sits_exactly_on_the_boundary_cost_of_next_draws() {
    let mut s = revealed_meadow(62);
    let patch = StructureId::CatnipPatch;
    let id = ContentId::Structure(patch);
    give(&mut s, ResourceId::Catnip, 137.0);

    let n = s
        .max_affordable(id)
        .expect("an escalating structure price has a closed form");
    assert!(n > 0, "137 catnip buys at least one 10-catnip patch");
    for (r, amount) in s.cost_of_next(id, n) {
        assert!(
            s.stock(r) + TOL >= amount,
            "{n} copies should be covered: {r:?} short of {amount}"
        );
    }
    let over = s.cost_of_next(id, n + 1);
    assert!(
        over.iter().any(|(r, amount)| s.stock(*r) + TOL < *amount),
        "{} copies should be beyond the stock",
        n + 1
    );
}

// ---------------------------------------------------------------------------
// SIM-ENG-044 / -045 / -046 — the gate, and what it is allowed to say
// ---------------------------------------------------------------------------

/// A meadow on its first tick has met almost nothing, and almost nothing may
/// say so. A kind visible from the start would state its whole ladder — every
/// name in it, and every price — before the player has met any of it.
///
/// The jobs a meadow opens with are the exception, and are checked here rather
/// than excused: the gate is open, so they are present, but only the ones the
/// meadow actually starts with.
/// @spec SIM-ENG-044
#[test]
fn a_meadow_at_tick_zero_states_nothing_it_has_not_reached() {
    let s = GameState::new(5);
    let shown = |all: Vec<ContentId>| -> usize {
        all.into_iter()
            .filter(|c| s.availability(*c) != Availability::Hidden)
            .count()
    };

    for (kind, count) in [
        (
            "structures",
            shown(
                StructureId::ALL
                    .iter()
                    .map(|i| ContentId::Structure(*i))
                    .collect(),
            ),
        ),
        (
            "ideas",
            shown(IdeaId::ALL.iter().map(|i| ContentId::Idea(*i)).collect()),
        ),
        (
            "crafts",
            shown(CraftId::ALL.iter().map(|i| ContentId::Craft(*i)).collect()),
        ),
        (
            "rites",
            shown(RiteId::ALL.iter().map(|i| ContentId::Rite(*i)).collect()),
        ),
        (
            "arch upgrades",
            shown(ArchId::ALL.iter().map(|i| ContentId::Arch(*i)).collect()),
        ),
        (
            "refinements",
            shown(
                RefinementId::ALL
                    .iter()
                    .map(|i| ContentId::Refinement(*i))
                    .collect(),
            ),
        ),
        (
            "house rules",
            shown(RuleId::ALL.iter().map(|i| ContentId::Rule(*i)).collect()),
        ),
        (
            "traditions",
            shown(
                TraditionId::ALL
                    .iter()
                    .map(|i| ContentId::Tradition(*i))
                    .collect(),
            ),
        ),
        (
            "dream-loom threads",
            shown(
                DreamLoomId::ALL
                    .iter()
                    .map(|i| ContentId::DreamLoom(*i))
                    .collect(),
            ),
        ),
    ] {
        assert_eq!(
            count, 0,
            "{count} {kind} are visible on a meadow that has not reached them — a kind with \
             no gate of its own spills its whole table on the first frame"
        );
    }

    let jobs = shown(JobId::ALL.iter().map(|i| ContentId::Job(*i)).collect());
    assert!(
        jobs > 0 && jobs < JobId::ALL.len(),
        "a new meadow shows {jobs} of {} jobs: it should show the ones it opens with and \
         no more",
        JobId::ALL.len()
    );
}

/// The gate query and the command layer must be the same gate. A caller
/// deciding what to show reads the query; acting on it runs the command. If
/// they disagree, a row offers something that refuses, or hides something that
/// would have worked.
///
/// Scoped to the actions with no content id of their own — the ones this query
/// exists for. An action backed by content answers through `availability`.
/// Five of the fifteen are held out, and the exclusion is a finding rather
/// than a convenience: see `the_gate_query_authors_rules_the_core_does_not`.
/// @spec SIM-ENG-045
#[test]
fn the_gate_query_and_the_command_layer_agree_on_every_gated_action() {
    let gated = [
        ("reverie", Command::Reverie),
        ("enlighten", Command::Enlighten),
        ("hold a parade", Command::HoldParade),
        ("scout a picnic", Command::ScoutPicnic),
        ("summon", Command::SummonStarWhale),
        ("feed", Command::FeedWhale),
        ("rummage", Command::Rummage),
        ("condense a jar", Command::CondenseJar),
        ("make a promise", Command::MakePromise),
        ("host a jamboree", Command::HostJamboree),
    ];

    let s = GameState::new(5);
    for (what, cmd) in gated {
        let open = s.command_gate_open(&cmd);
        let locked = matches!(s.clone().apply(cmd), Err(CommandError::Locked));
        assert_eq!(
            open,
            !locked,
            "on a meadow at tick zero the gate query says {what} is {} while the command \
             layer says {}: one of them is deciding a rule the other owns",
            if open { "open" } else { "closed" },
            if locked { "locked" } else { "not locked" }
        );
    }
}

/// And the agreement holds as a gate opens. A query that only ever answered
/// "closed" on a new meadow would pass the test above without deciding
/// anything.
/// @spec SIM-ENG-045
#[test]
fn a_gate_opens_for_the_query_and_the_command_layer_together() {
    let mut s = GameState::new(5);
    assert!(
        !s.command_gate_open(&Command::Rummage),
        "fixture: the gate should start closed, or opening it proves nothing"
    );

    s.ideas.insert(IdeaId::CouchSpelunking);

    assert!(
        s.command_gate_open(&Command::Rummage),
        "the idea that opens this action landed and the gate query did not notice"
    );
    assert!(
        !matches!(s.clone().apply(Command::Rummage), Err(CommandError::Locked)),
        "the gate query says the action exists for this meadow and the command layer still \
         refuses it as locked"
    );
}

/// The five the test above holds out, pinned so the defect cannot be forgotten
/// and cannot quietly change shape. Ignored because it fails: it asserts the
/// behaviour the arrow asks for, not the behaviour there is.
///
/// SIM-ENG-045 gives this query a purpose — that a caller deciding what to show
/// "does not restate rules the command layer already owns". For these five the
/// query is not restating a core rule; it is authoring one. The core would
/// refuse each of them for cost, or in the last case not refuse it at all, and
/// the only thing keeping them off the screen is this query's own opinion.
///
/// The last is the sharp one: an `Ok` that mutates nothing is neither of the
/// two outcomes SIM-ENG-005 allows, and it carries an event announcing a
/// turning point that did not turn.
///
/// Repairing this is a cascade through the segments that own these actions,
/// not through the engine, so it stops here and is reported rather than fixed.
/// @spec SIM-ENG-005, SIM-ENG-045
#[test]
#[ignore = "known defect: five gates live in the query rather than in the core"]
fn the_gate_query_authors_rules_the_core_does_not() {
    let s = GameState::new(5);

    for (what, cmd) in [
        ("basking", Command::Bask),
        ("refining a starshard", Command::RefineStarshard),
        ("refining an echo", Command::RefineEcho),
        ("moving the meadow", Command::GreatMeadowMove),
    ] {
        assert!(
            matches!(s.clone().apply(cmd), Err(CommandError::Locked)),
            "{what} is hidden by the gate query but the core does not call it locked: the \
             reason the player would be given and the reason the core would give differ"
        );
    }

    let outcome = s.clone().apply(Command::PassItDown);
    assert!(
        matches!(outcome, Err(CommandError::Locked)),
        "passing it down on a meadow that has met nothing returned {outcome:?}: the core \
         accepted an action it had nothing to act on, mutated nothing, and reported a \
         turning point anyway"
    );
}

/// Reach is a ratchet. A reveal keyed on the roster of the moment would
/// un-reveal the moment a kitten wandered off, taking an armed row out from
/// under the cursor.
/// @spec SIM-ENG-046
#[test]
fn the_roster_reach_never_falls() {
    let mut s = GameState::new(3);
    s.kittens = (0..5).map(|i| kitten(&format!("kitten-{i}"))).collect();
    run_ticks(&mut s, 1);
    assert_eq!(
        s.roster_reach(),
        5,
        "a roster of five was never recorded as having been met"
    );

    s.kittens.truncate(2);
    run_ticks(&mut s, 1);
    assert_eq!(
        s.roster_reach(),
        5,
        "reach fell to {} when three kittens wandered off; what prestige reveals is having \
         met a size, not currently having it",
        s.roster_reach()
    );
}

/// The other half of the ratchet: reach is never *below* the roster present,
/// even in the instant before a tick has recorded the growth.
/// @spec SIM-ENG-046
#[test]
fn the_roster_reach_is_never_below_the_roster_present() {
    let mut s = GameState::new(3);
    s.kittens = (0..9).map(|i| kitten(&format!("kitten-{i}"))).collect();
    assert_eq!(
        s.roster_reach(),
        9,
        "nine kittens are home and reach reports {} — a query that lags the state it \
         describes would hide a row that is already earned",
        s.roster_reach()
    );
}
