//! Achievements: the table, earning, the record, and that nothing reads it.
//!
//!   * SIM-ACH-001 — the table
//!   * SIM-ACH-002, 006 — earning, by state at a day close or after a command,
//!     by moment against the events just emitted
//!   * SIM-ACH-003, 004 — read by no rule, drawing nothing from the state RNG
//!   * SIM-ACH-005 — the query
//!   * SIM-MOVE-004 — kept across a Move
//!   * SAVE-030 — saved, and a v9 save migrating with an empty record

mod common;

use std::collections::BTreeSet;

use clowder_core::content;
use clowder_core::*;
use common::*;

fn earned_events(events: &[Event]) -> Vec<AchievementId> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::AchievementEarned { id } => Some(*id),
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// SIM-ACH-001
// ---------------------------------------------------------------------------

/// At least forty achievements, every one named, hinted, and worded, no two
/// names alike, at least eight secret.
/// @spec SIM-ACH-001
#[test]
fn the_table_is_full_and_tidy() {
    assert!(
        AchievementId::ALL.len() >= 40,
        "{}",
        AchievementId::ALL.len()
    );
    let mut names = BTreeSet::new();
    let mut secret = 0;
    for a in AchievementId::ALL {
        let d = content::achievement_def(a);
        assert!(
            !d.name.trim().is_empty() && !d.hint.trim().is_empty() && !d.words.trim().is_empty(),
            "{a:?}"
        );
        assert!(
            names.insert(d.name.to_lowercase()),
            "{a:?}: {:?} twice",
            d.name
        );
        secret += usize::from(d.secret);
    }
    assert!(secret >= 8, "{secret} secret");
}

// ---------------------------------------------------------------------------
// SIM-ACH-002, 006 — earning
// ---------------------------------------------------------------------------

/// A state condition is noticed at the day close and recorded once, with the
/// generation and day, as a milestone the journal keeps.
/// @spec SIM-ACH-002, SIM-ACH-006
#[test]
fn a_state_achievement_is_earned_at_the_day_close_once() {
    let mut s = GameState::new(1601);
    s.generation = 2;
    assert!(s.achievements.is_empty());
    // adopt() runs whole days; the arrival lands at a day close, and the
    // journal keeps what that close emitted.
    adopt(&mut s, 1);
    let events: Vec<Event> = s
        .journal
        .iter_chronological()
        .into_iter()
        .filter_map(|e| match &e.item {
            JournalItem::Happened(ev) => Some(ev.clone()),
            _ => None,
        })
        .collect();
    assert!(
        earned_events(&events).contains(&AchievementId::AFaceAtTheDoor),
        "{events:?}"
    );
    let earned = s
        .achievements
        .get(&AchievementId::AFaceAtTheDoor)
        .copied()
        .expect("recorded");
    assert_eq!(earned.generation, 2);
    assert_eq!(earned.day, s.day());
    assert_eq!(
        Event::AchievementEarned {
            id: AchievementId::AFaceAtTheDoor
        }
        .class(),
        EventClass::Milestone
    );
    // Never again.
    let later = run_days(&mut s, 5);
    assert!(!earned_events(&later).contains(&AchievementId::AFaceAtTheDoor));
}

/// A state condition met by a command is noticed in that command's events.
/// @spec SIM-ACH-002, SIM-ACH-006
#[test]
fn a_command_can_earn_an_achievement_at_once() {
    let mut s = GameState::new(1602);
    give(&mut s, ResourceId::Catnip, 200.0);
    let events = s.apply(Command::Build(StructureId::CatnipPatch)).unwrap();
    assert_eq!(earned_events(&events), vec![AchievementId::HomeImprovement]);
    assert!(s.achievements.contains_key(&AchievementId::HomeImprovement));
    let events = s.apply(Command::Build(StructureId::CatnipPatch)).unwrap();
    assert!(earned_events(&events).is_empty(), "earned once");
}

/// A speed change, and a command that emits nothing, are no occasion to look:
/// a condition already holding waits for the day's close, or for a command
/// that tells of something.
/// @spec SIM-ACH-002
#[test]
fn only_a_command_that_tells_of_something_prompts_a_look() {
    let mut s = GameState::new(1610);
    adopt(&mut s, 2);
    s.elder = Some(0);
    s.achievements.clear();
    for speed in [8.0, 0.25, 1.0] {
        let events = s.apply(Command::SetSpeed(speed)).unwrap();
        assert!(earned_events(&events).is_empty(), "{speed}: {events:?}");
    }
    let events = s.apply(Command::GatherCatnip).unwrap();
    assert!(
        events.is_empty(),
        "fixture: a command that tells of nothing"
    );
    let events = s.apply(Command::AppointElder(0)).unwrap();
    assert!(events.is_empty(), "fixture: the Elder already sits");
    assert!(s.achievements.is_empty(), "nothing looked");
    let events = s.apply(Command::AssignJob(JobId::Gardener)).unwrap();
    let earned = earned_events(&events);
    assert!(
        earned.contains(&AchievementId::AFaceAtTheDoor),
        "{earned:?}"
    );
    assert!(earned.contains(&AchievementId::WiseWhiskers), "{earned:?}");
}

/// A condition reading individual kittens waits for a day that brought a
/// kitten, a new year, or a command that tells of something; one reading a
/// count does not.
/// @spec SIM-ACH-002
#[test]
fn kitten_reading_conditions_wait_for_a_reason_to_look() {
    let mut s = GameState::new(1611);
    adopt(&mut s, 1);
    s.achievements.clear();
    let mut p = s.kittens[0].profile.clone().unwrap();
    p.eyes = EyeLook::OddBlueGreen;
    s.kittens[0].profile = Some(p);
    // Keep the roster still: no room for another, nothing to send anyone off.
    let events = run_days(&mut s, 1);
    assert!(
        !events.iter().any(|e| matches!(
            e,
            Event::KittenArrived { .. }
                | Event::KittenWanderedOff { .. }
                | Event::KittenReturned { .. }
                | Event::NewYear { .. }
        )),
        "fixture: a quiet day {events:?}"
    );
    assert!(
        s.achievements.contains_key(&AchievementId::AFaceAtTheDoor),
        "a count, daily"
    );
    assert!(
        !s.achievements.contains_key(&AchievementId::OddOneOut),
        "kittens wait"
    );
    let events = s.apply(Command::AssignJob(JobId::Gardener)).unwrap();
    assert!(
        earned_events(&events).contains(&AchievementId::OddOneOut),
        "{events:?}"
    );
}

/// A moment condition is noticed against the events of the tick that made it.
/// @spec SIM-ACH-002, SIM-ACH-006
#[test]
fn a_moment_achievement_is_earned_from_its_event() {
    let mut s = GameState::new(1603);
    // Zoomies roll at day boundaries; run until one starts.
    let mut found = false;
    for _ in 0..2000 {
        let events = run_ticks(&mut s, 1);
        if events
            .iter()
            .any(|e| matches!(e, Event::ZoomiesStarted { .. }))
        {
            assert!(
                earned_events(&events).contains(&AchievementId::TheZoomies),
                "{events:?}"
            );
            found = true;
            break;
        }
    }
    assert!(found, "fixture: the zoomies started");
    assert!(s.achievements.contains_key(&AchievementId::TheZoomies));
}

/// A sample of conditions, each decided as the table states it.
/// @spec SIM-ACH-006
#[test]
fn conditions_read_the_meadow_as_the_table_says() {
    // A kitten in surprising fur, two sharing a favourite food, odd eyes.
    let mut s = GameState::new(1604);
    adopt(&mut s, 2);
    let mut a = s.kittens[0].profile.clone().unwrap();
    a.fur.colours = vec![content::SURPRISING_COLOURS[0]; a.fur.colours.len()];
    a.eyes = EyeLook::OddBlueCopper;
    let food = Favourite {
        kind: FavouriteKind::Food,
        entry: "toast".into(),
    };
    a.favourites = vec![food.clone()];
    s.kittens[0].profile = Some(a);
    let mut b = s.kittens[1].profile.clone().unwrap();
    b.favourites = vec![food];
    s.kittens[1].profile = Some(b);
    // A command that tells of something looks at every kitten.
    s.apply(Command::AssignJob(JobId::Gardener)).unwrap();
    for id in [
        AchievementId::OnceInABlueMoon,
        AchievementId::OddOneOut,
        AchievementId::SnackBuddies,
    ] {
        assert!(s.achievements.contains_key(&id), "{id:?}");
    }
    assert!(
        !s.achievements.contains_key(&AchievementId::AProperClowder),
        "only two kittens"
    );

    // A napping caravan is a moment, read from its event.
    let mut t = GameState::new(1605);
    t.ideas.insert(IdeaId::GiftEconomy);
    t.discovered.insert(ColonyId::Owls); // shy: some caravans nap
    give(&mut t, ResourceId::Pep, 50_000.0);
    give(&mut t, ResourceId::Shinies, 15_000.0);
    give(&mut t, ResourceId::Mint, 50_000.0);
    let events = t.apply(Command::Trade(ColonyId::Owls, 200)).unwrap();
    let napped = events
        .iter()
        .any(|e| matches!(e, Event::TradeReturned { napped, .. } if *napped > 0));
    assert!(napped, "fixture: a shy neighbour napped");
    let earned = earned_events(&events);
    assert!(
        earned.contains(&AchievementId::ShhTheyreSleeping),
        "{earned:?}"
    );
    assert!(
        earned.contains(&AchievementId::OpenForBusiness),
        "{earned:?}"
    );
}

// ---------------------------------------------------------------------------
// SIM-ACH-003, 004 — read by nothing, drawing nothing
// ---------------------------------------------------------------------------

/// Two meadows differing only in their records stay identical, records and
/// journals aside, and the RNG with them.
/// @spec SIM-ACH-003, SIM-ACH-004
#[test]
fn nothing_reads_the_record_and_no_draw_is_taken() {
    let mut a = GameState::new(1606);
    adopt(&mut a, 6);
    grant(&mut a, StructureId::CozyCushion, 4);
    let mut b = a.clone();
    for id in AchievementId::ALL {
        b.achievements.insert(
            id,
            Earned {
                generation: 0,
                day: 0,
            },
        );
    }
    for s in [&mut a, &mut b] {
        give(s, ResourceId::Catnip, 400.0);
        let _ = s.apply(Command::AssignJob(JobId::Gardener));
        let _ = s.apply(Command::SetSpeed(8.0));
        run_days(s, 120);
    }
    assert_eq!(a.rng, b.rng, "the same draws");
    let strip = |s: &GameState| save_string(&without_achievements(s), 1_000);
    assert_eq!(strip(&a), strip(&b));
}

// ---------------------------------------------------------------------------
// SIM-ACH-005 — the query
// ---------------------------------------------------------------------------

/// Every achievement in table order, with when it was earned where it was.
/// @spec SIM-ACH-005
#[test]
fn the_query_lists_every_achievement_in_order() {
    let mut s = GameState::new(1607);
    s.achievements.insert(
        AchievementId::MovingDay,
        Earned {
            generation: 1,
            day: 42,
        },
    );
    let list = s.achievement_list();
    let ids: Vec<AchievementId> = list.iter().map(|(id, _, _)| *id).collect();
    assert_eq!(ids, AchievementId::ALL.to_vec());
    for (id, def, earned) in list {
        assert_eq!(def, content::achievement_def(id));
        assert_eq!(earned, s.achievements.get(&id).copied(), "{id:?}");
    }
}

// ---------------------------------------------------------------------------
// SIM-MOVE-004 — kept across a Move
// ---------------------------------------------------------------------------

/// A Move keeps the record whole.
/// @spec SIM-MOVE-004
#[test]
fn a_move_keeps_the_record() {
    let mut s = GameState::new(1608);
    adopt(&mut s, 1);
    let template = s.kittens[0].clone();
    s.kittens = (0..71)
        .map(|i| Kitten {
            name: content::NAME_POOL[i].to_string(),
            ..template.clone()
        })
        .collect();
    s.achievements.insert(
        AchievementId::PureBliss,
        Earned {
            generation: 0,
            day: 9,
        },
    );
    let before = s.achievements.clone();
    s.apply(Command::GreatMeadowMove).expect("a Move");
    for (id, e) in &before {
        assert_eq!(s.achievements.get(id), Some(e), "{id:?} kept");
    }
}

// ---------------------------------------------------------------------------
// SAVE-030 — saved and migrated
// ---------------------------------------------------------------------------

const V9_FIXTURE: &str = include_str!("fixtures/save_v9.json");

/// The record survives a save and a load, under a version newer than v9.
/// @spec SAVE-030
#[test]
fn the_record_is_saved() {
    const { assert!(SAVE_VERSION >= 10, "a version step carries it") };
    let mut s = GameState::new(1609);
    s.achievements.insert(
        AchievementId::Sprawl,
        Earned {
            generation: 3,
            day: 777,
        },
    );
    let back = load_string(&save_string(&s, 1_000), 1_000).unwrap().state;
    assert_eq!(back.achievements, s.achievements);
}

/// A v9 save migrates with an empty record and nothing else changed.
/// @spec SAVE-030, SAVE-013
#[test]
fn a_v9_save_migrates_with_an_empty_record() {
    assert!(V9_FIXTURE.contains("\"save_version\": 9"));
    let before: serde_json::Value = serde_json::from_str(V9_FIXTURE).unwrap();
    let at = before["saved_at_unix"].as_u64().unwrap();
    let sd = begin_load(V9_FIXTURE, at)
        .expect("a v9 save loads")
        .set_down();
    assert!(
        sd.state.achievements.is_empty(),
        "nothing is credited on load itself"
    );
    let resaved: serde_json::Value = serde_json::from_str(&save_string_carrying(
        &sd.state,
        at,
        sd.owed_ticks,
        Some(&sd.report_so_far),
    ))
    .unwrap();
    for (key, old) in before["state"].as_object().unwrap() {
        assert_eq!(
            old, &resaved["state"][key],
            "migrating a v9 save changed `{key}`"
        );
    }
    let added: Vec<&String> = resaved["state"]
        .as_object()
        .unwrap()
        .keys()
        .filter(|k| !before["state"].as_object().unwrap().contains_key(*k))
        .collect();
    assert_eq!(added, vec!["achievements"]);
}

/// A catch-up answers the record as the save held it, before any tick
/// replayed — what it earns along the way is not among it.
/// @spec SAVE-030
#[test]
fn a_catch_up_knows_the_record_as_loaded() {
    let mut s = GameState::new(1612);
    s.achievements.insert(
        AchievementId::MovingDay,
        Earned {
            generation: 0,
            day: 0,
        },
    );
    // Holds already, and is credited at the first day close the replay runs.
    s.ideas.insert(IdeaId::PantryOrganization);
    let json = save_string(&s, 0);
    let as_loaded = BTreeSet::from([AchievementId::MovingDay]);
    let c = begin_load(&json, 86_400).unwrap();
    assert_eq!(c.earned_at_load(), &as_loaded);
    let c = match c.advance(20) {
        Advance::Pending(c) => c,
        Advance::Finished(_) => panic!("fixture: a day's catch-up outlasts twenty ticks"),
    };
    assert_eq!(c.earned_at_load(), &as_loaded, "unmoved by the replay");
    let loaded = match c.advance(u64::MAX) {
        Advance::Finished(g) => g,
        Advance::Pending(_) => unreachable!("an unbounded advance finishes"),
    };
    assert!(loaded
        .state
        .achievements
        .contains_key(&AchievementId::ABrightIdea));
}
