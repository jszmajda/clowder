//! Arrivals of the kinds without a reveal ladder: an item of such a kind
//! announces itself with a `Revealed` event when an applied command, or an
//! adventure's completion, leaves it present after being absent.
//!
//! @spec SIM-ENG-088

use crate::common::*;

use clowder_core::*;
use std::collections::BTreeSet;

fn two_state_ids() -> Vec<ContentId> {
    every_content_id()
        .into_iter()
        .filter(|id| !id.is_reveal_tracked())
        .collect()
}

/// The items of those kinds the core reports present.
fn present(s: &GameState) -> BTreeSet<ContentId> {
    two_state_ids()
        .into_iter()
        .filter(|id| s.availability(*id) != Availability::Hidden)
        .collect()
}

/// The arrivals among `events`, each as often as it was announced.
fn arrivals(events: &[Event]) -> Vec<ContentId> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Revealed { id } if !id.is_reveal_tracked() => Some(*id),
            _ => None,
        })
        .collect()
}

/// Through a save and a load, as a meadow is played after it is opened: the
/// core builds what it derives from the state there.
fn reloaded(s: &GameState) -> GameState {
    load_string(&save_string(s, 0), 0).unwrap().state
}

/// A meadow far along: every structure, plenty of every store, every
/// structure and idea revealed, and every idea but `missing` known.
fn far_along_but(missing: Option<IdeaId>) -> GameState {
    let mut s = GameState::new(88);
    for r in ResourceId::ALL {
        s.resources[r.ix()] = 1.0e9;
    }
    for b in StructureId::ALL {
        s.structures[b.ix()] = 2;
    }
    s.ideas = IdeaId::ALL
        .iter()
        .copied()
        .filter(|i| Some(*i) != missing)
        .collect();
    s.revealed = every_content_id()
        .into_iter()
        .filter(|id| id.is_reveal_tracked())
        .collect();
    reloaded(&s)
}

/// Researching an idea announces each item it opens, once, and nothing else;
/// one that opens nothing announces nothing.
// @spec SIM-ENG-088
#[test]
fn a_command_that_opens_items_announces_each_once() {
    let mut opened_some = 0;
    let mut opened_none = 0;
    for idea in IdeaId::ALL {
        let mut s = far_along_but(Some(idea));
        let before = present(&s);
        let Ok(events) = s.apply(Command::ResearchIdea(idea)) else {
            continue;
        };
        let after = present(&s);
        let expected: BTreeSet<ContentId> = after.difference(&before).copied().collect();
        let announced = arrivals(&events);
        assert_eq!(
            announced.iter().copied().collect::<BTreeSet<_>>(),
            expected,
            "researching {idea:?} announced {announced:?}"
        );
        assert_eq!(
            announced.len(),
            expected.len(),
            "researching {idea:?} announced an item twice: {announced:?}"
        );
        if expected.is_empty() {
            opened_none += 1;
        } else {
            opened_some += 1;
        }
    }
    assert!(opened_some > 0, "fixture: some idea opens an item");
    assert!(opened_none > 0, "fixture: some idea opens none");
}

/// What a load finds present counts as known: a command that opens nothing
/// announces nothing, however much is present.
// @spec SIM-ENG-088
#[test]
fn what_a_load_finds_present_counts_as_known() {
    let mut s = far_along_but(None);
    assert!(present(&s).len() > 20, "fixture: a great deal is present");
    let events = s.apply(Command::GatherCatnip).unwrap();
    assert!(
        arrivals(&events).is_empty(),
        "nothing arrived, yet {:?} was announced",
        arrivals(&events)
    );

    let mut fresh = GameState::new(89);
    let events = fresh.apply(Command::GatherCatnip).unwrap();
    assert!(
        arrivals(&events).is_empty(),
        "what a new game begins with is known: {:?}",
        arrivals(&events)
    );
}

/// A first Move announces what it opens, such as the traditions a first Move
/// brings, and nothing it merely keeps.
// @spec SIM-ENG-088
#[test]
fn a_first_move_announces_what_it_opens() {
    // The Move needs only the roster's size, so the kittens are pushed
    // straight in, as the Move's own tests do.
    let mut s = GameState::new(90);
    for i in 0..content::MEADOW_MOVE_MIN_KITTENS {
        let name = format!("Kit{i:03}");
        s.kittens.push(Kitten {
            name: name.clone(),
            job: None,
            location: Location::Home,
            arrived_day: i as u64,
            job_seq: None,
            away_seq: None,
            skills: [0.0; 8],
            exp: 0.0,
            rank: 0,
            trait_id: KittenTrait::None,
            tinkering: None,
            profile: Some(draw_profile(&name, 0)),
        });
    }
    let mut s = reloaded(&s);
    let before = present(&s);
    let events = s
        .apply(Command::GreatMeadowMove)
        .expect("the Move is taken");
    let after = present(&s);
    let expected: BTreeSet<ContentId> = after.difference(&before).copied().collect();
    assert!(
        expected
            .iter()
            .any(|id| matches!(id, ContentId::Tradition(_))),
        "fixture: a first Move opens traditions"
    );
    assert_eq!(
        arrivals(&events).into_iter().collect::<BTreeSet<_>>(),
        expected
    );
}

/// An adventure's completion at a day close announces the tradition it
/// opens, and the ticks around it announce nothing else.
// @spec SIM-ENG-088
#[test]
fn an_adventures_completion_announces_the_tradition_it_opens() {
    let a = AdventureId::TidyMeadow;
    let t = content::adventure_def(a)
        .unlocks
        .expect("fixture: this adventure opens a tradition");
    let mut s = GameState::new(91);
    s.generation = 1;
    s.active_adventure = Some(a);
    let mut s = reloaded(&s);
    assert!(
        !present(&s).contains(&ContentId::Tradition(t)),
        "fixture: the tradition waits on the adventure"
    );
    let events = run_days(&mut s, content::DAYS_PER_YEAR + 2);
    assert!(
        s.adventure_completions[a.ix()] > 0,
        "fixture: the year was tidy"
    );
    assert_eq!(
        arrivals(&events),
        vec![ContentId::Tradition(t)],
        "the completion announced its tradition, once, and nothing else arrived"
    );
}

/// Ticks with no completion announce nothing of these kinds.
// @spec SIM-ENG-088
#[test]
fn ticks_announce_no_arrival() {
    let mut s = far_along_but(None);
    let events = run_days(&mut s, 5);
    assert!(arrivals(&events).is_empty(), "{:?}", arrivals(&events));
}
