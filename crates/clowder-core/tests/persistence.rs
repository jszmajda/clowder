mod common;

use clowder_core::*;
use common::*;
use std::fs;
use std::path::PathBuf;

fn scratch_dir(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("clowder-test-{}-{}", std::process::id(), test));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn played_state(seed: u64) -> GameState {
    let mut s = GameState::new(seed);
    give(&mut s, ResourceId::Catnip, 80.0);
    grant(&mut s, StructureId::CatnipPatch, 3);
    grant(&mut s, StructureId::CardboardBox, 1);
    adopt(&mut s, 2);
    s.apply(Command::AssignJob(JobId::Gardener)).unwrap();
    s
}

// @spec SAVE-001
#[test]
fn saves_are_versioned_pretty_json() {
    let s = played_state(1);
    let json = save_string(&s, 123_456);
    assert!(json.contains('\n'), "pretty-printed for hand-rescue");
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["save_version"], SAVE_VERSION);
    assert_eq!(v["saved_at_unix"], 123_456);
    assert!(v["state"].is_object());
}

// @spec SAVE-002
#[test]
fn newer_saves_are_refused_with_the_version_named() {
    let s = played_state(2);
    let json = save_string(&s, 1000);
    let mut v: serde_json::Value = serde_json::from_str(&json).unwrap();
    v["save_version"] = serde_json::json!(99);
    let doctored = serde_json::to_string(&v).unwrap();
    match load_string(&doctored, 1000) {
        Err(LoadError::TooNew { found }) => assert_eq!(found, 99),
        other => panic!("expected TooNew, got {other:?}"),
    }
}

// @spec SAVE-003
#[test]
fn writes_are_atomic_and_leftover_tmp_files_are_harmless() {
    let dir = scratch_dir("atomic");
    let path = dir.join("save.json");
    let s = played_state(3);

    // A stale tmp from a "crashed" earlier write must not confuse anything.
    fs::write(dir.join("save.json.tmp"), b"{ definitely not json").unwrap();
    write_save(&path, &s, 5000).unwrap();
    assert!(path.exists());
    assert!(!dir.join("save.json.tmp").exists(), "tmp is renamed away on success");

    fs::write(dir.join("save.json.tmp"), b"{ garbage again").unwrap();
    let loaded = read_save(&path, 5000).unwrap();
    assert_eq!(save_string(&loaded.state, 5000), save_string(&s, 5000), "load ignores the tmp");
}

// @spec SAVE-004
#[test]
fn load_errors_are_typed_and_never_touch_the_file() {
    let dir = scratch_dir("errors");
    assert!(matches!(read_save(&dir.join("save.json"), 0), Err(LoadError::NotFound)));

    let corrupt_path = dir.join("save.json");
    fs::write(&corrupt_path, b"not json at all").unwrap();
    assert!(matches!(read_save(&corrupt_path, 0), Err(LoadError::Corrupt { .. })));
    assert_eq!(
        fs::read(&corrupt_path).unwrap(),
        b"not json at all",
        "core never rewrites a save on load"
    );

    // Unknown content ids are Corrupt, not guessed at.
    let s = played_state(4);
    let json = save_string(&s, 1000);
    let doctored = json.replace("\"CatnipPatch\"", "\"OrbitalLaserPatch\"");
    // (If the id doesn't appear as a key, doctor the ideas list instead.)
    let target = if doctored != json {
        doctored
    } else {
        json.replace("\"ideas\": []", "\"ideas\": [\"NonsenseIdea\"]")
    };
    assert!(matches!(load_string(&target, 1000), Err(LoadError::Corrupt { .. })));
}

// @spec SAVE-005
#[test]
fn hand_edited_numbers_get_physics_not_errors() {
    let s = played_state(5);
    let json = save_string(&s, 1000);
    let mut v: serde_json::Value = serde_json::from_str(&json).unwrap();
    v["state"]["resources"][0] = serde_json::json!(999_999.0); // way over cap
    v["state"]["pity"] = serde_json::json!(300); // beyond u8
    let doctored = serde_json::to_string(&v).unwrap();
    let loaded = load_string(&doctored, 1000).unwrap();
    let cap = loaded.state.cap(ResourceId::Catnip);
    assert!(
        loaded.state.stock(ResourceId::Catnip) <= cap,
        "over-cap stock clamps on load"
    );
    assert_eq!(loaded.state.pity, 255, "wide-deserialized counters clamp to range");
}

// @spec SAVE-006
#[test]
fn catch_up_replays_the_real_tick_function() {
    let s = played_state(6);
    let saved_at = 10_000u64;
    let json = save_string(&s, saved_at);

    // 10 minutes away = 2400 ticks.
    let loaded = load_string(&json, saved_at + 600).unwrap();
    assert_eq!(loaded.state.ticks, s.ticks + 2400);

    // The replay must be *the same code path* as live play. It is no longer
    // byte-identical, and the difference is the point: live ticks journal and
    // replay does not (SIM-ENG-028). Assert that difference explicitly, then
    // normalize it away and require everything else to match exactly — which
    // is a stronger check than the byte comparison alone ever was.
    let mut manual = s.clone();
    for _ in 0..2400 {
        manual.tick();
    }

    let occurrences = |g: &GameState| -> usize {
        g.journal
            .iter_chronological()
            .into_iter()
            .filter(|e| matches!(&e.item, JournalItem::Happened(_)))
            .count()
    };
    // The save already carries whatever the pre-save commands journaled; those
    // ride through the save and are not replay's doing. What replay must add is
    // nothing at all.
    let inherited = occurrences(&s);
    assert_eq!(
        occurrences(&loaded.state),
        inherited,
        "catch-up replay must add no occurrences to the journal (had {inherited} before the save)"
    );
    assert!(
        manual.journal.iter_chronological().iter().any(|e| matches!(&e.item, JournalItem::Happened(_))),
        "live ticks over the same span must journal, or this test proves nothing"
    );

    let mut normalized = loaded.state.clone();
    normalized.journal = manual.journal.clone();
    assert_eq!(save_string(&normalized, 0), save_string(&manual, 0));
    assert_eq!(loaded.report.elapsed_seconds, 600);
}

// @spec SAVE-007
#[test]
fn backwards_clocks_load_safely_with_zero_ticks() {
    let s = played_state(7);
    let json = save_string(&s, 50_000);
    let loaded = load_string(&json, 10_000).unwrap();
    assert_eq!(save_string(&loaded.state, 0), save_string(&s, 0), "state never regresses");
    assert_eq!(loaded.report.elapsed_seconds, 0);
}

// @spec SAVE-008
#[test]
fn the_offline_report_tells_you_what_you_missed() {
    // A famine-bound clowder: 3 kittens, no food, no production.
    let mut s = GameState::new(8);
    adopt(&mut s, 3);
    for k in &mut s.kittens {
        k.job = None;
    }
    give(&mut s, ResourceId::Catnip, 0.0);
    let json = save_string(&s, 100_000);

    // 75 real seconds = 300 ticks = 30 game days: wanders + a season boundary.
    let loaded = load_string(&json, 100_075).unwrap();
    let report = &loaded.report;
    assert_eq!(report.elapsed_seconds, 75);
    assert!(!report.truncated);
    assert!(report.seasons_passed >= 1);
    let wandered_total: u32 = report.wandered.iter().map(|(_, n)| n).sum();
    assert_eq!(wandered_total, 2, "two kittens wandered; the last stayed home");
    assert!(report.arrived.is_empty());
    // Net deltas cover every resource; catnip net is ~zero (was zero, stays zero).
    assert!(report.resource_deltas[ResourceId::Catnip.ix()].abs() < 1e-6);
}

// @spec SAVE-006
#[test]
#[ignore = "replays 365 days of ticks; run with --release --ignored"]
fn absurdly_long_absences_are_capped_and_flagged() {
    let s = GameState::new(9);
    let json = save_string(&s, 0);
    let year = 365u64 * 86_400;
    let loaded = load_string(&json, year + 86_400).unwrap(); // 366 days
    assert!(loaded.report.truncated);
    assert_eq!(loaded.state.ticks, year * 4, "capped at 365 days of ticks");
}

// @spec SAVE-009
#[test]
#[ignore = "perf bound is a release-build property; run with --release --ignored"]
fn thirty_day_catch_up_feels_like_loading_not_compiling() {
    let mut s = GameState::new(10);
    grant(&mut s, StructureId::CatnipPatch, 5);
    grant(&mut s, StructureId::CozyCushion, 8);
    give(&mut s, ResourceId::Catnip, 200.0);
    let json = save_string(&s, 0);
    let start = std::time::Instant::now();
    let loaded = load_string(&json, 30 * 86_400).unwrap();
    let took = start.elapsed();
    assert_eq!(loaded.state.ticks, 30 * 86_400 * 4);
    assert!(took.as_secs_f64() < 5.0, "30-day catch-up took {took:?}");
}

// @spec SAVE-010
#[test]
fn round_trips_are_bit_identical() {
    let s = played_state(11);
    let json = save_string(&s, 42_000);
    let loaded = load_string(&json, 42_000).unwrap();
    let again = save_string(&loaded.state, 42_000);
    assert_eq!(json, again, "save → load → save is byte-stable");
}

// @spec SAVE-011
#[test]
fn only_canonical_state_is_persisted() {
    let s = played_state(12);
    let json = save_string(&s, 1);
    assert!(!json.contains("happiness"), "derived values are recomputed, not stored");
    assert!(!json.contains("mood"));
    assert!(!json.contains("event"));
}

// @spec SAVE-012
#[test]
fn the_default_path_is_the_platform_data_dir() {
    let path = default_save_path().expect("platform data dir resolves");
    let tail: PathBuf = path.iter().rev().take(2).collect::<Vec<_>>().into_iter().rev().collect();
    assert_eq!(tail, PathBuf::from("clowder/save.json"));
}

// ---- Depth Wave 1 ----

// @spec SAVE-013
#[test]
fn v1_saves_migrate_forward_losslessly() {
    let fixture = include_str!("fixtures/save_v1.json");
    let loaded = load_string(fixture, 1_751_700_000).expect("v1 fixture loads");
    let s = &loaded.state;
    assert_eq!(s.ticks, 45, "old meadow preserved exactly");
    assert_eq!(s.kittens.len(), 2);
    assert_eq!(s.structures[StructureId::CatnipPatch.ix()], 1);
    assert_eq!(s.speed, 1.0, "new fields get new-game defaults");
    // Re-saving stamps the current version — whatever it currently is, which
    // is the property under test. A literal here just breaks on every bump.
    let resaved = save_string(s, 1_751_700_000);
    let v: serde_json::Value = serde_json::from_str(&resaved).unwrap();
    assert_eq!(v["save_version"], clowder_core::SAVE_VERSION);
}

// @spec SAVE-006
#[test]
fn catch_up_scales_with_persisted_game_speed() {
    let mut s = GameState::new(40);
    grant(&mut s, StructureId::CatnipPatch, 2);
    s.apply(Command::SetSpeed(2.0)).unwrap();
    let json = save_string(&s, 10_000);
    let loaded = load_string(&json, 10_060).unwrap(); // 60s away at 2×
    assert_eq!(loaded.state.ticks, s.ticks + 60 * 4 * 2);
}

// =====================================================================
// Save format v3 — the journal becomes persisted state
// =====================================================================

const TOL: f64 = 1e-9;

/// The committed v2 fixture. Read at runtime rather than through
/// `include_str!` on purpose: SAVE-014 makes the *committed file* part of the
/// requirement, and a missing fixture should read as a failing test with a
/// message, not as a build error in an unrelated test binary.
const V2_FIXTURE_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/save_v2.json");

fn v2_fixture() -> String {
    fs::read_to_string(V2_FIXTURE_PATH).unwrap_or_else(|e| {
        panic!("SAVE-014 requires a committed v2 fixture at {V2_FIXTURE_PATH}: {e}")
    })
}

/// Numeric-aware JSON equality. A hand-written fixture may spell a float
/// `0` where the serializer spells it `0.0`; that is the same meadow, and a
/// migration-fidelity test should not fail on the spelling. Integers still
/// compare exactly, so a differing RNG word is still caught.
fn json_eq(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    use serde_json::Value as V;
    match (a, b) {
        (V::Number(x), V::Number(y)) => {
            if let (Some(i), Some(j)) = (x.as_u64(), y.as_u64()) {
                return i == j;
            }
            if let (Some(i), Some(j)) = (x.as_i64(), y.as_i64()) {
                return i == j;
            }
            match (x.as_f64(), y.as_f64()) {
                (Some(i), Some(j)) => i == j,
                _ => x == y,
            }
        }
        (V::Array(x), V::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y.iter()).all(|(p, q)| json_eq(p, q))
        }
        (V::Object(x), V::Object(y)) => {
            x.len() == y.len()
                && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| json_eq(v, w)))
        }
        _ => a == b,
    }
}

/// A meadow with lived history: kittens, structures, food, and long enough
/// that the journal holds both rolling entries and kept milestones.
fn lived_state(seed: u64) -> GameState {
    let mut s = GameState::new(seed);
    grant(&mut s, StructureId::CatnipPatch, 4);
    grant(&mut s, StructureId::CardboardBox, 1);
    give(&mut s, ResourceId::Catnip, 200.0);
    adopt(&mut s, 2);
    run_days(&mut s, 300);
    s
}

/// Every away-report the journal retained, in reading order.
fn away_entries(j: &Journal) -> Vec<(u64, OfflineReport)> {
    j.iter_chronological()
        .into_iter()
        .filter_map(|e| match &e.item {
            JournalItem::Away(r) => Some((e.tick, r.clone())),
            JournalItem::Happened(_) => None,
        })
        .collect()
}

/// Every occurrence the journal retained — the away-reports removed, so a
/// test can assert an absence left the lived history alone.
fn occurrences(j: &Journal) -> Vec<(u64, Event)> {
    j.iter_chronological()
        .into_iter()
        .filter_map(|e| match &e.item {
            JournalItem::Happened(ev) => Some((e.tick, ev.clone())),
            JournalItem::Away(_) => None,
        })
        .collect()
}

/// An event the rolling window is allowed to evict.
fn rolling_event() -> Event {
    let e = Event::Flavor { template: 0, kitten: None };
    assert_ne!(e.class(), EventClass::Milestone, "this helper must drive the rolling window");
    e
}

// @spec SAVE-011
#[test]
fn derived_values_are_recomputed_after_load_but_the_journal_cannot_be() {
    let s = lived_state(3_101);
    assert!(!s.journal.iter_chronological().is_empty(), "there is lived history to keep");

    let json = save_string(&s, 60_000);
    let whole: serde_json::Value = serde_json::from_str(&json).unwrap();
    let keys: Vec<&String> = whole["state"].as_object().unwrap().keys().collect();
    for derived in ["happiness", "mood", "mood_tier", "season", "rate", "rates", "net_rates"] {
        assert!(
            !keys.iter().any(|k| k.as_str() == derived),
            "`{derived}` is derived and must not be stored; state keys: {keys:?}"
        );
    }
    assert!(
        keys.iter().any(|k| k.as_str() == "journal"),
        "the journal is canonical state and is stored"
    );

    // Strip the journal out of an otherwise identical save. Everything derived
    // comes back on its own — which is exactly why it is not persisted — and
    // the history does not, which is exactly why the journal must be.
    let mut doctored = whole.clone();
    doctored["state"].as_object_mut().unwrap().remove("journal");
    let stripped = load_string(&serde_json::to_string(&doctored).unwrap(), 60_000)
        .expect("a save without a journal still loads")
        .state;

    assert_eq!(stripped.season(), s.season(), "season recomputes from state");
    assert_eq!(stripped.mood_tier(), s.mood_tier(), "mood recomputes from state");
    assert!((stripped.happiness() - s.happiness()).abs() < TOL, "happiness recomputes");
    assert_eq!(stripped.net_rates(), s.net_rates(), "rates recompute");
    assert_eq!(
        stripped.journal,
        Journal::default(),
        "nothing in the game can recompute what the player lived through"
    );

    let kept = load_string(&json, 60_000).expect("the real save loads").state;
    assert_eq!(kept.journal, s.journal, "so the save carries it verbatim");
}

// @spec SAVE-010, SAVE-011
#[test]
fn a_heavy_journal_does_not_perturb_the_bit_identical_round_trip() {
    // A journal carrying all three shapes at once: kept milestones, an
    // away-report, and a rolling window driven past capacity so eviction has
    // already happened.
    let staged = lived_state(3_102);
    let mut s = load_string(
        &save_string(&staged, 10_000),
        10_000 + 10 * MIN_REPORTABLE_ABSENCE_SECONDS,
    )
    .expect("the staging load succeeds")
    .state;
    for i in 0..1_500u64 {
        s.journal.record(s.ticks + i, &rolling_event());
    }
    assert!(
        s.journal.kept.iter().any(|e| matches!(e.item, JournalItem::Away(_))),
        "the journal carries an away-report"
    );
    assert!(
        s.journal.kept.iter().any(|e| matches!(e.item, JournalItem::Happened(_))),
        "and kept milestones"
    );
    assert!(!s.journal.recent.is_empty(), "and a loaded rolling window");

    let json = save_string(&s, 90_000);
    let loaded = load_string(&json, 90_000).expect("a journal-heavy save loads");
    assert_eq!(loaded.state.journal, s.journal, "every retained entry survives verbatim");
    assert_eq!(
        save_string(&loaded.state, 90_000),
        json,
        "save -> load -> save stays byte-stable with the journal in it"
    );
}

// @spec SAVE-014
#[test]
fn the_save_format_carries_version_three() {
    assert_eq!(SAVE_VERSION, 3, "the journal's arrival is save_version 3");
    let v: serde_json::Value =
        serde_json::from_str(&save_string(&GameState::new(3_110), 7)).unwrap();
    assert_eq!(v["save_version"], 3, "a freshly written save stamps 3");
}

// @spec SAVE-014
#[test]
fn the_v2_fixture_is_committed_and_predates_the_journal() {
    let fixture = v2_fixture();
    let v: serde_json::Value = serde_json::from_str(&fixture).expect("the v2 fixture is JSON");
    assert_eq!(v["save_version"], 2, "the fixture is a v2 save, frozen as a historical artifact");
    let state = v["state"].as_object().expect("a save carries a state object");
    assert!(
        !state.contains_key("journal"),
        "a v2 save predates the journal entirely — that is what makes it the migration case"
    );
    assert!(
        state.contains_key("speed") && state.contains_key("revealed"),
        "and it is v2, not a relabelled v1: the v1->v2 fields are present"
    );
    assert!(
        state["ticks"].as_u64().is_some_and(|t| t > 0)
            && !state["kittens"].as_array().unwrap().is_empty(),
        "a played meadow, not a fresh game — fidelity needs something to preserve"
    );
}

// @spec SAVE-014
#[test]
fn a_v2_save_migrates_to_an_empty_journal_with_its_meadow_otherwise_exact() {
    let fixture = v2_fixture();
    let before: serde_json::Value = serde_json::from_str(&fixture).unwrap();
    let saved_at = before["saved_at_unix"].as_u64().expect("the fixture stamps its save time");

    // Loading at the save's own instant replays nothing, so every difference
    // that shows up is migration rather than physics.
    let loaded = load_string(&fixture, saved_at).expect("a v2 save loads");
    assert_eq!(loaded.report.elapsed_seconds, 0, "no time passed");
    assert_eq!(
        loaded.state.journal,
        Journal::default(),
        "a save written before the journal existed has no history to recover"
    );

    let resaved: serde_json::Value =
        serde_json::from_str(&save_string(&loaded.state, saved_at)).unwrap();
    assert_eq!(resaved["save_version"], SAVE_VERSION, "migrated forward, then stamped current");
    assert_eq!(resaved["saved_at_unix"], saved_at);

    for (key, was) in before["state"].as_object().unwrap() {
        let now = resaved["state"]
            .get(key)
            .unwrap_or_else(|| panic!("migration dropped the v2 field `{key}`"));
        assert!(json_eq(was, now), "`{key}` changed across migration: {was} -> {now}");
    }
}

// @spec SAVE-014
#[test]
fn a_migrated_v2_save_journals_its_own_history_from_where_it_resumed() {
    let fixture = v2_fixture();
    let saved_at = serde_json::from_str::<serde_json::Value>(&fixture).unwrap()["saved_at_unix"]
        .as_u64()
        .unwrap();
    let mut s = load_string(&fixture, saved_at).expect("a v2 save loads").state;
    let resumed_at = s.ticks;

    run_ticks(&mut s, 1_000);
    let entries = s.journal.iter_chronological();
    assert!(!entries.is_empty(), "the migrated game is a working v3 game and records history");
    assert!(
        entries.iter().all(|e| e.tick >= resumed_at),
        "and claims no history from before the journal existed"
    );
}

// @spec SAVE-015
#[test]
fn a_qualifying_absence_is_recorded_into_the_journal_as_a_retained_entry() {
    let s = lived_state(3_103);
    let lived = s.journal.clone();
    let json = save_string(&s, 200_000);

    let away = MIN_REPORTABLE_ABSENCE_SECONDS + 600;
    let loaded = load_string(&json, 200_000 + away).expect("the save loads");
    assert_eq!(loaded.report.elapsed_seconds, away);

    let recorded = away_entries(&loaded.state.journal);
    assert_eq!(recorded.len(), 1, "one absence, one journal entry");
    assert_eq!(recorded[0].1, loaded.report, "the entry is the report the caller was handed");

    // Retained, not rolling: no amount of subsequent play evicts it.
    assert!(
        loaded.state.journal.kept.iter().any(|e| matches!(e.item, JournalItem::Away(_))),
        "an away-report is a kept entry"
    );
    let mut flooded = loaded.state.clone();
    for i in 0..5_000u64 {
        flooded.journal.record(flooded.ticks + i, &rolling_event());
    }
    assert_eq!(away_entries(&flooded.journal), recorded, "the away entry never rolls off");

    // It reads back after everything the player lived through before leaving,
    // and no later than the moment they came back.
    let last_lived = lived.iter_chronological().last().map(|e| e.tick).unwrap_or(0);
    assert!(recorded[0].0 >= last_lived, "an absence sorts after the play that preceded it");
    assert!(recorded[0].0 <= loaded.state.ticks, "and no later than the return");

    // The absence is recorded as its folded report, not as its ticks.
    assert_eq!(
        occurrences(&loaded.state.journal),
        occurrences(&lived),
        "replay events never join the journal; only the fold does"
    );
}

// @spec SAVE-015
#[test]
fn an_absence_below_the_reportable_minimum_leaves_no_trace() {
    let s = lived_state(3_104);
    let lived = s.journal.clone();
    let json = save_string(&s, 300_000);

    let brief = MIN_REPORTABLE_ABSENCE_SECONDS - 1;
    let loaded = load_string(&json, 300_000 + brief).expect("the save loads");
    assert_eq!(loaded.report.elapsed_seconds, brief, "the caller is still handed a report");
    assert!(loaded.state.ticks > s.ticks, "and time really did pass");
    assert!(
        away_entries(&loaded.state.journal).is_empty(),
        "nothing worth remembering happened in a blink"
    );
    assert_eq!(loaded.state.journal, lived, "the journal is exactly as the player left it");
}

// @spec SAVE-015
#[test]
fn an_away_report_survives_being_dismissed_unread_and_the_game_being_closed() {
    let s = lived_state(3_105);
    let json = save_string(&s, 400_000);
    let away = 30 * MIN_REPORTABLE_ABSENCE_SECONDS;

    let loaded = load_string(&json, 400_000 + away).expect("the save loads");
    let recorded = away_entries(&loaded.state.journal);
    assert_eq!(recorded.len(), 1);

    // The shell never renders the report — it saves and quits with it unread.
    let closed = save_string(&loaded.state, 400_000 + away);
    let reopened = load_string(&closed, 400_000 + away).expect("the game reopens");
    assert_eq!(
        away_entries(&reopened.state.journal),
        recorded,
        "the unread report is still there the next time the game opens"
    );
    assert_ne!(
        reopened.report, recorded[0].1,
        "and it is the journal that preserved it, not a re-derived report"
    );
}

// @spec SAVE-015
#[test]
fn each_qualifying_absence_is_recorded_in_its_own_right() {
    let s = lived_state(3_106);
    let away = MIN_REPORTABLE_ABSENCE_SECONDS + 120;

    let first = load_string(&save_string(&s, 500_000), 500_000 + away).expect("first load");
    let second =
        load_string(&save_string(&first.state, 600_000), 600_000 + away).expect("second load");

    let recorded = away_entries(&second.state.journal);
    assert_eq!(recorded.len(), 2, "two absences, two entries");
    assert_eq!(recorded[0].1, first.report, "the older absence keeps its own report");
    assert_eq!(recorded[1].1, second.report, "the newer one keeps its own");
    assert!(recorded[0].0 <= recorded[1].0, "and they read back in the order they were lived");
}

// @spec SAVE-016
#[test]
fn the_catch_up_cap_is_public_and_is_the_rule_load_applies() {
    // Reachability is the requirement, and this file is outside the crate:
    // naming the constant here is the test that a caller can name it too.
    assert_eq!(CATCHUP_CAP_SECONDS, 365 * 86_400, "a year of absence is replayed");

    // A caller that observed an interval itself gets the same answer from the
    // cap it can read as load gives the interval it computed — one rule.
    let s = played_state(3_107);
    let json = save_string(&s, 700_000);
    for observed in [0u64, 1, 600, 3_600] {
        let loaded = load_string(&json, 700_000 + observed).expect("the save loads");
        assert_eq!(
            loaded.report.elapsed_seconds,
            observed.min(CATCHUP_CAP_SECONDS),
            "elapsed is the observed interval under the exported cap"
        );
        assert_eq!(
            loaded.report.truncated,
            observed > CATCHUP_CAP_SECONDS,
            "and truncation is that same cap, not a second opinion"
        );
    }
}

// @spec SAVE-016
#[test]
#[ignore = "replays a year of ticks twice; run with --release --ignored"]
fn the_cap_is_one_rule_measured_in_seconds_of_absence() {
    let mut s = GameState::new(3_108);
    s.apply(Command::SetSpeed(0.25)).unwrap(); // fewer ticks to replay, same seconds away
    let json = save_string(&s, 0);

    let at_cap = load_string(&json, CATCHUP_CAP_SECONDS).expect("a year away loads");
    assert!(!at_cap.report.truncated, "a year away is replayed in full, not clipped");
    assert_eq!(at_cap.report.elapsed_seconds, CATCHUP_CAP_SECONDS);

    let over_cap = load_string(&json, CATCHUP_CAP_SECONDS + 86_400).expect("a longer away loads");
    assert!(over_cap.report.truncated, "beyond the cap is clock damage, and is flagged");
    assert_eq!(over_cap.report.elapsed_seconds, CATCHUP_CAP_SECONDS, "clamped to the same cap");
    assert_eq!(
        over_cap.state.ticks, at_cap.state.ticks,
        "the cap bounds seconds of absence, and one rule governs both sides of it"
    );
}
