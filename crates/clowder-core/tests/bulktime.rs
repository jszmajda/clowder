//! Bulk time: what comes back when the meadow advances all at once.
//!
//! Two ways time passes in bulk — a player's Deep Nap and a player's absence —
//! and SIM-DREAM-008 says they are one mechanism: a report folded from the
//! replay and retained in the journal, so a caller has one place to read it.
//! SIM-DREAM-001 says what a nap hands back is that summarized report —
//! elapsed span and net deltas — never a bare count and never a per-tick
//! event stream.
//!
//! ---- What these tests deliberately do NOT pin ----
//!
//! The *layout* of the event that carries the summary. SIM-DREAM-001 and
//! SIM-DREAM-008 constrain what a nap delivers (the folded report, the same
//! kind offline absence produces) and where it is kept (retained in the
//! journal), not which field of which variant holds it. So the assertions
//! reach the returned report through the event's serialized form and
//! deserialize it as the `OfflineReport` the spec says it is — any layout
//! that actually carries the report satisfies them, and a bare year count
//! satisfies none of them.
//!
//! Two things the specs leave genuinely open are called out at their use
//! sites and excluded from assertions rather than silently decided:
//!   1. whether the starshard cost is inside the fold (the spend happens
//!      before the replay; SIM-DREAM-008 says the report is folded *from the
//!      replay*, SIM-DREAM-001 says it carries "net deltas");
//!   2. what unit the elapsed span is carried in (SIM-DREAM-001 says
//!      "elapsed years"; the report type offline absence uses carries
//!      `elapsed_seconds`).

mod common;

use clowder_core::*;
use common::*;

const TOL: f64 = 1e-6;

/// A meadow that has been played a little and can afford to nap: producers
/// running, kittens home, the Deep Nap Theory known, starshards in store.
///
/// Two properties the tests below lean on. Spare cushions mean a few years of
/// meadow really does produce arrivals, wanderings and homecomings, so a fold
/// of that span exercises every list in the report rather than a row of empty
/// ones. And every stock sits inside its cap, so a save/load round-trip of
/// this state normalizes nothing — which is what lets an absence and a nap be
/// compared as the same simulation.
fn napping_meadow(seed: u64) -> GameState {
    let mut s = GameState::new(seed);
    give(&mut s, ResourceId::Catnip, 80.0);
    grant(&mut s, StructureId::CatnipPatch, 3);
    grant(&mut s, StructureId::CardboardBox, 1);
    adopt(&mut s, 2);
    grant(&mut s, StructureId::CozyCushion, 8);
    s.apply(Command::AssignJob(JobId::Gardener)).unwrap();
    s.ideas.insert(IdeaId::DeepNapTheory);
    give(&mut s, ResourceId::Starshards, 1e12);
    s
}

/// The report a nap handed back, read out of the command's return value.
///
/// Shape-agnostic on purpose (see the module header): the returned event is
/// serialized and the tree searched for a value that *is* an `OfflineReport`
/// — the same report type an absence produces. A summary that carries only a
/// year count contains no such value, which is exactly the failure this
/// helper is meant to report.
fn nap_summary(events: &[Event]) -> OfflineReport {
    let napped = events
        .iter()
        .find(|e| matches!(e, Event::Napped { .. }))
        .expect("a nap comes back as a summarized nap report");
    let value = serde_json::to_value(napped).expect("events serialize");
    find_report(&value).unwrap_or_else(|| {
        panic!(
            "SIM-DREAM-001: a nap must return the summarized report (elapsed span + net \
             deltas), the same kind of report an absence folds — not a bare count. Got: {value}"
        )
    })
}

/// Depth-first search for a sub-value that deserializes as an `OfflineReport`.
fn find_report(v: &serde_json::Value) -> Option<OfflineReport> {
    if let Ok(report) = serde_json::from_value::<OfflineReport>(v.clone()) {
        return Some(report);
    }
    match v {
        serde_json::Value::Object(map) => map.values().find_map(find_report),
        serde_json::Value::Array(items) => items.iter().find_map(find_report),
        _ => None,
    }
}

/// Every bulk-time report the journal is holding, in occurrence order.
fn away_reports(journal: &Journal) -> Vec<OfflineReport> {
    journal
        .iter_chronological()
        .into_iter()
        .filter_map(|entry| match &entry.item {
            JournalItem::Away(report) => Some(report.clone()),
            _ => None,
        })
        .collect()
}

/// The absence that replays exactly as many ticks as a `years`-long nap:
/// catch-up replays elapsed × TICKS_PER_SECOND × speed, and a game-year is
/// 1000 ticks.
fn absence_matching(years: u32, speed: f64) -> u64 {
    assert_eq!(speed, 1.0, "the equivalence is set up at ordinary speed");
    years as u64 * 1000 / content::TICKS_PER_SECOND
}

// =====================================================================
// SIM-DREAM-001 — a nap returns a summarized report, not a count
// =====================================================================

/// The whole point of the summary: it says what the meadow *gained* while the
/// clowder dreamed. A count of years cannot answer that, and the player has no
/// other way to find out — the per-tick stream is deliberately not retained.
// @spec SIM-DREAM-001
#[test]
fn a_nap_reports_the_net_deltas_the_meadow_actually_moved_by() {
    let years = 3u32;
    let mut s = napping_meadow(700);
    assert!(years <= s.max_nap_length());

    let before = s.resources;
    let events = s
        .apply(Command::DeepNap(years))
        .expect("the nap is affordable");
    let report = nap_summary(&events);

    let mut moved = 0;
    for r in ResourceId::ALL {
        // Starshards excluded: the nap spends its cost before the replay
        // begins, and whether that spend is inside the fold is an open
        // question on these specs, not something a test may decide.
        if r == ResourceId::Starshards {
            continue;
        }
        let observed = s.resources[r.ix()] - before[r.ix()];
        assert!(
            (report.resource_deltas[r.ix()] - observed).abs() < TOL,
            "the summary's {r:?} delta ({}) must be what the meadow actually moved by ({observed})",
            report.resource_deltas[r.ix()],
        );
        if observed.abs() > TOL {
            moved += 1;
        }
    }
    assert!(
        moved > 0,
        "three game-years must move something, or the check above proves nothing"
    );
}

/// The other half of the summary: how much time went by. A report of deltas
/// with no span cannot be read — the same gain over three years and over one
/// is a different story.
// @spec SIM-DREAM-001
#[test]
fn a_nap_reports_its_own_elapsed_span() {
    let mut one = napping_meadow(701);
    let mut two = one.clone();

    let short = nap_summary(&one.apply(Command::DeepNap(1)).expect("affordable"));
    let long = nap_summary(&two.apply(Command::DeepNap(2)).expect("affordable"));

    assert!(
        short.elapsed_seconds > 0,
        "the summary reports how much time passed"
    );
    // Unit-agnostic (see the module header's open question 2): whatever the
    // span is carried in, twice the nap is twice the span.
    assert_eq!(
        long.elapsed_seconds,
        2 * short.elapsed_seconds,
        "the reported span is the nap's own length, so it scales with the nap"
    );
    assert!(
        !short.truncated && !long.truncated,
        "a nap is bounded by max_nap_length, so it never runs into the catch-up cap"
    );
}

/// "Summarized" is load-bearing, not decorative: a nap is thousands of ticks,
/// and handing the caller (or the journal) that stream back would be the same
/// mistake offline catch-up already refuses to make.
// @spec SIM-DREAM-001
#[test]
fn a_long_nap_comes_back_as_one_summary_not_a_per_tick_stream() {
    let years = 20u32;
    let mut s = napping_meadow(702);
    assert!(years <= s.max_nap_length());
    let entries_before = s.journal.iter_chronological().len();

    let events = s
        .apply(Command::DeepNap(years))
        .expect("the nap is affordable");

    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, Event::Napped { .. }))
            .count(),
        1,
        "one nap, one summary"
    );
    assert!(
        events.len() < 50,
        "{} game-years replayed {} ticks; the caller gets a summary, not {} events",
        years,
        years as u64 * 1000,
        events.len()
    );
    let added = s.journal.iter_chronological().len() - entries_before;
    assert!(
        added < 50,
        "and the journal keeps the summary, not the {added} entries of a stream"
    );

    // Summarized, not merely small: the one event standing in for twenty
    // thousand ticks has to carry the account of them.
    let report = nap_summary(&events);
    assert!(
        report.elapsed_seconds > 0,
        "the one summary spans the whole nap"
    );
}

// =====================================================================
// SIM-DREAM-008 — one mechanism for every way time passes in bulk
// =====================================================================

/// The spec's own words: the summary comes back *by the same route offline
/// absence uses* — folded from the replay, retained in the journal. So what
/// the command returns and what the journal keeps are one report, not two
/// parallel summaries a caller has to reconcile.
// @spec SIM-DREAM-008
#[test]
fn the_summary_a_nap_returns_is_the_report_the_journal_retains() {
    let mut s = napping_meadow(703);
    let returned = nap_summary(&s.apply(Command::DeepNap(3)).expect("affordable"));

    let retained = away_reports(&s.journal);
    assert_eq!(
        retained.len(),
        1,
        "one bulk-time skip leaves one report in the journal"
    );
    assert_eq!(
        retained[0], returned,
        "one mechanism, one place to read it — the retained report IS the returned one"
    );
    assert!(
        s.journal
            .kept
            .iter()
            .any(|e| matches!(e.item, JournalItem::Away(_))),
        "retained means retained: a bulk-time report is never left in the evictable window"
    );
}

/// One mechanism means the fold is the same fold. A nap and an absence that
/// advance the identical meadow by the identical number of ticks must produce
/// the same account of it — otherwise there are two mechanisms wearing one
/// name, and the player learns different things depending on how time passed.
// @spec SIM-DREAM-008
#[test]
fn a_nap_and_an_absence_of_the_same_length_fold_the_same_report() {
    let years = 3u32;
    let base = napping_meadow(704);

    let away = load_string(&save_string(&base, 0), absence_matching(years, base.speed))
        .expect("the save round-trips")
        .report;

    // The absence has to have something to tell, or matching it proves nothing.
    assert!(away.seasons_passed > 0, "three game-years turn the seasons");
    assert!(
        !away.arrived.is_empty() && !away.wandered.is_empty() && !away.returned.is_empty(),
        "three game-years of this meadow move kittens in and out — every list is exercised"
    );

    let mut napper = base.clone();
    let napped = nap_summary(&napper.apply(Command::DeepNap(years)).expect("affordable"));

    assert_eq!(
        napped.seasons_passed, away.seasons_passed,
        "the same seasons went by"
    );
    assert_eq!(
        napped.arrived, away.arrived,
        "the same kittens arrived, folded the same way"
    );
    assert_eq!(
        napped.wandered, away.wandered,
        "the same kittens wandered off"
    );
    assert_eq!(napped.returned, away.returned, "the same kittens came home");
    assert!(
        !napped.truncated && !away.truncated,
        "neither run hit the catch-up cap"
    );
    for r in ResourceId::ALL {
        // Starshards excluded: only the nap pays a fare. See the module header.
        if r == ResourceId::Starshards {
            continue;
        }
        assert!(
            (napped.resource_deltas[r.ix()] - away.resource_deltas[r.ix()]).abs() < TOL,
            "{r:?}: a nap folds {} where the same absence folds {}",
            napped.resource_deltas[r.ix()],
            away.resource_deltas[r.ix()],
        );
    }
}

/// Retention is why the journal is the route at all: a report the player
/// dismissed unread, or never saw because the game closed, is still there.
/// A nap is a deliberate skip — the shortest one is still a year of meadow,
/// never the blink that SAVE-015's minimum exists to ignore.
// @spec SIM-DREAM-008
#[test]
fn a_naps_report_survives_being_dismissed_unread() {
    let mut s = napping_meadow(705);
    let _dismissed = s
        .apply(Command::DeepNap(1))
        .expect("the shortest nap is affordable");

    assert_eq!(
        away_reports(&s.journal).len(),
        1,
        "even a one-year nap is a bulk-time skip worth remembering"
    );

    let reloaded = load_string(&save_string(&s, 0), 0)
        .expect("the save round-trips")
        .state;
    assert_eq!(
        away_reports(&reloaded.journal),
        away_reports(&s.journal),
        "the report lives in the save, not only in the value the caller threw away"
    );
}
