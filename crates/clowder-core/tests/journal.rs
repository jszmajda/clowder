//! The event journal — classification, the bounded ring, eviction, and the
//! live-versus-replay rule.
//!
//! Specs exercised here:
//!   SIM-ENG-026  every event carries a classification, queryable on the event
//!   SIM-ENG-027  the journal is game state: a bounded rolling window plus
//!                every milestone forever, each entry stamped with its tick
//!   SIM-ENG-028  live ticks and applied commands append; catch-up replay
//!                never does — an absence is its folded report, not its
//!                millions of constituent events
//!   SIM-ENG-029  a full window evicts its oldest non-milestone entry
//!   SIM-ENG-040  the core decides live-versus-replay; replay has its own
//!                non-journaling entry point, so no caller can journal a
//!                replay by omission
//!
//! The classification table itself is content the design does not enumerate,
//! so these tests never hard-code "which events are milestones" beyond the one
//! ranking the design states outright (reaching a new site matters more than a
//! job reassignment). Everywhere else they *discover* a milestone and a
//! non-milestone from the vocabulary through `Event::class()` — which is also
//! the coupling SIM-ENG-026 describes: classification is what lets the journal
//! know what it may never evict.

mod common;

use clowder_core::*;
use common::*;
use std::collections::BTreeSet;

// =====================================================================
// Vocabulary sampling
// =====================================================================

/// One value of every `Event` variant. `variant_name` below is an exhaustive
/// match, so adding a variant to the vocabulary breaks this file's build —
/// which is the point: SIM-ENG-026 says *every* event is classified, and a
/// sample that quietly missed a variant would not be testing that.
fn vocabulary() -> Vec<Event> {
    vec![
        Event::KittenArrived { name: "Pib".into() },
        Event::KittenWanderedOff { name: "Pib".into() },
        Event::KittenReturned { name: "Pib".into() },
        Event::JobAssigned { name: "Pib".into(), job: JobId::Gardener },
        Event::JobUnassigned { name: "Pib".into(), job: JobId::Gardener },
        Event::SeasonChanged { season: Season::Summer },
        Event::NewYear { year: 3 },
        Event::StructureBuilt { id: StructureId::CatnipPatch, count: 2 },
        Event::IdeaResearched { id: IdeaId::DeepNapTheory },
        Event::ZoomiesStarted { until_tick: 120 },
        Event::ZoomiesEnded,
        Event::KittenMood { tier: MoodTier::Grumpy },
        Event::Flavor { template: 0, kitten: None },
        Event::Revealed { id: ContentId::Structure(StructureId::CatnipPatch) },
        Event::SkyEventVisible,
        Event::SpeedChanged { speed: 2.0 },
        Event::PounceReturned {
            squads: 1,
            feathers: 1.0,
            bells: 1.0,
            shinies: 1.0,
            unicorns: 0,
        },
        Event::JamboreeStarted { until_tick: 1000 },
        Event::JamboreeEnded,
        Event::ElderAppointed { name: "Pib".into() },
        Event::KittenPromoted { name: "Pib".into(), rank: 1 },
        Event::RefinementBought { id: RefinementId::PebbleCombs },
        Event::RuleAdopted { id: RuleId::EarlyBirds },
        Event::Basked { devotion: 1.0 },
        Event::Reveried { serenity: 1.0 },
        Event::RiteBought { id: RiteId::MorningStretch },
        Event::Enlightened { tier: 1 },
        Event::ParadeHeld { rainbows: 1.0 },
        Event::ArchBought { id: ArchId::RibbonArch },
        Event::StarshardRefined,
        Event::EchoRefined,
        Event::JarCondensed,
        Event::PromiseMade,
        Event::PromiseQuieted { index: 0 },
        Event::PromiseResumed { index: 0 },
        Event::PrismDescended,
        Event::TradeReturned { colony: ColonyId::Otters, caravans: 1 },
        Event::PenPalSent { colony: ColonyId::Otters, level: 1 },
        Event::ColonyDiscovered { colony: ColonyId::Otters },
        Event::MeadowMoved { generation: 1 },
        Event::Passed { passed_down_wisdom: 1.0 },
        Event::TraditionBought { id: TraditionId::CarefulPacking },
        Event::AdventureDeclared { id: Some(AdventureId::QuietRetreat) },
        Event::AdventureCompleted { id: AdventureId::QuietRetreat },
        Event::SiteReached { site: SiteId::FenceTop },
        Event::WhaleSummoned,
        Event::WhaleFed { contentment: 1 },
        Event::Napped { years: 2, report: clowder_core::OfflineReport::default() },
        Event::DreamLoomBought { id: DreamLoomId::SoftPillow },
        Event::DejaVuDay,
        Event::DistantDawn,
    ]
}

/// Exhaustive by construction: a new event variant fails to compile here.
fn variant_name(e: &Event) -> &'static str {
    match e {
        Event::KittenArrived { .. } => "KittenArrived",
        Event::KittenWanderedOff { .. } => "KittenWanderedOff",
        Event::KittenReturned { .. } => "KittenReturned",
        Event::JobAssigned { .. } => "JobAssigned",
        Event::JobUnassigned { .. } => "JobUnassigned",
        Event::SeasonChanged { .. } => "SeasonChanged",
        Event::NewYear { .. } => "NewYear",
        Event::StructureBuilt { .. } => "StructureBuilt",
        Event::IdeaResearched { .. } => "IdeaResearched",
        Event::ZoomiesStarted { .. } => "ZoomiesStarted",
        Event::ZoomiesEnded => "ZoomiesEnded",
        Event::KittenMood { .. } => "KittenMood",
        Event::Flavor { .. } => "Flavor",
        Event::Revealed { .. } => "Revealed",
        Event::SkyEventVisible => "SkyEventVisible",
        Event::SpeedChanged { .. } => "SpeedChanged",
        Event::PounceReturned { .. } => "PounceReturned",
        Event::JamboreeStarted { .. } => "JamboreeStarted",
        Event::JamboreeEnded => "JamboreeEnded",
        Event::ElderAppointed { .. } => "ElderAppointed",
        Event::KittenPromoted { .. } => "KittenPromoted",
        Event::RefinementBought { .. } => "RefinementBought",
        Event::RuleAdopted { .. } => "RuleAdopted",
        Event::Basked { .. } => "Basked",
        Event::Reveried { .. } => "Reveried",
        Event::RiteBought { .. } => "RiteBought",
        Event::Enlightened { .. } => "Enlightened",
        Event::ParadeHeld { .. } => "ParadeHeld",
        Event::ArchBought { .. } => "ArchBought",
        Event::StarshardRefined => "StarshardRefined",
        Event::EchoRefined => "EchoRefined",
        Event::JarCondensed => "JarCondensed",
        Event::PromiseMade => "PromiseMade",
        Event::PromiseQuieted { .. } => "PromiseQuieted",
        Event::PromiseResumed { .. } => "PromiseResumed",
        Event::PrismDescended => "PrismDescended",
        Event::TradeReturned { .. } => "TradeReturned",
        Event::PenPalSent { .. } => "PenPalSent",
        Event::ColonyDiscovered { .. } => "ColonyDiscovered",
        Event::MeadowMoved { .. } => "MeadowMoved",
        Event::Passed { .. } => "Passed",
        Event::TraditionBought { .. } => "TraditionBought",
        Event::AdventureDeclared { .. } => "AdventureDeclared",
        Event::AdventureCompleted { .. } => "AdventureCompleted",
        Event::SiteReached { .. } => "SiteReached",
        Event::WhaleSummoned => "WhaleSummoned",
        Event::WhaleFed { .. } => "WhaleFed",
        Event::Napped { .. } => "Napped",
        Event::DreamLoomBought { .. } => "DreamLoomBought",
        Event::DejaVuDay => "DejaVuDay",
        Event::DistantDawn => "DistantDawn",
    }
}

/// A milestone drawn from the vocabulary itself — the journal's "never evict"
/// rule reads classification, so its tests must too.
fn a_milestone() -> Event {
    vocabulary()
        .into_iter()
        .find(|e| e.class() == EventClass::Milestone)
        .expect("the vocabulary classifies at least one event as a milestone")
}

/// An event the window is allowed to roll away.
fn a_non_milestone() -> Event {
    vocabulary()
        .into_iter()
        .find(|e| e.class() != EventClass::Milestone)
        .expect("the vocabulary classifies at least one event below milestone")
}

// =====================================================================
// Journal reading helpers
// =====================================================================

/// Everything the journal retains, without going through
/// `iter_chronological` (which has a test of its own).
fn all_entries(j: &Journal) -> Vec<JournalEntry> {
    j.kept.iter().chain(j.recent.iter()).cloned().collect()
}

/// Every occurrence the journal retained, as (tick, event). Away-reports are
/// a fold of an interval, not an occurrence, and belong to SAVE-015.
fn happened(j: &Journal) -> Vec<(u64, Event)> {
    all_entries(j)
        .into_iter()
        .filter_map(|e| match e.item {
            JournalItem::Happened(ev) => Some((e.tick, ev)),
            JournalItem::Away(_) => None,
        })
        .collect()
}

fn count_of(j: &Journal, target: &Event) -> usize {
    happened(j).iter().filter(|(_, e)| e == target).count()
}

/// A meadow with kittens, structures and food — a game with things happening
/// in it, so live play and catch-up both have events to produce.
fn played_state(seed: u64) -> GameState {
    let mut s = GameState::new(seed);
    grant(&mut s, StructureId::CatnipPatch, 4);
    grant(&mut s, StructureId::CardboardBox, 1);
    give(&mut s, ResourceId::Catnip, 200.0);
    adopt(&mut s, 2);
    s
}

// =====================================================================
// SIM-ENG-026 — every event carries a classification
// =====================================================================

// @spec SIM-ENG-026
#[test]
fn every_event_in_the_vocabulary_carries_a_classification() {
    let vocab = vocabulary();

    // The sample really is one of each variant — no variant counted twice,
    // none silently skipped (`variant_name` is exhaustive, so the build breaks
    // when the vocabulary grows and this list does not).
    let names: BTreeSet<&'static str> = vocab.iter().map(variant_name).collect();
    assert_eq!(names.len(), vocab.len(), "the sample holds each variant exactly once");

    // Classification is a total query on the event: every variant answers, and
    // the answer is a property of the event alone — no state, no caller.
    for e in &vocab {
        let class = e.class();
        assert_eq!(class, e.class(), "{} classifies the same way every time", variant_name(e));
        assert!(
            matches!(class, EventClass::Flavor | EventClass::Mechanical | EventClass::Milestone),
            "{} must classify as flavor, mechanical or milestone",
            variant_name(e)
        );
    }

    // Payload does not change prominence: two arrivals are equally prominent
    // whoever arrived. Prominence is about *what happened*, not its details.
    assert_eq!(
        Event::KittenArrived { name: "Pib".into() }.class(),
        Event::KittenArrived { name: "Marmalade".into() }.class(),
    );

    // A vocabulary that used only one class could not rank anything, which is
    // the whole reason the query exists.
    for class in [EventClass::Flavor, EventClass::Mechanical, EventClass::Milestone] {
        assert!(
            vocab.iter().any(|e| e.class() == class),
            "no event is classified {class:?} — the ranking would be useless"
        );
    }
}

// @spec SIM-ENG-026
#[test]
fn classification_ranks_prominence_the_way_the_game_rules_do() {
    // The design's own example of a rule a frontend must not be allowed to
    // decide: reaching a new site matters more than a job reassignment.
    assert_eq!(
        Event::SiteReached { site: SiteId::FenceTop }.class(),
        EventClass::Milestone,
        "reaching a new site is a moment worth remembering"
    );
    assert_eq!(
        Event::JobAssigned { name: "Pib".into(), job: JobId::Gardener }.class(),
        EventClass::Mechanical,
        "a job reassignment is an ordinary mechanical occurrence"
    );
    assert_eq!(
        Event::JobUnassigned { name: "Pib".into(), job: JobId::Gardener }.class(),
        EventClass::Mechanical,
    );

    // Flavor is defined by having no mechanical effect at all.
    assert_eq!(
        Event::Flavor { template: 0, kitten: None }.class(),
        EventClass::Flavor,
        "flavor events are pure delight, ranked quietest"
    );
    assert_eq!(
        Event::Flavor { template: 3, kitten: Some("Pib".into()) }.class(),
        EventClass::Flavor,
    );
}

// =====================================================================
// SIM-ENG-027 — the journal is game state, stamped and bounded
// =====================================================================

// @spec SIM-ENG-027
#[test]
fn live_play_records_what_happened_and_the_tick_it_happened_on() {
    let mut s = GameState::new(2_701);
    assert!(happened(&s.journal).is_empty(), "a new meadow has lived through nothing");

    // Day 25 is a season boundary: the change happens on the tick that carries
    // the counter into day 25.
    run_days(&mut s, 25);
    let entries = happened(&s.journal);
    assert!(!entries.is_empty(), "live ticks record what the player lived through");

    let ticks: Vec<u64> = entries.iter().map(|(t, _)| *t).collect();
    let mut sorted = ticks.clone();
    sorted.sort_unstable();
    assert_eq!(ticks, sorted, "entries are stamped in occurrence order");
    assert!(ticks.iter().all(|t| *t <= s.ticks), "no entry claims a tick that has not happened");

    let (tick, _) = entries
        .iter()
        .find(|(_, e)| matches!(e, Event::SeasonChanged { .. }))
        .expect("the season change of day 25 is in the journal");
    // The stamp exists so a reader can group by day: the season change must
    // group into day 25, not the last tick of day 24.
    assert_eq!(*tick, 25 * clowder_core::content::TICKS_PER_DAY);
    assert_eq!(*tick / clowder_core::content::TICKS_PER_DAY, 25);
}

// @spec SIM-ENG-027
#[test]
fn the_journal_is_canonical_state_and_survives_closing_the_terminal() {
    let mut s = played_state(2_702);
    run_days(&mut s, 40);
    let lived = happened(&s.journal);
    assert!(!lived.is_empty(), "there is history to preserve");

    // Same instant on load: nothing replays, so this is pure round-trip.
    let json = save_string(&s, 42_000);
    let loaded = load_string(&json, 42_000).expect("a saved game loads");
    assert_eq!(loaded.state.journal, s.journal, "the journal is part of the save");
    assert_eq!(happened(&loaded.state.journal), lived);
    assert_eq!(
        save_string(&loaded.state, 42_000),
        json,
        "the journal round-trips byte-identically with the rest of state"
    );
}

// @spec SIM-ENG-027
#[test]
fn the_window_is_bounded_while_every_milestone_is_kept_forever() {
    let plain = a_non_milestone();
    let milestone = a_milestone();
    let mut j = Journal::default();

    // Three milestones, spread across a long flood of ordinary events.
    j.record(0, &milestone);
    for t in 1..=20_000u64 {
        j.record(t, &plain);
    }
    let settled = j.recent.len();
    assert!(
        settled < 20_000,
        "the window is bounded, not a growing log (held {settled} of 20000)"
    );
    let oldest = j.recent.front().expect("a full window is not empty").tick;

    j.record(20_001, &milestone);
    for t in 20_002..=40_000u64 {
        j.record(t, &plain);
    }
    j.record(40_001, &milestone);

    assert_eq!(j.recent.len(), settled, "a full window stops growing as events keep arriving");
    assert!(
        j.recent.front().expect("still not empty").tick > oldest,
        "the window rolled forward rather than freezing"
    );
    assert_eq!(
        count_of(&j, &milestone),
        3,
        "every milestone is retained, exactly once, however much happened after it"
    );
}

// @spec SIM-ENG-027
#[test]
fn the_journal_reads_back_in_occurrence_order() {
    let plain = a_non_milestone();
    let milestone = a_milestone();
    let mut j = Journal::default();
    j.record(1, &plain);
    j.record(2, &milestone);
    j.record(3, &plain);
    j.record(30, &milestone);
    j.record(31, &plain);

    let merged = j.iter_chronological();
    assert_eq!(merged.len(), 5, "kept entries and the window are both there, none twice");
    let ticks: Vec<u64> = merged.iter().map(|e| e.tick).collect();
    assert_eq!(ticks, vec![1, 2, 3, 30, 31], "merged in occurrence order, not kept-then-recent");
}

// =====================================================================
// SIM-ENG-029 — a full window evicts its oldest non-milestone entry
// =====================================================================

// @spec SIM-ENG-029
#[test]
fn a_full_window_evicts_its_oldest_non_milestone_entry() {
    let plain = a_non_milestone();
    let mut j = Journal::default();
    for t in 1..=30_000u64 {
        j.record(t, &plain);
    }
    let cap = j.recent.len() as u64;
    assert!(cap < 30_000, "the window filled and started evicting");

    // Oldest-first: the survivors are exactly the most recent window-full.
    let ticks: Vec<u64> = j.recent.iter().map(|e| e.tick).collect();
    let expected: Vec<u64> = (30_000 - cap + 1..=30_000).collect();
    assert_eq!(ticks, expected, "eviction takes the oldest entry, never a newer one");
}

// @spec SIM-ENG-029
#[test]
fn eviction_never_reaches_a_milestone() {
    let plain = a_non_milestone();
    let milestone = a_milestone();
    let mut j = Journal::default();

    // A milestone at the very start — the first thing an oldest-first eviction
    // would reach if classification were not what protects it.
    j.record(1, &milestone);
    j.record(2, &plain);
    for t in 3..=50_000u64 {
        j.record(t, &plain);
    }

    assert_eq!(count_of(&j, &milestone), 1, "the oldest entry of all survived 50000 events");
    let survivors = happened(&j);
    assert!(
        !survivors.iter().any(|(t, _)| *t == 2),
        "the oldest non-milestone entry is what got dropped"
    );
    assert!(
        survivors.len() < 50_000,
        "growth stayed bounded even with a milestone pinned at the front"
    );
}

// =====================================================================
// SIM-ENG-028 — live ticks and applied commands append
// =====================================================================

// @spec SIM-ENG-028
#[test]
fn an_applied_command_journals_and_a_rejected_one_journals_nothing() {
    let mut s = played_state(2_803);
    let before = happened(&s.journal).len();

    let events = s.apply(Command::AssignJob(JobId::Gardener)).expect("an idle kitten takes a job");
    assert!(!events.is_empty(), "the command reported what happened");
    let after = happened(&s.journal);
    assert_eq!(after.len(), before + events.len(), "exactly the command's events were appended");
    assert!(
        after.iter().any(|(t, e)| *t == s.ticks && matches!(e, Event::JobAssigned { .. })),
        "the assignment is in the journal, stamped with the tick it was applied at"
    );

    // A rejected command produced no events and mutated nothing — the journal
    // is state, so it does not move either.
    let settled = happened(&s.journal);
    assert!(s.apply(Command::DeepNap(1)).is_err(), "the nap is locked in this meadow");
    assert_eq!(happened(&s.journal), settled, "a rejected command journals nothing");
}

// @spec SIM-ENG-028
#[test]
fn catch_up_replay_never_appends_its_constituent_events() {
    let mut s = played_state(2_804);
    run_days(&mut s, 30);
    let lived = happened(&s.journal);
    assert!(!lived.is_empty(), "live play journals — this is the contrast the spec draws");
    let kept_before = s.journal.kept.len();

    // An hour away is 14400 ticks — 1440 game days, dozens of seasons. If
    // replay appended, thousands of entries would arrive here, and the
    // permanent (never-evicted) list would grow without bound, which is the
    // cost the rule exists to prevent.
    let json = save_string(&s, 1_000);
    let loaded = load_string(&json, 1_000 + 3_600).expect("an hour-old save loads");
    assert_eq!(loaded.state.ticks, s.ticks + 14_400, "a real catch-up ran");
    assert!(
        loaded.report.seasons_passed >= 50,
        "the replay really did produce events to be discarded (seasons: {})",
        loaded.report.seasons_passed
    );

    assert_eq!(
        happened(&loaded.state.journal),
        lived,
        "not one replayed event entered the journal"
    );
    let kept_after = loaded
        .state
        .journal
        .kept
        .iter()
        .filter(|e| matches!(e.item, JournalItem::Happened(_)))
        .count();
    assert_eq!(
        kept_after, kept_before,
        "the permanent list — the unbounded one — did not grow across a long absence"
    );
}

// =====================================================================
// SIM-ENG-040 — the core decides live-versus-replay, not the caller
// =====================================================================

// @spec SIM-ENG-040
#[test]
fn the_core_decides_live_versus_replay_not_the_caller() {
    // Two callers, each doing the least possible: one drives ticks, the other
    // loads a save. Neither passes a flag, neither opts in or out. The core
    // journals the first and not the second — a tick cannot tell which it is,
    // so the decision has to have been made inside.
    let base = played_state(2_840);
    let ticks_away = 14_400u64; // 3600 s at the default speed

    let mut live = base.clone();
    run_ticks(&mut live, ticks_away);
    assert!(
        !happened(&live.journal).is_empty(),
        "a frontend that asked for nothing still gets its history"
    );

    let json = save_string(&base, 5_000);
    let caught_up = load_string(&json, 5_000 + ticks_away / 4).expect("the save loads").state;
    assert_eq!(caught_up.ticks, live.ticks, "both paths advanced the same number of ticks");
    assert_eq!(
        happened(&caught_up.journal),
        happened(&base.journal),
        "the replay entry point journals nothing, without the caller having to know"
    );

    // Same simulation, same seed, same ticks: the two states differ in the
    // journal and in nothing else. That difference is the rule, made in here.
    let mut live_bare = live.clone();
    let mut caught_up_bare = caught_up.clone();
    live_bare.journal = Journal::default();
    caught_up_bare.journal = Journal::default();
    assert_eq!(
        save_string(&live_bare, 0),
        save_string(&caught_up_bare, 0),
        "live play and catch-up run the identical simulation — only the journal differs"
    );
}

// @spec SIM-ENG-040
#[test]
fn a_replay_inside_a_command_does_not_stream_into_the_journal() {
    // The Deep Nap fast-forwards years of ticks from inside command
    // application. The caller passes a year count, nothing about journaling —
    // if journaling lived in the ordinary tick, a nap would pour its whole
    // per-tick stream into the save. It takes the replay entry point instead,
    // and only its folded report is recorded.
    let mut s = GameState::new(2_841);
    s.ideas.insert(IdeaId::DeepNapTheory);
    give(&mut s, ResourceId::Starshards, 1e12);
    let before = happened(&s.journal).len();

    let years = 10u32;
    assert!(years <= s.max_nap_length());
    let ticks_before = s.ticks;
    let events = s.apply(Command::DeepNap(years)).expect("the nap is affordable");
    assert_eq!(s.ticks, ticks_before + years as u64 * 1_000, "years of ticks really replayed");

    // The summary IS recorded — as the nap's away entry, which is the one
    // place a bulk-time span is read from (SIM-DREAM-008). Journaling the
    // Napped event as well would file one sleep under two headings.
    let aways = s
        .journal
        .iter_chronological()
        .into_iter()
        .filter(|e| matches!(&e.item, JournalItem::Away(_)))
        .count();
    assert_eq!(aways, 1, "the nap left exactly one away entry — its summary");
    assert!(
        events.iter().any(|e| matches!(e, Event::Napped { .. })),
        "and the caller still receives the summary event for the wake-up screen"
    );

    let after = happened(&s.journal);
    assert_eq!(
        after.len(),
        before,
        "and nothing else: none of the {} replayed ticks reached the journal",
        years as u64 * 1_000
    );
}
