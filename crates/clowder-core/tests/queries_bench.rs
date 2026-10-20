//! Tests-first for the tinkerers' bench — where tinkerer seats come from, when
//! the Tinkerer commands exist, and the two queries a shell reads the bench
//! from:
//!
//!   * SIM-ECO-045 — every structure definition carries the seats each copy
//!     gives; the meadow's seats are copies held times seats per copy.
//!   * SIM-ECO-046 — AssignTinkerer and UnassignTinkerer exist exactly while
//!     the Tinkerer job is unlocked.
//!   * SIM-ENG-072 — why a bind would be refused, as a lock reason, in the
//!     command's own precedence, so it and the command never disagree.
//!   * SIM-ENG-073 — the bench: seats, every binding by roster position with
//!     its recipe, and every Tinkerer at home waiting without one.

mod common;

use clowder_core::*;
use common::*;

/// A meadow where the Tinkerer job and the first recipes are open, with
/// `workshops` seat-giving structures and `tinkerers` kittens holding the job.
fn tinker_world(seed: u64, workshops: u32, tinkerers: usize) -> GameState {
    let mut s = GameState::new(seed);
    s.ideas.insert(IdeaId::Toymaking);
    s.ideas.insert(IdeaId::Tinkering);
    adopt(&mut s, tinkerers);
    grant(&mut s, StructureId::ToyWorkshop, workshops);
    for _ in 0..tinkerers {
        s.apply(Command::AssignJob(JobId::Tinkerer)).unwrap();
    }
    s
}

// ---------------------------------------------------------------------------
// SIM-ECO-045 — seats are a field of the structure definition
// ---------------------------------------------------------------------------

/// The Toy Workshop seats one tinkerer per copy, and no other structure seats
/// any — read from the definitions, where the inspector reads them too.
/// @spec SIM-ECO-045
#[test]
fn only_the_toy_workshop_definition_gives_tinkerer_seats() {
    for s in StructureId::ALL {
        let expected = if s == StructureId::ToyWorkshop { 1 } else { 0 };
        assert_eq!(
            content::structure_def(s).tinkerer_seats,
            expected,
            "{s:?} gives {expected} tinkerer seats per copy"
        );
    }
}

/// The meadow's seats are the sum over structures of copies held times the
/// seats each copy gives: other structures add none, however many are held.
/// @spec SIM-ECO-045, SIM-ENG-073
#[test]
fn the_meadows_seats_are_copies_held_times_seats_per_copy() {
    let mut s = tinker_world(901, 0, 0);
    assert_eq!(s.bench().seats, 0, "no seat-giving structure, no seats");
    for b in StructureId::ALL {
        if b != StructureId::ToyWorkshop {
            grant(&mut s, b, 3);
        }
    }
    assert_eq!(s.bench().seats, 0, "structures that give no seats add none");
    for n in 1..=4 {
        grant(&mut s, StructureId::ToyWorkshop, 1);
        assert_eq!(s.bench().seats, n, "{n} workshops seat {n}");
    }
}

/// The bind refuses on the seats the definitions give: as many binds succeed
/// as the meadow has seats, and the next is refused for want of one.
/// @spec SIM-ECO-045, SIM-ECO-043
#[test]
fn binds_succeed_up_to_the_seats_and_no_further() {
    for seats in 0..=3u32 {
        let mut s = tinker_world(910 + seats as u64, seats, 5);
        for _ in 0..seats {
            s.apply(Command::AssignTinkerer(CraftId::Wands)).unwrap();
        }
        assert_eq!(
            s.apply(Command::AssignTinkerer(CraftId::Tiles))
                .unwrap_err(),
            CommandError::NoSeats,
            "{seats} seats, all taken"
        );
    }
}

// ---------------------------------------------------------------------------
// SIM-ECO-046 — the Tinkerer commands exist while the job does
// ---------------------------------------------------------------------------

/// Before the Tinkerer job is unlocked, neither tinkerer command's gate is
/// open for any recipe — even one the meadow can already make; once it is,
/// both are open for every recipe.
/// @spec SIM-ECO-046
#[test]
fn the_tinkerer_commands_gate_follows_the_tinkerer_job() {
    let mut s = GameState::new(920);
    s.ideas.insert(IdeaId::Tinkering);
    assert!(
        !s.job_unlocked(JobId::Tinkerer),
        "fixture: job still locked"
    );
    for c in CraftId::ALL {
        assert!(
            !s.command_gate_open(&Command::AssignTinkerer(c)),
            "{c:?}: no kitten can hold the job yet, so no bind exists"
        );
        assert!(
            !s.command_gate_open(&Command::UnassignTinkerer(c)),
            "{c:?}: no kitten can hold the job yet, so no unbind exists"
        );
    }
    s.ideas.insert(IdeaId::Toymaking);
    assert!(s.job_unlocked(JobId::Tinkerer), "fixture: job unlocked");
    for c in CraftId::ALL {
        assert!(s.command_gate_open(&Command::AssignTinkerer(c)), "{c:?}");
        assert!(s.command_gate_open(&Command::UnassignTinkerer(c)), "{c:?}");
    }
}

// ---------------------------------------------------------------------------
// SIM-ENG-073 — the bench
// ---------------------------------------------------------------------------

/// The bench lists every binding by roster position in roster order with its
/// recipe — an away kitten's included, since it keeps its seat — and every
/// Tinkerer at home with no binding, in roster order. A Tinkerer away without
/// a binding, and a kitten holding another job, are on neither list.
/// @spec SIM-ENG-073
#[test]
fn the_bench_lists_bindings_and_the_tinkerers_waiting_in_roster_order() {
    let mut s = tinker_world(930, 4, 6);
    // Newest unbound binds first: kittens 5, 4, 3 take seats.
    s.apply(Command::AssignTinkerer(CraftId::Tiles)).unwrap();
    s.apply(Command::AssignTinkerer(CraftId::Wands)).unwrap();
    s.apply(Command::AssignTinkerer(CraftId::Tiles)).unwrap();
    // Kitten 4 wanders off with its binding; kitten 1 wanders off unbound.
    s.kittens[4].location = Location::NeighborsYard;
    s.kittens[1].location = Location::NeighborsYard;
    // Kitten 0 changes trade: no longer a Tinkerer.
    s.kittens[0].job = Some(JobId::Gardener);

    let before = format!("{s:?}");
    let bench = s.bench();
    assert_eq!(format!("{s:?}"), before, "asking changes nothing");

    assert_eq!(bench.seats, 4);
    assert_eq!(
        bench.seated,
        vec![
            (3, CraftId::Tiles),
            (4, CraftId::Wands),
            (5, CraftId::Tiles),
        ],
        "every binding, the away kitten's included, in roster order"
    );
    assert_eq!(
        bench.waiting,
        vec![2],
        "only the unbound Tinkerer at home waits — not the away one, not the gardener"
    );
}

/// Waiting is exactly who a bind can take: with a free seat, a bind moves the
/// newest waiting kitten from waiting to seated.
/// @spec SIM-ENG-073, SIM-ECO-043
#[test]
fn a_bind_moves_the_newest_waiting_kitten_onto_the_bench() {
    let mut s = tinker_world(931, 2, 3);
    assert_eq!(s.bench().waiting, vec![0, 1, 2]);
    assert!(s.bench().seated.is_empty());
    s.apply(Command::AssignTinkerer(CraftId::Wands)).unwrap();
    let bench = s.bench();
    assert_eq!(bench.seated, vec![(2, CraftId::Wands)]);
    assert_eq!(bench.waiting, vec![0, 1]);
}

// ---------------------------------------------------------------------------
// SIM-ENG-072 — why a bind is refused
// ---------------------------------------------------------------------------

/// Each reason, alone: a locked recipe, no one waiting, no seat free — and
/// none where the bind would be made.
/// @spec SIM-ENG-072
#[test]
fn the_bind_reason_names_each_blocker() {
    let s = tinker_world(940, 1, 1);
    assert_eq!(
        s.tinkerer_bind_reason(CraftId::Wands),
        None,
        "a bind would seat the kitten"
    );
    assert_eq!(
        s.tinkerer_bind_reason(CraftId::Cogs),
        Some(LockReason::Prerequisite),
        "the recipe is locked"
    );

    let s = tinker_world(941, 2, 0);
    assert_eq!(
        s.tinkerer_bind_reason(CraftId::Wands),
        Some(LockReason::NoOneFree),
        "seats free, but no Tinkerer waits"
    );

    let s = tinker_world(942, 0, 2);
    assert_eq!(
        s.tinkerer_bind_reason(CraftId::Wands),
        Some(LockReason::NoRoom),
        "Tinkerers wait, but there is no seat"
    );
}

/// Where several blockers hold, the first in the command's precedence wins:
/// Prerequisite, then NoOneFree, then NoRoom.
/// @spec SIM-ENG-072, SIM-ENG-006
#[test]
fn the_bind_reason_resolves_in_the_commands_precedence() {
    // Locked recipe, nobody waiting, no seats.
    let s = tinker_world(950, 0, 0);
    assert_eq!(
        s.tinkerer_bind_reason(CraftId::Cogs),
        Some(LockReason::Prerequisite)
    );
    // Nobody waiting and no seats: no-one-free first.
    assert_eq!(
        s.tinkerer_bind_reason(CraftId::Wands),
        Some(LockReason::NoOneFree)
    );
    // Every seat full and every Tinkerer bound.
    let mut s = tinker_world(951, 1, 1);
    s.apply(Command::AssignTinkerer(CraftId::Wands)).unwrap();
    assert_eq!(
        s.tinkerer_bind_reason(CraftId::Tiles),
        Some(LockReason::NoOneFree)
    );
}

/// The reason and the command never disagree: across a sweep of seats,
/// Tinkerers, bindings, absences, and recipes, the reason is none exactly
/// where the bind succeeds, and otherwise names the cause the command's
/// refusal names. Asking changes nothing.
/// @spec SIM-ENG-072
#[test]
fn the_bind_reason_agrees_with_the_command_everywhere() {
    let expect = |e: &CommandError| match e {
        CommandError::Locked => LockReason::Prerequisite,
        CommandError::NoIdleKittens => LockReason::NoOneFree,
        CommandError::NoSeats => LockReason::NoRoom,
        other => panic!("AssignTinkerer refused with {other:?}, which no bind reason names"),
    };
    let mut checked = 0;
    for seats in 0..=3u32 {
        for tinkerers in 0..=3usize {
            for prebound in 0..=tinkerers.min(seats as usize) {
                for away in 0..=tinkerers {
                    let mut s = tinker_world(960, seats, tinkerers);
                    for _ in 0..prebound {
                        s.apply(Command::AssignTinkerer(CraftId::Wands)).unwrap();
                    }
                    for k in s.kittens.iter_mut().take(away) {
                        k.location = Location::NeighborsYard;
                    }
                    for c in CraftId::ALL {
                        let before = format!("{s:?}");
                        let reason = s.tinkerer_bind_reason(c);
                        assert_eq!(format!("{s:?}"), before, "asking changes nothing");
                        let mut probe = s.clone();
                        match probe.apply(Command::AssignTinkerer(c)) {
                            Ok(_) => assert_eq!(reason, None, "{c:?}: the bind succeeded"),
                            Err(e) => {
                                assert_eq!(reason, Some(expect(&e)), "{c:?}: refused with {e:?}")
                            }
                        }
                        checked += 1;
                    }
                }
            }
        }
    }
    assert!(
        checked > 100,
        "fixture: the sweep covers many states ({checked})"
    );
}
