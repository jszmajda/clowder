//! Shared scenario helpers. Tests manipulate public state fields directly to
//! construct scenarios, then assert on behavior through tick()/apply().
#![allow(dead_code)]

use clowder_core::*;

pub fn run_ticks(s: &mut GameState, n: u64) -> Vec<Event> {
    (0..n).flat_map(|_| s.tick()).collect()
}

/// Run whole days. Assumes (and asserts) day-aligned ticks so boundary
/// processing lands predictably.
pub fn run_days(s: &mut GameState, days: u64) -> Vec<Event> {
    assert_eq!(s.ticks % 10, 0, "run_days requires day-aligned state");
    run_ticks(s, days * 10)
}

pub fn give(s: &mut GameState, r: ResourceId, amount: f64) {
    s.resources[r.ix()] = amount;
}

pub fn grant(s: &mut GameState, st: StructureId, n: u32) {
    s.structures[st.ix()] += n;
}

/// Adopt exactly `n` kittens by forcing pity-guaranteed arrivals, one per day.
/// Ensures housing and food eligibility; leaves stock wherever the sim put it.
pub fn adopt(s: &mut GameState, n: usize) {
    for _ in 0..n {
        while s.free_slots() == 0 {
            grant(s, StructureId::CozyCushion, 1);
        }
        let stock = s.stock(ResourceId::Catnip);
        give(s, ResourceId::Catnip, stock.max(100.0));
        s.pity = 4;
        let before = s.kittens.len();
        let events = run_days(s, 1);
        assert_eq!(
            s.kittens.len(),
            before + 1,
            "pity-forced arrival must land in one day; events: {events:?}"
        );
    }
}

pub fn names_of(events: &[Event]) -> Vec<String> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::KittenArrived { name } => Some(name.clone()),
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Revealed resources — the oracles SIM-ENG-048..056 and SAVE-023/024 share
// ---------------------------------------------------------------------------

use std::collections::BTreeSet;

/// Every content id the core knows, across every kind.
pub fn every_content_id() -> Vec<ContentId> {
    let mut ids = Vec::new();
    ids.extend(StructureId::ALL.iter().map(|x| ContentId::Structure(*x)));
    ids.extend(IdeaId::ALL.iter().map(|x| ContentId::Idea(*x)));
    ids.extend(CraftId::ALL.iter().map(|x| ContentId::Craft(*x)));
    ids.extend(JobId::ALL.iter().map(|x| ContentId::Job(*x)));
    ids.extend(RiteId::ALL.iter().map(|x| ContentId::Rite(*x)));
    ids.extend(ArchId::ALL.iter().map(|x| ContentId::Arch(*x)));
    ids.extend(RefinementId::ALL.iter().map(|x| ContentId::Refinement(*x)));
    ids.extend(RuleId::ALL.iter().map(|x| ContentId::Rule(*x)));
    ids.extend(TraditionId::ALL.iter().map(|x| ContentId::Tradition(*x)));
    ids.extend(DreamLoomId::ALL.iter().map(|x| ContentId::DreamLoom(*x)));
    ids
}

/// The resources a content item's price names: the components of the price of
/// one more of it, as the cost query states them (SIM-ENG-038).
pub fn price_components(s: &GameState, id: ContentId) -> BTreeSet<ResourceId> {
    s.cost_of_next(id, 1).into_iter().map(|(r, _)| r).collect()
}

/// Every count-taking command priced per unit: a pounce, and a trade with
/// every colony there is, the whale's exchange among them (SIM-ENG-047).
pub fn per_unit_commands() -> Vec<Command> {
    let mut out = vec![Command::Pounce(1)];
    out.extend(ColonyId::ALL.map(|c| Command::Trade(c, 1)));
    out
}

/// The resources a meadow holds: every stock standing above zero, by any
/// amount (SIM-ENG-048 draws no dust guard).
pub fn held_resources(s: &GameState) -> BTreeSet<ResourceId> {
    ResourceId::ALL
        .into_iter()
        .filter(|r| s.stock(*r) > 0.0)
        .collect()
}

/// The resources named in a price the core would state for this meadow,
/// assembled from the three queries SIM-ENG-049 names and nothing else: the
/// price of one more of every content item the core reports Teased or
/// Available (owned or not), the per-unit price of every count-taking command
/// that query answers for, and a deep nap's cost — starshards, by
/// SIM-DREAM-001 — while that command's gate is open.
pub fn priced_resources(s: &GameState) -> BTreeSet<ResourceId> {
    let mut out = BTreeSet::new();
    for id in every_content_id() {
        if s.availability(id) != Availability::Hidden {
            out.extend(price_components(s, id));
        }
    }
    for cmd in per_unit_commands() {
        if let Some(price) = s.unit_price(&cmd) {
            out.extend(price.into_iter().map(|(r, _)| r));
        }
    }
    if s.command_gate_open(&Command::DeepNap(1)) {
        out.insert(ResourceId::Starshards);
    }
    out
}

/// The revealed-resource set as the meadow would persist it: the
/// `revealed_resources` array of the state's own serialization, in the order
/// it is written. Read from the save bytes rather than from a field, so what
/// is asserted is what is recorded — the query (SIM-ENG-053) also answers for
/// a holding no record has yet been made of, and would hide a record that was
/// never made.
pub fn recorded_resources(s: &GameState) -> Vec<ResourceId> {
    let v: serde_json::Value = serde_json::from_str(&save_string(s, 0)).expect("a save is JSON");
    let set = v["state"].get("revealed_resources").unwrap_or_else(|| {
        panic!(
            "the meadow's state serializes no `revealed_resources`: the revealed resources are \
             held in game state as a persisted set (SIM-ENG-051)"
        )
    });
    serde_json::from_value(set.clone()).unwrap_or_else(|e| {
        panic!("`revealed_resources` should serialize as a list of resource ids, got {set}: {e}")
    })
}

/// As [`recorded_resources`], as a set.
pub fn recorded_set(s: &GameState) -> BTreeSet<ResourceId> {
    recorded_resources(s).into_iter().collect()
}

/// An identifier spelled as words: a space before each capital after the
/// first.
pub fn spaced(tag: &str) -> String {
    let mut out = String::new();
    for (i, c) in tag.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

/// A spelling with its last word made singular: a final "ies" read as "y",
/// otherwise a final "s" dropped.
pub fn singular(name: &str) -> String {
    if let Some(stem) = name.strip_suffix("ies") {
        format!("{stem}y")
    } else if let Some(stem) = name.strip_suffix('s') {
        stem.to_string()
    } else {
        name.to_string()
    }
}

/// Every name the game gives a thing that is not a kitten, lower-cased: each
/// resource as the stores spell it and with its last word made singular, and
/// every structure, job, idea, craft, refinement, house rule, rite, arch,
/// tradition, dream loom, colony, site, adventure, season, and kitten trait
/// as the game spells it — a trait by its own name and by the name the
/// player reads it by. The vocabulary the name pool (SIM-KIT-029) and the
/// favourites (SIM-KIT-038) are kept apart from.
pub fn game_vocabulary() -> Vec<String> {
    let mut others: Vec<String> = Vec::new();
    for r in ResourceId::ALL {
        let name = spaced(&format!("{r:?}"));
        others.push(singular(&name));
        others.push(name);
    }
    let debug = |v: &dyn std::fmt::Debug| spaced(&format!("{v:?}"));
    for s in StructureId::ALL {
        others.push(content::structure_def(s).name.to_string());
    }
    for j in JobId::ALL {
        others.push(content::job_def(j).name.to_string());
    }
    for i in IdeaId::ALL {
        others.push(content::idea_def(i).name.to_string());
    }
    for c in CraftId::ALL {
        others.push(content::craft_def(c).name.to_string());
    }
    for r in RefinementId::ALL {
        others.push(content::refinement_def(r).name.to_string());
    }
    for r in RuleId::ALL {
        others.push(content::rule_def(r).name.to_string());
    }
    for r in RiteId::ALL {
        others.push(content::rite_def(r).name.to_string());
    }
    for a in ArchId::ALL {
        others.push(content::arch_def(a).name.to_string());
    }
    for t in TraditionId::ALL {
        others.push(debug(&t));
    }
    for l in DreamLoomId::ALL {
        others.push(content::dream_loom_def(l).name.to_string());
    }
    for c in ColonyId::ALL {
        others.push(content::colony_def(c).name.to_string());
        others.push(debug(&c));
    }
    for s in SiteId::ALL {
        others.push(content::site_def(s).name.to_string());
        others.push(debug(&s));
    }
    for a in AdventureId::ALL {
        others.push(debug(&a));
    }
    for s in Season::ALL {
        others.push(debug(&s));
    }
    for t in KittenTrait::ALL {
        others.push(debug(&t));
        others.push(content::trait_def(t).name.to_string());
    }
    others.iter().map(|o| o.to_lowercase()).collect()
}

/// Whether a migrated save's `now` keeps the `was` its fixture held: equal as
/// numbers wherever both are numbers, save that an id-indexed array may have
/// grown since the fixture was written by a tail of zeros — ids only ever
/// append, and a load zero-fills the ids a save predates.
pub fn kept(was: &serde_json::Value, now: &serde_json::Value) -> bool {
    use serde_json::Value as V;
    match (was, now) {
        (V::Number(x), V::Number(y)) => {
            if let (Some(i), Some(j)) = (x.as_i64(), y.as_i64()) {
                return i == j;
            }
            x.as_f64() == y.as_f64()
        }
        (V::Array(x), V::Array(y)) => {
            y.len() >= x.len()
                && x.iter().zip(y).all(|(p, q)| kept(p, q))
                && y[x.len()..].iter().all(|v| v.as_f64() == Some(0.0))
        }
        (V::Object(x), V::Object(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| kept(v, w)))
        }
        _ => was == now,
    }
}

/// Take the achievement record out of a migrated save's state, checking it
/// arrived empty (SAVE-030), so a test of an earlier migration step reads only
/// what that step changed.
pub fn take_empty_record(resaved: &mut serde_json::Value) {
    let record = resaved["state"]
        .as_object_mut()
        .expect("a save holds its state")
        .remove("achievements");
    assert!(
        record.is_some_and(|r| r.as_object().is_some_and(|m| m.is_empty())),
        "a migrated save's achievement record arrives empty (SAVE-030)"
    );
}

/// The meadow with its achievements set aside: the record cleared, and the
/// journal without the entries earning wrote — milestones, so all in the kept
/// list (SIM-ACH-002).
pub fn without_achievements(s: &GameState) -> GameState {
    let mut s = s.clone();
    s.achievements.clear();
    s.journal.kept.retain(|e| {
        !matches!(
            e.item,
            JournalItem::Happened(Event::AchievementEarned { .. })
        )
    });
    s
}

/// Mark every achievement earned, for a test that measures what something
/// else writes to the events or the journal: each achievement is earned once,
/// on its own account (SIM-ACH-002), and would otherwise ride along.
pub fn all_earned(s: &mut GameState) {
    let when = Earned {
        generation: s.generation,
        day: s.day(),
    };
    for id in AchievementId::ALL {
        s.achievements.insert(id, when);
    }
}
