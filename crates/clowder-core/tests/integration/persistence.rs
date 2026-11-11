use crate::common;

use clowder_core::*;
use common::*;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

/// A scratch directory of this call's own: tests run in parallel in one
/// process, so the name carries a per-call counter as well as the process id.
fn scratch_dir(test: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir =
        std::env::temp_dir().join(format!("clowder-test-{}-{}-{n}", std::process::id(), test));
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
    assert!(
        !dir.join("save.json.tmp").exists(),
        "tmp is renamed away on success"
    );

    fs::write(dir.join("save.json.tmp"), b"{ garbage again").unwrap();
    let loaded = read_save(&path, 5000).unwrap();
    assert_eq!(
        save_string(&loaded.state, 5000),
        save_string(&s, 5000),
        "load ignores the tmp"
    );
}

// @spec SAVE-004
#[test]
fn load_errors_are_typed_and_never_touch_the_file() {
    let dir = scratch_dir("errors");
    assert!(matches!(
        read_save(&dir.join("save.json"), 0),
        Err(LoadError::NotFound)
    ));

    let corrupt_path = dir.join("save.json");
    fs::write(&corrupt_path, b"not json at all").unwrap();
    assert!(matches!(
        read_save(&corrupt_path, 0),
        Err(LoadError::Corrupt { .. })
    ));
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
    assert!(matches!(
        load_string(&target, 1000),
        Err(LoadError::Corrupt { .. })
    ));
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
    assert_eq!(
        loaded.state.pity, 255,
        "wide-deserialized counters clamp to range"
    );
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
        manual
            .journal
            .iter_chronological()
            .iter()
            .any(|e| matches!(&e.item, JournalItem::Happened(_))),
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
    assert_eq!(
        save_string(&loaded.state, 0),
        save_string(&s, 0),
        "state never regresses"
    );
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
    assert_eq!(
        wandered_total, 2,
        "two kittens wandered; the last stayed home"
    );
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
    assert!(took.as_secs_f64() < 10.0, "30-day catch-up took {took:?}");
}

/// The resource-reveal rule leaves that bound standing at either extreme of
/// its own cost, on the same meadow SAVE-009's bound is measured on: with no
/// resource yet revealed — every one still to be compared each tick, and the
/// meadow's stores revealing as the replay fills them — and with every
/// resource already revealed, nothing left to compare. A meadow that has met
/// nothing at all, idling, is the third case: the whole comparison every tick
/// for the whole absence.
// @spec SAVE-009, SIM-ENG-055
#[test]
#[ignore = "perf bound is a release-build property; run with --release --ignored"]
fn thirty_day_catch_up_holds_its_bound_under_the_resource_reveal_rule() {
    let mut s = GameState::new(10);
    grant(&mut s, StructureId::CatnipPatch, 5);
    grant(&mut s, StructureId::CozyCushion, 8);
    give(&mut s, ResourceId::Catnip, 200.0);
    let none_met = save_string(&s, 0);
    let all_met = {
        let mut v: serde_json::Value = serde_json::from_str(&none_met).unwrap();
        v["state"]["revealed_resources"] = serde_json::to_value(ResourceId::ALL.to_vec()).unwrap();
        v.to_string()
    };
    let idle = save_string(&GameState::new(10), 0);

    for (what, json) in [
        ("SAVE-009's meadow, no resource yet revealed", none_met),
        ("SAVE-009's meadow, every resource revealed", all_met),
        ("an idle meadow that has met nothing", idle),
    ] {
        let start = std::time::Instant::now();
        let loaded = load_string(&json, 30 * 86_400).unwrap();
        let took = start.elapsed();
        assert_eq!(loaded.state.ticks, 30 * 86_400 * 4);
        assert!(
            took.as_secs_f64() < 10.0,
            "30-day catch-up of {what} took {took:?}"
        );
    }
}

/// The meadow SAVE-033's bound is measured on: twenty copies of every structure
/// kind, everything else it can hold, and as many kittens as those copies house.
/// Built from the content lists rather than by name, so what later content
/// costs a tick is measured here without anyone remembering to add it.
fn late_meadow() -> GameState {
    let mut s = GameState::new(33);
    for structure in StructureId::ALL {
        grant(&mut s, structure, 20);
    }
    s.ideas.extend(IdeaId::ALL);
    s.refinements.extend(RefinementId::ALL);
    for rule in RuleId::ALL {
        if !s.rules.contains(&content::rule_def(rule).blocks) {
            s.rules.insert(rule);
        }
    }
    s.rites.extend(RiteId::ALL);
    s.rite_counts = [3; RiteId::ALL.len()];
    s.arches_bought.extend(ArchId::ALL);
    s.traditions.extend(TraditionId::ALL);
    s.dream_looms.extend(DreamLoomId::ALL);
    s.promises = vec![Promise { quiet: false }; 3];
    s.generation = 3;
    s.elder_wisdom = 50.0;
    s.passed_down_wisdom = 50.0;
    s.warm_memories = 10.0;
    s.devotion = 1_000.0;
    s.serenity = 10.0;
    s.enlightenment_tier = 3;
    s.adventure_rewards = [0.1; AdventureId::ALL.len()];

    let seats = s.bench().seats as usize;
    let mut bound = 0;
    for i in 0..s.housing_capacity() as usize {
        let name = format!("Kitten {i}");
        let job = JobId::ALL[i % JobId::ALL.len()];
        let tinkering = (job == JobId::Tinkerer && bound < seats).then(|| {
            bound += 1;
            CraftId::ALL[i % CraftId::ALL.len()]
        });
        let profile = draw_profile(&name, s.generation);
        s.kittens.push(Kitten {
            name,
            job: Some(job),
            location: Location::Home,
            arrived_day: 0,
            job_seq: Some(i as u64),
            away_seq: None,
            skills: [3_000.0; JobId::ALL.len()],
            exp: 0.0,
            rank: 2,
            trait_id: KittenTrait::ALL[i % KittenTrait::ALL.len()],
            tinkering,
            profile: Some(profile),
        });
    }
    s.elder = Some(0);
    for r in ResourceId::ALL {
        let cap = s.cap(r);
        if cap.is_finite() && cap > 0.0 {
            give(&mut s, r, cap / 2.0);
        }
    }
    s
}

/// A guard is worth only what its meadow is, so the meadow is held to what the
/// spec says it holds, and to staying that size while it replays.
// @spec SAVE-033
#[test]
fn the_late_meadow_holds_what_its_bound_says() {
    let s = late_meadow();
    assert!(
        s.structures.iter().all(|&copies| copies == 20),
        "twenty copies of every structure kind"
    );
    assert_eq!(s.refinements.len(), RefinementId::ALL.len());
    assert_eq!(s.rites.len(), RiteId::ALL.len());
    assert_eq!(s.arches_bought.len(), ArchId::ALL.len());
    assert_eq!(s.traditions.len(), TraditionId::ALL.len());
    assert_eq!(s.dream_looms.len(), DreamLoomId::ALL.len());
    for rule in RuleId::ALL {
        assert!(
            s.rules.contains(&rule) || s.rules.contains(&content::rule_def(rule).blocks),
            "{rule:?} is held, or the rule that blocks it is"
        );
    }
    assert_eq!(
        s.kittens.len(),
        s.housing_capacity() as usize,
        "as many kittens as its copies house"
    );
    assert!(
        s.kittens.len() >= 200,
        "about two hundred kittens, and more only as content adds homes"
    );

    let json = save_string(&s, 0);
    let loaded = load_string(&json, 59).unwrap();
    assert_eq!(
        loaded.state.kittens.len(),
        s.kittens.len(),
        "the roster it starts with is the one the replay runs"
    );
}

/// Every source class is live on this meadow, so the replay's every step reads
/// what a sum over the whole content would: replayed or ticked, it is one
/// simulation, byte for byte. An absence under the reportable minimum, so the
/// two differ in no journal entry.
// @spec SAVE-006, SIM-ENG-084
#[test]
fn a_late_meadow_replays_exactly_as_it_lives() {
    let json = save_string(&late_meadow(), 0);
    let replayed = load_string(&json, MIN_REPORTABLE_ABSENCE_SECONDS - 1)
        .unwrap()
        .state;

    let mut ticked = load_string(&json, 0).unwrap().state;
    for _ in 0..replayed.ticks - ticked.ticks {
        ticked.tick_replay();
    }
    assert_eq!(
        save_string(&replayed, 0),
        save_string(&ticked, 0),
        "a catch-up is the same ticks as any others"
    );
}

/// The late meadow's bound, at more than twice what it measures, so this fails
/// on a real regression and not on a busy machine.
// @spec SAVE-033
#[test]
#[ignore = "perf bound is a release-build property; run with --release --ignored"]
fn thirty_days_away_from_a_late_meadow_wakes_within_a_minute() {
    let json = save_string(&late_meadow(), 0);
    let start = std::time::Instant::now();
    let loaded = load_string(&json, 30 * 86_400).unwrap();
    let took = start.elapsed();
    assert_eq!(loaded.state.ticks, 30 * 86_400 * 4);
    assert!(took.as_secs_f64() < 60.0, "30-day catch-up took {took:?}");
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
    assert!(
        !json.contains("happiness"),
        "derived values are recomputed, not stored"
    );
    assert!(!json.contains("mood"));
    assert!(!json.contains("event"));
}

// @spec SAVE-012
#[test]
fn the_default_path_is_the_platform_data_dir() {
    let path = default_save_path().expect("platform data dir resolves");
    let tail: PathBuf = path
        .iter()
        .rev()
        .take(2)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    assert_eq!(tail, PathBuf::from("clowder/save.json"));
}

// ---- Depth Wave 1 ----

// @spec SAVE-013
#[test]
fn v1_saves_migrate_forward_losslessly() {
    let fixture = include_str!("../fixtures/save_v1.json");
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

/// A migrated save's kittens and memorials with their profiles set aside —
/// the one thing each gains from a save written before profiles (SAVE-028) —
/// after checking every one has gained one; any other field as it is.
fn profiles_set_aside(key: &str, now: &serde_json::Value) -> serde_json::Value {
    let mut now = now.clone();
    if key == "kittens" || key == "family_album" {
        for entry in now.as_array_mut().into_iter().flatten() {
            let profile = entry.as_object_mut().and_then(|o| o.remove("profile"));
            assert!(
                profile.is_some_and(|p| !p.is_null()),
                "every entry of `{key}` gains a profile"
            );
        }
    }
    now
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
                && x.iter()
                    .all(|(k, v)| y.get(k).is_some_and(|w| json_eq(v, w)))
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

/// An event that must survive eviction forever.
fn milestone_event() -> Event {
    let e = Event::StructureBuilt {
        id: StructureId::CatnipPatch,
        count: 1,
    };
    assert_eq!(
        e.class(),
        EventClass::Milestone,
        "this helper must drive the kept list"
    );
    e
}

/// An event the rolling window is allowed to evict.
fn rolling_event() -> Event {
    let e = Event::Flavor {
        template: 0,
        kitten: None,
    };
    assert_ne!(
        e.class(),
        EventClass::Milestone,
        "this helper must drive the rolling window"
    );
    e
}

// @spec SAVE-011
#[test]
fn derived_values_are_recomputed_after_load_but_the_journal_cannot_be() {
    let s = lived_state(3_101);
    assert!(
        !s.journal.iter_chronological().is_empty(),
        "there is lived history to keep"
    );

    let json = save_string(&s, 60_000);
    let whole: serde_json::Value = serde_json::from_str(&json).unwrap();
    let keys: Vec<&String> = whole["state"].as_object().unwrap().keys().collect();
    for derived in [
        "happiness",
        "mood",
        "mood_tier",
        "season",
        "rate",
        "rates",
        "net_rates",
    ] {
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

    assert_eq!(
        stripped.season(),
        s.season(),
        "season recomputes from state"
    );
    assert_eq!(
        stripped.mood_tier(),
        s.mood_tier(),
        "mood recomputes from state"
    );
    assert!(
        (stripped.happiness() - s.happiness()).abs() < TOL,
        "happiness recomputes"
    );
    assert_eq!(stripped.net_rates(), s.net_rates(), "rates recompute");
    assert_eq!(
        stripped.journal,
        Journal::default(),
        "nothing in the game can recompute what the player lived through"
    );

    let kept = load_string(&json, 60_000)
        .expect("the real save loads")
        .state;
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
    // A genuine milestone, recorded explicitly. Ticking alone no longer
    // produces one: milestones must be bounded over a playthrough, so the
    // recurring calendar turn is ordinary and only the *first* of a structure
    // is the moment worth keeping. @spec SIM-ENG-026
    s.journal.record(s.ticks, &milestone_event());
    for i in 0..1_500u64 {
        s.journal.record(s.ticks + i, &rolling_event());
    }
    assert!(
        s.journal
            .kept
            .iter()
            .any(|e| matches!(e.item, JournalItem::Away(_))),
        "the journal carries an away-report"
    );
    assert!(
        s.journal
            .kept
            .iter()
            .any(|e| matches!(e.item, JournalItem::Happened(_))),
        "and kept milestones"
    );
    assert!(!s.journal.recent.is_empty(), "and a loaded rolling window");

    let json = save_string(&s, 90_000);
    let loaded = load_string(&json, 90_000).expect("a journal-heavy save loads");
    assert_eq!(
        loaded.state.journal, s.journal,
        "every retained entry survives verbatim"
    );
    assert_eq!(
        save_string(&loaded.state, 90_000),
        json,
        "save -> load -> save stays byte-stable with the journal in it"
    );
}

// @spec SAVE-014
#[test]
fn a_fresh_save_stamps_the_current_format_version() {
    // Deliberately not a literal: the property is that what is written matches
    // what this binary understands, which must hold at every future version.
    // Migration from each older version is pinned by its own fixture test.
    let v: serde_json::Value =
        serde_json::from_str(&save_string(&GameState::new(3_110), 7)).unwrap();
    assert_eq!(
        v["save_version"], SAVE_VERSION,
        "a freshly written save stamps the current version"
    );
}

// @spec SAVE-014
#[test]
fn the_v2_fixture_is_committed_and_predates_the_journal() {
    let fixture = v2_fixture();
    let v: serde_json::Value = serde_json::from_str(&fixture).expect("the v2 fixture is JSON");
    assert_eq!(
        v["save_version"], 2,
        "the fixture is a v2 save, frozen as a historical artifact"
    );
    let state = v["state"]
        .as_object()
        .expect("a save carries a state object");
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
    let saved_at = before["saved_at_unix"]
        .as_u64()
        .expect("the fixture stamps its save time");

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
    assert_eq!(
        resaved["save_version"], SAVE_VERSION,
        "migrated forward, then stamped current"
    );
    assert_eq!(resaved["saved_at_unix"], saved_at);

    for (key, was) in before["state"].as_object().unwrap() {
        let now = resaved["state"]
            .get(key)
            .unwrap_or_else(|| panic!("migration dropped the v2 field `{key}`"));
        assert!(
            kept(was, &profiles_set_aside(key, now)),
            "`{key}` changed across migration: {was} -> {now}"
        );
    }
}

// @spec SAVE-014
#[test]
fn a_migrated_v2_save_journals_its_own_history_from_where_it_resumed() {
    let fixture = v2_fixture();
    let saved_at = serde_json::from_str::<serde_json::Value>(&fixture).unwrap()["saved_at_unix"]
        .as_u64()
        .unwrap();
    let mut s = load_string(&fixture, saved_at)
        .expect("a v2 save loads")
        .state;
    let resumed_at = s.ticks;

    run_ticks(&mut s, 1_000);
    let entries = s.journal.iter_chronological();
    assert!(
        !entries.is_empty(),
        "the migrated game is a working v3 game and records history"
    );
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
    assert_eq!(
        recorded[0].1, loaded.report,
        "the entry is the report the caller was handed"
    );

    // Retained, not rolling: no amount of subsequent play evicts it.
    assert!(
        loaded
            .state
            .journal
            .kept
            .iter()
            .any(|e| matches!(e.item, JournalItem::Away(_))),
        "an away-report is a kept entry"
    );
    let mut flooded = loaded.state.clone();
    for i in 0..5_000u64 {
        flooded.journal.record(flooded.ticks + i, &rolling_event());
    }
    assert_eq!(
        away_entries(&flooded.journal),
        recorded,
        "the away entry never rolls off"
    );

    // It reads back after everything the player lived through before leaving,
    // and no later than the moment they came back.
    let last_lived = lived
        .iter_chronological()
        .last()
        .map(|e| e.tick)
        .unwrap_or(0);
    assert!(
        recorded[0].0 >= last_lived,
        "an absence sorts after the play that preceded it"
    );
    assert!(
        recorded[0].0 <= loaded.state.ticks,
        "and no later than the return"
    );

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
    assert_eq!(
        loaded.report.elapsed_seconds, brief,
        "the caller is still handed a report"
    );
    assert!(loaded.state.ticks > s.ticks, "and time really did pass");
    assert!(
        away_entries(&loaded.state.journal).is_empty(),
        "nothing worth remembering happened in a blink"
    );
    assert_eq!(
        loaded.state.journal, lived,
        "the journal is exactly as the player left it"
    );
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
    assert_eq!(
        recorded[0].1, first.report,
        "the older absence keeps its own report"
    );
    assert_eq!(recorded[1].1, second.report, "the newer one keeps its own");
    assert!(
        recorded[0].0 <= recorded[1].0,
        "and they read back in the order they were lived"
    );
}

// @spec SAVE-016
#[test]
fn the_catch_up_cap_is_public_and_is_the_rule_load_applies() {
    // Reachability is the requirement, and this file is outside the crate:
    // naming the constant here is the test that a caller can name it too.
    assert_eq!(
        CATCHUP_CAP_SECONDS,
        365 * 86_400,
        "a year of absence is replayed"
    );

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
    assert!(
        !at_cap.report.truncated,
        "a year away is replayed in full, not clipped"
    );
    assert_eq!(at_cap.report.elapsed_seconds, CATCHUP_CAP_SECONDS);

    let over_cap = load_string(&json, CATCHUP_CAP_SECONDS + 86_400).expect("a longer away loads");
    assert!(
        over_cap.report.truncated,
        "beyond the cap is clock damage, and is flagged"
    );
    assert_eq!(
        over_cap.report.elapsed_seconds, CATCHUP_CAP_SECONDS,
        "clamped to the same cap"
    );
    assert_eq!(
        over_cap.state.ticks, at_cap.state.ticks,
        "the cap bounds seconds of absence, and one rule governs both sides of it"
    );
}

// =====================================================================
// Save format v4 — banked ticks become part of the save
//
// A v3 save is what the previous build wrote, so a player who has been
// playing has one. SAVE-013 asks for a committed fixture of every prior
// version; this is the one that was missing when the version moved.
// =====================================================================

const V3_FIXTURE_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/save_v3.json");

fn v3_fixture() -> String {
    fs::read_to_string(V3_FIXTURE_PATH).unwrap_or_else(|e| {
        panic!("SAVE-013 requires a committed v3 fixture at {V3_FIXTURE_PATH}: {e}")
    })
}

/// @spec SAVE-013, SAVE-017
#[test]
fn the_v3_fixture_is_committed_and_predates_the_banked_tick_count() {
    let fixture = v3_fixture();
    let v: serde_json::Value = serde_json::from_str(&fixture).expect("the v3 fixture is JSON");
    assert_eq!(
        v["save_version"], 3,
        "the fixture is a v3 save, frozen as a historical artifact"
    );
    assert!(
        v.get("owed_ticks").is_none(),
        "a v3 save predates the banked count entirely — that is what makes it the migration case"
    );
    let state = v["state"]
        .as_object()
        .expect("a save carries a state object");
    assert!(
        state.contains_key("journal"),
        "and it is v3, not a relabelled v2: the journal that v3 added is present"
    );
    assert!(
        state["ticks"].as_u64().is_some_and(|t| t > 0)
            && !state["kittens"].as_array().unwrap().is_empty(),
        "a played meadow, not a fresh game — fidelity needs something to preserve"
    );
}

/// A save written before ticks were counted owes none. The field defaults
/// rather than being derived from the interval, which is the whole point of
/// SAVE-017: a count that was never written cannot be reconstructed from a
/// timestamp and a speed, and guessing at it would hand the player ticks they
/// never earned or take ones they did.
/// @spec SAVE-013, SAVE-017
#[test]
fn a_v3_save_migrates_owing_nothing_with_its_meadow_otherwise_exact() {
    let fixture = v3_fixture();
    let before: serde_json::Value = serde_json::from_str(&fixture).unwrap();
    let saved_at = before["saved_at_unix"]
        .as_u64()
        .expect("the fixture stamps its save time");

    // Loading at the save's own instant replays nothing, so every difference
    // that shows up is migration rather than physics.
    let loaded = load_string(&fixture, saved_at).expect("a v3 save loads");
    assert_eq!(loaded.report.elapsed_seconds, 0, "no time passed");
    assert_eq!(
        loaded.state.ticks,
        before["state"]["ticks"].as_u64().unwrap(),
        "a save that banked nothing must not advance on load"
    );

    let resaved: serde_json::Value =
        serde_json::from_str(&save_string(&loaded.state, saved_at)).unwrap();
    assert_eq!(
        resaved["save_version"], SAVE_VERSION,
        "migrated forward, then stamped current"
    );
    assert_eq!(
        resaved["owed_ticks"], 0,
        "nothing was banked, so nothing is owed"
    );

    for (key, was) in before["state"].as_object().unwrap() {
        let now = &resaved["state"][key];
        assert!(
            kept(was, &profiles_set_aside(key, now)),
            "migrating a v3 save changed `{key}`: {was} became {now}"
        );
    }
}

// =====================================================================
// Paced catch-up — SAVE-018 to SAVE-022
//
// A load hands back its catch-up pending, and the caller advances it by
// counts of its own choosing (`begin_load` → `CatchUp`). The catch-up either
// finishes — `Advance::Finished` carrying exactly what a one-call
// `load_string` returns — or is set down early (`CatchUp::set_down` →
// `SetDown`), and the save written from a set-down carries its report so far
// (`save_string_carrying` / `write_save_carrying`) for the next load to
// continue.
//
// The surface these tests address, which the code stage adds:
//
//   pub fn begin_load(s: &str, now_unix: u64) -> Result<CatchUp, LoadError>
//   impl CatchUp {
//       pub fn total_ticks(&self) -> u64
//       pub fn remaining_ticks(&self) -> u64
//       pub fn advance(self, max_ticks: u64) -> Advance
//       pub fn set_down(self) -> SetDown
//   }
//   pub enum Advance { Pending(CatchUp), Finished(LoadedGame) }
//   pub struct SetDown { pub state: GameState, pub owed_ticks: u64,
//                        pub report_so_far: OfflineReport }
//   pub fn save_string_carrying(state: &GameState, saved_at_unix: u64,
//       owed_ticks: u64, report_so_far: Option<&OfflineReport>) -> String
//   pub fn write_save_carrying(path: &Path, state: &GameState,
//       saved_at_unix: u64, owed_ticks: u64,
//       report_so_far: Option<&OfflineReport>) -> std::io::Result<()>
//
// An advance on a catch-up with nothing left to replay (no absence, a
// backwards clock) finishes it: there is no last tick to wait for.
// =====================================================================

/// The catch-up was still owed ticks; the test needed it finished.
fn finished(a: Advance) -> LoadedGame {
    match a {
        Advance::Finished(g) => g,
        Advance::Pending(c) => {
            panic!(
                "the catch-up should have finished, but {} ticks remain",
                c.remaining_ticks()
            )
        }
    }
}

/// The catch-up finished; the test needed it still pending.
fn pending(a: Advance) -> CatchUp {
    match a {
        Advance::Pending(c) => c,
        Advance::Finished(_) => panic!("the catch-up finished early"),
    }
}

/// Advance in the given chunk sizes, cycling them, until the catch-up
/// finishes.
fn drive(mut c: CatchUp, chunks: &[u64]) -> LoadedGame {
    let mut i = 0usize;
    loop {
        match c.advance(chunks[i % chunks.len()]) {
            Advance::Finished(g) => return g,
            Advance::Pending(next) => c = next,
        }
        i += 1;
        assert!(i < 10_000_000, "a catch-up that never finishes");
    }
}

/// Advance until exactly `target` ticks have been replayed, in chunks of at
/// most `chunk`, and return the catch-up still pending.
fn advance_to(mut c: CatchUp, target: u64, chunk: u64) -> CatchUp {
    let total = c.total_ticks();
    assert!(
        target < total,
        "fixture: set-down point must precede the last tick"
    );
    loop {
        let replayed = total - c.remaining_ticks();
        if replayed == target {
            return c;
        }
        assert!(
            replayed < target,
            "an advance replayed more than it was asked to"
        );
        c = pending(c.advance(chunk.min(target - replayed)));
    }
}

fn state_bytes(s: &GameState) -> String {
    save_string(s, 0)
}

/// A meadow whose absence has a tale: three kittens with no food and no jobs,
/// so kittens wander and seasons turn while it is away (SAVE-008's fixture).
fn famine_state(seed: u64) -> GameState {
    let mut s = GameState::new(seed);
    adopt(&mut s, 3);
    for k in &mut s.kittens {
        k.job = None;
    }
    give(&mut s, ResourceId::Catnip, 0.0);
    s
}

/// Float deltas folded in two parts are the same sum to within rounding.
fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
}

fn assert_same_tale(got: &OfflineReport, want: &OfflineReport, ctx: &str) {
    assert_eq!(got.elapsed_seconds, want.elapsed_seconds, "{ctx}: span");
    assert_eq!(got.truncated, want.truncated, "{ctx}: truncated");
    assert_eq!(got.arrived, want.arrived, "{ctx}: arrivals");
    assert_eq!(got.wandered, want.wandered, "{ctx}: wanderings");
    assert_eq!(got.returned, want.returned, "{ctx}: returns");
    assert_eq!(got.seasons_passed, want.seasons_passed, "{ctx}: seasons");
    for r in ResourceId::ALL {
        assert!(
            close(got.resource_deltas[r.ix()], want.resource_deltas[r.ix()]),
            "{ctx}: net delta for {r:?}: {} vs {}",
            got.resource_deltas[r.ix()],
            want.resource_deltas[r.ix()]
        );
    }
}

/// A report a set-down catch-up might have carried, with names and fractions
/// that a fresh fold could not produce by accident.
fn carried_report(elapsed_seconds: u64, truncated: bool) -> OfflineReport {
    let mut r = OfflineReport {
        elapsed_seconds,
        truncated,
        ..OfflineReport::default()
    };
    for (i, d) in r.resource_deltas.iter_mut().enumerate() {
        *d = 0.375 * (i as f64 + 1.0) - 1.5;
    }
    r.arrived = vec![("Pemberly".into(), 2)];
    r.wandered = vec![("Pemberly".into(), 1)];
    r.returned = vec![("Quillon".into(), 3)];
    r.seasons_passed = 3;
    r
}

fn count_of(list: &[(String, u32)], name: &str) -> u32 {
    list.iter()
        .filter(|(n, _)| n == name)
        .map(|(_, c)| *c)
        .sum()
}

// ---------------------------------------------------------------------
// SAVE-018 — the caller paces the catch-up
// ---------------------------------------------------------------------

/// The total is the interval's ticks at the save's own speed plus the ticks
/// the save carried as owed, and load replays none of them before it returns.
/// @spec SAVE-006, SAVE-018
#[test]
fn a_paced_load_owes_the_intervals_ticks_plus_the_carried_count_and_has_replayed_none() {
    for (speed, owed) in [(1.0, 0u64), (1.0, 37), (2.0, 37), (0.25, 5)] {
        let mut s = played_state(18_001);
        s.apply(Command::SetSpeed(speed)).unwrap();
        let json = save_string_owing(&s, 50_000, owed);
        let c = begin_load(&json, 50_000 + 600).expect("the save loads");
        let interval_ticks = (600.0 * 4.0 * speed) as u64;
        assert_eq!(
            c.total_ticks(),
            interval_ticks + owed,
            "at speed {speed} with {owed} owed: the interval's ticks plus the carried count"
        );
        assert_eq!(
            c.remaining_ticks(),
            c.total_ticks(),
            "load returns before replaying any of them"
        );

        // Setting it down at once shows the state exactly as saved.
        let sd = c.set_down();
        assert_eq!(sd.state.ticks, s.ticks, "no tick ran inside load");
        assert_eq!(
            sd.owed_ticks,
            interval_ticks + owed,
            "and every one is still owed"
        );
    }
}

/// Each advance replays at most the count the caller chose, and how many it
/// replays is decided by that count alone — two identical catch-ups advanced
/// by the same counts stand at the same place after every call.
/// @spec SAVE-018
#[test]
fn an_advance_replays_at_most_the_callers_count_and_the_count_alone_decides() {
    let s = played_state(18_002);
    let json = save_string(&s, 60_000);
    let counts = [1u64, 0, 7, 250, 3, 1_000, 2];
    let mut a = begin_load(&json, 60_000 + 600).unwrap();
    let mut b = begin_load(&json, 60_000 + 600).unwrap();
    assert_eq!(a.total_ticks(), 2_400);
    for &n in &counts {
        let before = a.remaining_ticks();
        a = pending(a.advance(n));
        // A deliberate pause between the twins: a pace decided by a clock
        // would read it; a pace decided by the count cannot.
        std::thread::sleep(std::time::Duration::from_millis(5));
        b = pending(b.advance(n));
        let replayed = before - a.remaining_ticks();
        assert!(replayed <= n, "asked for at most {n}, replayed {replayed}");
        assert_eq!(
            a.remaining_ticks(),
            b.remaining_ticks(),
            "the same counts, the same place"
        );
        assert_eq!(a.total_ticks(), 2_400, "the total never moves");
    }
    let replayed = a.total_ticks() - a.remaining_ticks();
    let sa = a.set_down();
    let sb = b.set_down();
    assert_eq!(
        sa.state.ticks,
        s.ticks + replayed,
        "remaining counts exactly the ticks not yet run"
    );
    assert_eq!(state_bytes(&sa.state), state_bytes(&sb.state));
}

/// However the catch-up is divided into calls, the same ticks run in the same
/// order and fold the same report — at the end, and at every point it could
/// be set down along the way.
/// @spec SAVE-018
#[test]
fn how_a_catch_up_is_divided_changes_nothing_it_produces() {
    let s = famine_state(18_003);
    let json = save_string(&s, 70_000);
    let now = 70_000 + 150; // 600 ticks: wanderings and a season turn
    let divisions: [&[u64]; 5] = [&[u64::MAX], &[1], &[7, 1, 120], &[599, 1], &[300, 0, 300]];

    let whole = finished(begin_load(&json, now).unwrap().advance(u64::MAX));
    assert!(
        !whole.report.wandered.is_empty() && whole.report.seasons_passed > 0,
        "fixture: the absence must have a tale for the fold to be compared"
    );
    for d in divisions {
        let g = drive(begin_load(&json, now).unwrap(), d);
        assert_eq!(
            state_bytes(&g.state),
            state_bytes(&whole.state),
            "division {d:?}: state"
        );
        assert_eq!(g.report, whole.report, "division {d:?}: report");
    }

    for k in [1u64, 150, 333, 599] {
        let reference = advance_to(begin_load(&json, now).unwrap(), k, u64::MAX).set_down();
        for chunk in [1u64, 13, 100] {
            let sd = advance_to(begin_load(&json, now).unwrap(), k, chunk).set_down();
            assert_eq!(
                state_bytes(&sd.state),
                state_bytes(&reference.state),
                "set down after {k} ticks in chunks of {chunk}: state"
            );
            assert_eq!(sd.owed_ticks, reference.owed_ticks);
            assert_eq!(
                sd.report_so_far, reference.report_so_far,
                "set down after {k} ticks in chunks of {chunk}: report so far"
            );
        }
    }
}

// ---------------------------------------------------------------------
// SAVE-019 — the advance that replays the last tick finishes it
// ---------------------------------------------------------------------

/// Finished by any division, the result is bit-identical to a one-call load
/// of the same save at the same moment — state, journal, and report.
/// @spec SAVE-019
#[test]
fn the_last_advance_finishes_bit_identical_to_a_one_call_load() {
    for (seed, away, owed) in [
        (19_001u64, 600u64, 0u64),
        (19_002, 150, 40),
        (19_003, 20, 0),
    ] {
        let s = famine_state(seed);
        let json = save_string_owing(&s, 80_000, owed);
        let one_call = load_string(&json, 80_000 + away).unwrap();
        for d in [&[u64::MAX][..], &[1], &[97, 3]] {
            let g = drive(begin_load(&json, 80_000 + away).unwrap(), d);
            assert_eq!(
                save_string(&g.state, 80_000 + away),
                save_string(&one_call.state, 80_000 + away),
                "away {away}s owing {owed}, division {d:?}: state and journal"
            );
            assert_eq!(
                g.report, one_call.report,
                "away {away}s, division {d:?}: report"
            );
        }
    }
}

/// Finishing records the completed report in the journal under the minimum
/// interval rule, exactly once — and never before the last tick.
/// @spec SAVE-019
#[test]
fn finishing_records_the_report_and_only_finishing_does() {
    let s = lived_state(19_004);
    let json = save_string(&s, 90_000);
    let away = MIN_REPORTABLE_ABSENCE_SECONDS + 240;
    let mut c = begin_load(&json, 90_000 + away).unwrap();
    let before = away_entries(&s.journal).len();
    loop {
        match c.advance(97) {
            Advance::Pending(next) => {
                c = next;
                let peek = begin_load(&json, 90_000 + away).unwrap();
                let at = c.total_ticks() - c.remaining_ticks();
                let sd = advance_to(peek, at, u64::MAX).set_down();
                assert_eq!(
                    away_entries(&sd.state.journal).len(),
                    before,
                    "nothing recorded mid-way"
                );
            }
            Advance::Finished(g) => {
                let recorded = away_entries(&g.state.journal);
                assert_eq!(recorded.len(), before + 1, "one absence, one entry");
                assert_eq!(
                    recorded.last().unwrap().1,
                    g.report,
                    "the entry is the finished report"
                );
                break;
            }
        }
    }

    let brief = save_string(&s, 95_000);
    let g = drive(
        begin_load(&brief, 95_000 + MIN_REPORTABLE_ABSENCE_SECONDS - 1).unwrap(),
        &[50],
    );
    assert_eq!(
        away_entries(&g.state.journal).len(),
        before,
        "below the minimum, nothing"
    );
}

/// A catch-up owing nothing — no time away, or a clock that ran backwards —
/// finishes on its first advance, as the one-call load would.
/// @spec SAVE-019
#[test]
fn a_catch_up_owing_nothing_finishes_on_its_first_advance() {
    let s = played_state(19_005);
    let json = save_string(&s, 100_000);
    for now in [100_000u64, 40_000] {
        let c = begin_load(&json, now).unwrap();
        assert_eq!(c.total_ticks(), 0);
        for n in [0u64, 1, 5_000] {
            let g = finished(begin_load(&json, now).unwrap().advance(n));
            let one_call = load_string(&json, now).unwrap();
            assert_eq!(
                state_bytes(&g.state),
                state_bytes(&one_call.state),
                "now {now}, advance {n}"
            );
            assert_eq!(g.report, one_call.report);
        }
    }
}

// ---------------------------------------------------------------------
// SAVE-020 — setting a catch-up down early
// ---------------------------------------------------------------------

/// Set down after some of its ticks: the state is the meadow those ticks
/// reached, the owed count is the rest, the report so far wears the
/// catch-up's own span and flag over contents folded from the ticks replayed,
/// and the journal is untouched.
/// @spec SAVE-020
#[test]
fn a_set_down_returns_the_state_so_far_the_rest_owed_and_the_report_so_far() {
    let s = famine_state(20_001);
    let json = save_string(&s, 110_000);
    let away = 150u64; // 600 ticks
    for k in [0u64, 1, 240, 599] {
        let c = begin_load(&json, 110_000 + away).unwrap();
        let sd = advance_to(c, k, 61).set_down();

        // The meadow after exactly k replayed ticks, folded the same way: a
        // load owing k ticks over no interval at all.
        let reference = load_string(&save_string_owing(&s, 110_000, k), 110_000).unwrap();
        assert_eq!(
            state_bytes(&sd.state),
            state_bytes(&reference.state),
            "after {k}: state so far"
        );
        assert_eq!(sd.owed_ticks, 600 - k, "after {k}: the rest is owed");
        assert_eq!(
            sd.report_so_far.elapsed_seconds, away,
            "after {k}: the catch-up's own span"
        );
        assert!(!sd.report_so_far.truncated);
        let mut contents = sd.report_so_far.clone();
        contents.elapsed_seconds = 0;
        assert_eq!(
            contents, reference.report,
            "after {k}: contents folded from the ticks replayed"
        );
        assert_eq!(
            sd.state.journal, s.journal,
            "after {k}: nothing recorded in the journal"
        );
    }
}

/// The truncated flag a set-down reports is the catch-up's own, even when the
/// ticks replayed so far are nowhere near the cap.
/// @spec SAVE-020
#[test]
fn a_set_down_carries_the_catch_ups_truncation() {
    let s = played_state(20_002);
    let json = save_string(&s, 1_000);
    let c = begin_load(&json, 1_000 + 2 * CATCHUP_CAP_SECONDS).unwrap();
    let sd = pending(c.advance(10)).set_down();
    assert!(
        sd.report_so_far.truncated,
        "a capped absence is flagged from the start"
    );
    assert_eq!(sd.report_so_far.elapsed_seconds, CATCHUP_CAP_SECONDS);
    assert_eq!(sd.owed_ticks, CATCHUP_CAP_SECONDS * 4 - 10);
}

/// A set-down of a catch-up whose save carried a report so far folds that
/// report in too, so setting down twice loses nothing of the first.
/// @spec SAVE-020
#[test]
fn a_set_down_folds_in_any_report_the_save_carried() {
    let s = famine_state(20_003);
    let carried = carried_report(4_000, false);
    let json = save_string_carrying(&s, 120_000, 0, Some(&carried));
    let sd = advance_to(begin_load(&json, 120_000 + 150).unwrap(), 300, 300).set_down();

    let fresh = advance_to(
        begin_load(&save_string(&s, 120_000), 120_000 + 150).unwrap(),
        300,
        300,
    )
    .set_down()
    .report_so_far;
    let r = &sd.report_so_far;
    assert_eq!(r.elapsed_seconds, 4_000 + 150);
    for res in ResourceId::ALL {
        assert_eq!(
            r.resource_deltas[res.ix()],
            carried.resource_deltas[res.ix()] + fresh.resource_deltas[res.ix()],
            "{res:?}"
        );
    }
    for name in ["Pemberly", "Quillon"] {
        assert_eq!(
            count_of(&r.arrived, name),
            count_of(&carried.arrived, name) + count_of(&fresh.arrived, name)
        );
        assert_eq!(
            count_of(&r.returned, name),
            count_of(&carried.returned, name) + count_of(&fresh.returned, name)
        );
    }
    assert_eq!(
        r.seasons_passed,
        carried.seasons_passed + fresh.seasons_passed
    );
    assert_eq!(sd.state.journal, s.journal, "nothing recorded");
}

/// The point of a set-down: a save carrying its count and report, loaded
/// again, replays the rest and tells the absence once — the same tale, the
/// same meadow, and one journal entry, wherever the wake was interrupted.
/// @spec SAVE-020, SAVE-022
#[test]
fn an_interrupted_catch_up_saved_and_reloaded_tells_the_absence_once() {
    let s = famine_state(20_004);
    let json = save_string(&s, 130_000);
    let now = 130_000 + 600; // 2400 ticks
    let whole = load_string(&json, now).unwrap();
    assert!(
        !whole.report.wandered.is_empty(),
        "fixture: a tale worth keeping whole"
    );

    for k in [1u64, 300, 1_200, 2_399] {
        let sd = advance_to(begin_load(&json, now).unwrap(), k, 500).set_down();
        let resaved = save_string_carrying(&sd.state, now, sd.owed_ticks, Some(&sd.report_so_far));
        let again = load_string(&resaved, now).unwrap();

        assert_same_tale(
            &again.report,
            &whole.report,
            &format!("interrupted after {k}"),
        );
        let mut a = again.state.clone();
        let mut w = whole.state.clone();
        a.journal = Journal::default();
        w.journal = Journal::default();
        assert_eq!(
            state_bytes(&a),
            state_bytes(&w),
            "after {k}: not one owed tick lost or doubled"
        );
        assert_eq!(
            occurrences(&again.state.journal),
            occurrences(&whole.state.journal)
        );
        let entries = away_entries(&again.state.journal);
        assert_eq!(
            entries.len(),
            away_entries(&whole.state.journal).len(),
            "after {k}: told once"
        );
        assert_eq!(
            entries.last().unwrap().1,
            again.report,
            "and the entry is the whole tale"
        );
    }
}

// ---------------------------------------------------------------------
// SAVE-021 — the report so far is canonical state in a newer format
// ---------------------------------------------------------------------

const V4_FIXTURE_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/save_v4.json");

fn v4_fixture() -> String {
    fs::read_to_string(V4_FIXTURE_PATH).unwrap_or_else(|e| {
        panic!("SAVE-013 requires a committed v4 fixture at {V4_FIXTURE_PATH}: {e}")
    })
}

/// A save carries its report so far, or none, beside its owed count.
/// @spec SAVE-021
#[test]
fn a_save_carries_its_report_so_far_or_none() {
    let s = played_state(21_001);
    let r = carried_report(9_000, true);

    let with: serde_json::Value =
        serde_json::from_str(&save_string_carrying(&s, 5_000, 77, Some(&r))).unwrap();
    assert_eq!(with["save_version"], SAVE_VERSION);
    assert_eq!(with["owed_ticks"], 77);
    assert_eq!(
        with["report_so_far"],
        serde_json::to_value(&r).unwrap(),
        "the report, whole"
    );

    let without: serde_json::Value =
        serde_json::from_str(&save_string_carrying(&s, 5_000, 77, None)).unwrap();
    assert!(
        without["report_so_far"].is_null(),
        "none is written as none"
    );
    assert!(
        without.as_object().unwrap().contains_key("report_so_far"),
        "the field is part of the format"
    );
    let plain: serde_json::Value = serde_json::from_str(&save_string(&s, 5_000)).unwrap();
    assert!(
        plain["report_so_far"].is_null(),
        "an ordinary save carries none"
    );

    let dir = scratch_dir("carrying");
    let path = dir.join("save.json");
    write_save_carrying(&path, &s, 5_000, 77, Some(&r)).unwrap();
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        save_string_carrying(&s, 5_000, 77, Some(&r)),
        "the atomic write writes the same save"
    );
}

/// Canonical: a save carrying a report so far, loaded and set down again with
/// nothing replayed and no time passed, writes back byte for byte.
/// @spec SAVE-021
#[test]
fn a_carried_report_round_trips_byte_for_byte() {
    let s = played_state(21_002);
    let r = carried_report(9_000, false);
    let first = save_string_carrying(&s, 6_000, 55, Some(&r));
    let sd = begin_load(&first, 6_000).unwrap().set_down();
    assert_eq!(
        sd.report_so_far, r,
        "the report comes back as it was written"
    );
    assert_eq!(sd.owed_ticks, 55);
    let second = save_string_carrying(&sd.state, 6_000, sd.owed_ticks, Some(&sd.report_so_far));
    assert_eq!(
        first, second,
        "save → load → save is byte-stable with a report carried"
    );
}

/// The report so far arrives under a version newer than any save written
/// without it, so a build that predates it refuses such a save as too new.
/// @spec SAVE-021
#[test]
fn the_report_so_far_arrives_under_a_newer_save_version() {
    let v4: serde_json::Value = serde_json::from_str(&v4_fixture()).unwrap();
    let last_without = v4["save_version"].as_u64().unwrap() as u32;
    assert!(
        SAVE_VERSION > last_without,
        "SAVE_VERSION {SAVE_VERSION} must be newer than {last_without}, the last written without it"
    );
    let s = played_state(21_003);
    let v: serde_json::Value = serde_json::from_str(&save_string_carrying(
        &s,
        1,
        0,
        Some(&carried_report(100, false)),
    ))
    .unwrap();
    assert_eq!(v["save_version"], SAVE_VERSION);
}

/// @spec SAVE-013, SAVE-021
#[test]
fn the_v4_fixture_is_committed_and_predates_the_report_so_far() {
    let v: serde_json::Value = serde_json::from_str(&v4_fixture()).expect("the v4 fixture is JSON");
    assert_eq!(
        v["save_version"], 4,
        "a v4 save, frozen as a historical artifact"
    );
    assert!(
        v.get("report_so_far").is_none(),
        "it predates the report so far entirely"
    );
    assert!(
        v["owed_ticks"].as_u64().is_some_and(|n| n > 0),
        "and it is v4, not a relabelled v3: it carries a nonzero owed count"
    );
    let state = v["state"]
        .as_object()
        .expect("a save carries a state object");
    assert!(state.contains_key("journal"));
    assert!(
        state["ticks"].as_u64().is_some_and(|t| t > 0)
            && !state["kittens"].as_array().unwrap().is_empty(),
        "a played meadow, not a fresh game"
    );
}

/// A v4 save migrates carrying no report so far: its load begins a tale of
/// its own, with its meadow and owed count exactly as written.
/// @spec SAVE-013, SAVE-021
#[test]
fn a_v4_save_migrates_carrying_no_report_so_far() {
    let fixture = v4_fixture();
    let before: serde_json::Value = serde_json::from_str(&fixture).unwrap();
    let saved_at = before["saved_at_unix"].as_u64().unwrap();
    let owed = before["owed_ticks"].as_u64().unwrap();

    let c = begin_load(&fixture, saved_at).expect("a v4 save loads");
    assert_eq!(c.total_ticks(), owed, "the owed count survives migration");
    let sd = c.set_down();
    assert_eq!(
        sd.report_so_far,
        OfflineReport::default(),
        "no report so far was carried"
    );

    let resaved: serde_json::Value = serde_json::from_str(&save_string_carrying(
        &sd.state,
        saved_at,
        sd.owed_ticks,
        None,
    ))
    .unwrap();
    assert_eq!(resaved["save_version"], SAVE_VERSION);
    for (key, was) in before["state"].as_object().unwrap() {
        let now = &resaved["state"][key];
        assert!(
            kept(was, &profiles_set_aside(key, now)),
            "migrating a v4 save changed `{key}`: {was} became {now}"
        );
    }

    let g = finished(
        begin_load(&fixture, saved_at + 30)
            .unwrap()
            .advance(u64::MAX),
    );
    assert_eq!(
        g.report.elapsed_seconds, 30,
        "the tale is this load's alone"
    );
    assert_eq!(
        g.report.seasons_passed,
        load_string(&fixture, saved_at + 30)
            .unwrap()
            .report
            .seasons_passed
    );
}

// ---------------------------------------------------------------------
// SAVE-022 — a carried report is continued, not begun again
// ---------------------------------------------------------------------

/// Every field of the continued report: span summed, truncation either's,
/// deltas the carried plus the replay's against the state as loaded, names
/// merged with counts summed, seasons summed.
/// @spec SAVE-022
#[test]
fn a_carried_report_is_continued_field_by_field() {
    let s = famine_state(22_001);
    let away = 150u64;
    let fresh = load_string(&save_string(&s, 140_000), 140_000 + away).unwrap();
    let wanderer = fresh
        .report
        .wandered
        .first()
        .expect("fixture: someone wanders")
        .0
        .clone();

    let mut carried = carried_report(5_000, false);
    carried.wandered.push((wanderer.clone(), 4));
    let json = save_string_carrying(&s, 140_000, 0, Some(&carried));
    let g = finished(begin_load(&json, 140_000 + away).unwrap().advance(u64::MAX));
    let r = &g.report;

    assert_eq!(
        r.elapsed_seconds,
        5_000 + away,
        "span: the carried span plus this interval"
    );
    assert!(!r.truncated, "neither was truncated");
    for res in ResourceId::ALL {
        assert_eq!(
            r.resource_deltas[res.ix()],
            carried.resource_deltas[res.ix()] + fresh.report.resource_deltas[res.ix()],
            "{res:?}: the carried delta plus the replay's against the state as loaded"
        );
    }
    for (list, c, f) in [
        (&r.arrived, &carried.arrived, &fresh.report.arrived),
        (&r.wandered, &carried.wandered, &fresh.report.wandered),
        (&r.returned, &carried.returned, &fresh.report.returned),
    ] {
        let mut names: Vec<&String> = c.iter().chain(f.iter()).map(|(n, _)| n).collect();
        names.sort();
        names.dedup();
        for name in names {
            assert_eq!(
                count_of(list, name),
                count_of(c, name) + count_of(f, name),
                "{name}"
            );
            assert_eq!(
                list.iter().filter(|(n, _)| n == name).count(),
                1,
                "{name} listed once"
            );
        }
        assert_eq!(list.len(), {
            let mut all: Vec<&String> = c.iter().chain(f.iter()).map(|(n, _)| n).collect();
            all.sort();
            all.dedup();
            all.len()
        });
    }
    assert!(
        count_of(&r.wandered, &wanderer) > 4,
        "a name in both lists has its counts summed"
    );
    assert_eq!(
        r.seasons_passed,
        carried.seasons_passed + fresh.report.seasons_passed
    );
    assert_eq!(
        away_entries(&g.state.journal).last().unwrap().1,
        *r,
        "the continued report is recorded"
    );
}

/// Truncated where either was: a capped year interrupted mid-wake still reads
/// as more than its span after the interruption.
/// @spec SAVE-022
#[test]
fn a_continued_report_is_truncated_where_either_part_was() {
    let s = played_state(22_002);

    let json = save_string_carrying(
        &s,
        150_000,
        0,
        Some(&carried_report(CATCHUP_CAP_SECONDS, true)),
    );
    let g = finished(begin_load(&json, 150_000 + 30).unwrap().advance(u64::MAX));
    assert!(g.report.truncated, "the carried report was truncated");
    assert_eq!(g.report.elapsed_seconds, CATCHUP_CAP_SECONDS + 30);

    let json = save_string_carrying(&s, 1_000, 0, Some(&carried_report(600, false)));
    let sd = pending(
        begin_load(&json, 1_000 + 2 * CATCHUP_CAP_SECONDS)
            .unwrap()
            .advance(4),
    )
    .set_down();
    assert!(
        sd.report_so_far.truncated,
        "this load's interval was truncated"
    );
    assert_eq!(sd.report_so_far.elapsed_seconds, 600 + CATCHUP_CAP_SECONDS);

    let json = save_string_carrying(&s, 160_000, 0, Some(&carried_report(600, false)));
    let g = finished(begin_load(&json, 160_000 + 30).unwrap().advance(u64::MAX));
    assert!(!g.report.truncated, "neither was");
}

/// The minimum-interval rule judges the continued report by its whole span:
/// a relaunch soon after a quit mid-wake still retains the absence.
/// @spec SAVE-022
#[test]
fn retention_judges_a_continued_report_by_its_whole_span() {
    let s = lived_state(22_003);
    let before = away_entries(&s.journal).len();
    let min = MIN_REPORTABLE_ABSENCE_SECONDS;
    for (carried_span, interval, kept) in [
        (min + 3_600, 5, true), // a long absence, relaunched seconds later
        (min - 10, 20, true),   // neither part alone, together enough
        (10, 5, false),         // together still too short
        (min + 3_600, 0, true), // relaunched at the very same second
    ] {
        let json = save_string_carrying(&s, 170_000, 0, Some(&carried_report(carried_span, false)));
        let g = finished(
            begin_load(&json, 170_000 + interval)
                .unwrap()
                .advance(u64::MAX),
        );
        assert_eq!(g.report.elapsed_seconds, carried_span + interval);
        let entries = away_entries(&g.state.journal);
        if kept {
            assert_eq!(
                entries.len(),
                before + 1,
                "carried {carried_span}s + {interval}s: retained"
            );
            assert_eq!(entries.last().unwrap().1, g.report);
        } else {
            assert_eq!(
                entries.len(),
                before,
                "carried {carried_span}s + {interval}s: too short"
            );
        }
    }
}

/// Relaunching later than the quit: the finished report is the uninterrupted
/// one extended by the time between the quit and the load that finishes it.
/// @spec SAVE-022
#[test]
fn a_later_relaunch_extends_the_tale_by_the_time_between() {
    let s = famine_state(22_004);
    let json = save_string(&s, 180_000);
    let quit_at = 180_000 + 600;
    let sd = advance_to(begin_load(&json, quit_at).unwrap(), 900, 900).set_down();
    let resaved = save_string_carrying(&sd.state, quit_at, sd.owed_ticks, Some(&sd.report_so_far));
    let g = finished(
        begin_load(&resaved, quit_at + 45)
            .unwrap()
            .advance(u64::MAX),
    );
    assert_eq!(g.report.elapsed_seconds, 600 + 45);
    assert_eq!(
        g.state.ticks,
        s.ticks + 2_400 + 180,
        "every owed tick, plus the new interval's"
    );
    assert_eq!(
        away_entries(&g.state.journal).len(),
        away_entries(&s.journal).len() + 1
    );
}

// =====================================================================
// Save format v6 — the revealed resources become persisted state
// (SAVE-013, SAVE-023, SAVE-024)
//
// The engine keeps the set of resources its meadow has revealed
// (SIM-ENG-051), and a save carries it: that a store was once held and has
// since been spent to nothing can be recomputed from nothing else. A save
// written before the set existed is not given an empty one. Its migration
// applies the engine's own reveal rule once, to the state as loaded and
// normalized and before any catch-up tick: every resource the save holds,
// every resource named in a price the core would state for it, and no other.
//
// What is asserted here is the set as *persisted* — the `revealed_resources`
// array of the state a load hands back, re-serialized — read through
// `common::recorded_resources`. The rule it is checked against is assembled
// from the core's own queries in `common::held_resources` and
// `common::priced_resources`.
// =====================================================================

const V5_FIXTURE_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/save_v5.json");

fn v5_fixture() -> String {
    fs::read_to_string(V5_FIXTURE_PATH).unwrap_or_else(|e| {
        panic!("SAVE-013 requires a committed v5 fixture at {V5_FIXTURE_PATH}: {e}")
    })
}

/// The committed fixture of every version before the revealed-resource set.
fn fixtures_without_the_set() -> Vec<(u32, String)> {
    vec![
        (1, include_str!("../fixtures/save_v1.json").to_string()),
        (2, v2_fixture()),
        (3, v3_fixture()),
        (4, v4_fixture()),
        (5, v5_fixture()),
    ]
}

fn saved_at(save: &serde_json::Value) -> u64 {
    save["saved_at_unix"]
        .as_u64()
        .expect("a save stamps its save time")
}

/// A save as load hands it back before any catch-up tick: parsed, migrated,
/// and normalized, its owed ticks still owed.
fn loaded_unreplayed(json: &str, now_unix: u64) -> GameState {
    begin_load(json, now_unix)
        .expect("the save loads")
        .set_down()
        .state
}

/// The stores a build without the set listed for a save: exactly those whose
/// stock, as written, stands above zero.
fn stores_a_prior_version_listed(save: &serde_json::Value) -> BTreeSet<ResourceId> {
    save["state"]["resources"]
        .as_array()
        .expect("a save carries its stocks")
        .iter()
        .enumerate()
        .filter(|(_, v)| v.as_f64().is_some_and(|x| x > 0.0))
        .map(|(i, _)| ResourceId::ALL[i])
        .collect()
}

/// A v5 save reduced to a bare meadow: nothing built, nothing learnt, nobody
/// met, nothing in store, every kitten home with no work, no ticks owed — so
/// that no price is in view and whatever is revealed was revealed by a stock.
fn bare_v5_save() -> serde_json::Value {
    let mut v: serde_json::Value = serde_json::from_str(&v5_fixture()).unwrap();
    v["owed_ticks"] = serde_json::json!(0);
    let state = &mut v["state"];
    for key in ["structures", "resources"] {
        for x in state[key].as_array_mut().unwrap() {
            *x = serde_json::json!(0);
        }
    }
    for key in ["ideas", "revealed", "discovered", "refinements", "rules"] {
        state[key] = serde_json::json!([]);
    }
    for k in state["kittens"].as_array_mut().unwrap() {
        k["job"] = serde_json::Value::Null;
        k["job_seq"] = serde_json::Value::Null;
        k["location"] = serde_json::json!("Home");
        k["away_seq"] = serde_json::Value::Null;
        k["tinkering"] = serde_json::Value::Null;
    }
    let bare = loaded_unreplayed(&v.to_string(), saved_at(&v));
    assert!(
        priced_resources(&bare).is_empty() && held_resources(&bare).is_empty(),
        "fixture: the bare save should hold nothing and show no price"
    );
    assert!(!bare.kittens.is_empty(), "fixture: somebody is home to eat");
    v
}

/// @spec SAVE-013, SAVE-024
#[test]
fn the_v5_fixture_is_committed_and_predates_the_revealed_resources() {
    let v: serde_json::Value = serde_json::from_str(&v5_fixture()).expect("the v5 fixture is JSON");
    assert_eq!(
        v["save_version"], 5,
        "a v5 save, frozen as a historical artifact"
    );
    let state = v["state"]
        .as_object()
        .expect("a save carries a state object");
    assert!(
        !state.contains_key("revealed_resources"),
        "it predates the revealed-resource set entirely — that is what makes it the migration case"
    );
    assert!(
        v["report_so_far"].is_object() && v["owed_ticks"].as_u64().is_some_and(|n| n > 0),
        "and it is v5, not a relabelled v4: it carries a report so far beside its owed count"
    );
    assert!(
        state["ticks"].as_u64().is_some_and(|t| t > 0)
            && !state["kittens"].as_array().unwrap().is_empty()
            && state.contains_key("journal"),
        "a played meadow, not a fresh game"
    );

    // And it holds each case the migration's rule has to tell apart.
    let loaded = loaded_unreplayed(&v5_fixture(), saved_at(&v));
    let held = held_resources(&loaded);
    let priced = priced_resources(&loaded);
    assert!(
        held.difference(&priced).next().is_some(),
        "fixture: a store held that no stated price names — revealed by its stock alone"
    );
    assert!(
        priced.difference(&held).next().is_some(),
        "fixture: a store standing at nothing that a stated price names — met, and spent"
    );
    assert!(
        ResourceId::ALL
            .iter()
            .any(|r| !held.contains(r) && !priced.contains(r)),
        "fixture: a store neither held nor named — never met"
    );
}

/// The set arrives under a version newer than any save written without it,
/// so a build that predates it refuses such a save as too new rather than
/// loading it and writing it back with the set dropped.
/// @spec SAVE-023
#[test]
fn the_revealed_resources_arrive_under_a_newer_save_version() {
    let v5: serde_json::Value = serde_json::from_str(&v5_fixture()).unwrap();
    let last_without = v5["save_version"].as_u64().unwrap() as u32;
    assert!(
        SAVE_VERSION > last_without,
        "SAVE_VERSION {SAVE_VERSION} must be newer than {last_without}, the last written without \
         the revealed-resource set"
    );
    // The set is part of the format: written even where nothing is revealed.
    let v: serde_json::Value =
        serde_json::from_str(&save_string(&GameState::new(23_001), 1)).unwrap();
    assert_eq!(v["save_version"], SAVE_VERSION);
    assert_eq!(
        v["state"].get("revealed_resources"),
        Some(&serde_json::json!([])),
        "a new meadow's save carries the set, empty"
    );
    // And a build one version back, shown such a save, is shown a version it
    // does not know: the refusal SAVE-002 already makes.
    let mut future = v.clone();
    future["save_version"] = serde_json::json!(SAVE_VERSION + 1);
    assert!(matches!(
        load_string(&future.to_string(), 1),
        Err(LoadError::TooNew { found }) if found == SAVE_VERSION + 1
    ));
}

/// A reveal survives save and load. The save carries the set as canonical
/// state: a store held, spent to exactly nothing, and named by no price is
/// still revealed after the round trip — through the string and through the
/// file — and the round trip is byte-stable with the set in it.
/// @spec SAVE-023, SAVE-010
#[test]
fn a_reveal_survives_save_and_load() {
    let mut s = GameState::new(23_002);
    for _ in 0..10 {
        s.apply(Command::GatherCatnip).unwrap();
    }
    let spent: Vec<ResourceId> = held_resources(&s).into_iter().collect();
    s.apply(Command::Forage)
        .expect("fixture: ten gathers pay for a forage");
    run_ticks(&mut s, 30);
    assert!(!spent.is_empty(), "fixture: something was gathered");
    for r in &spent {
        assert_eq!(
            s.stock(*r),
            0.0,
            "fixture: {r:?} was spent to exactly nothing"
        );
        assert!(
            !priced_resources(&s).contains(r),
            "fixture: no stated price names {r:?}, so nothing but the set remembers it"
        );
    }
    let before = recorded_resources(&s);
    for r in &spent {
        assert!(
            before.contains(r),
            "{r:?} was held and spent, and the meadow's save does not carry it as revealed"
        );
    }

    let json = save_string(&s, 77_000);
    let loaded = load_string(&json, 77_000).expect("the save loads").state;
    assert_eq!(
        recorded_resources(&loaded),
        before,
        "the revealed resources changed across save and load"
    );
    assert_eq!(
        save_string(&loaded, 77_000),
        json,
        "save → load → save is byte-stable with the set in it"
    );

    let dir = scratch_dir("revealed-resources");
    let path = dir.join("save.json");
    write_save(&path, &s, 77_000).unwrap();
    let mut from_file = read_save(&path, 77_000).expect("the file loads").state;
    assert_eq!(
        recorded_resources(&from_file),
        before,
        "the revealed resources changed across the file"
    );

    // Still revealed as the reloaded meadow plays on with none in store.
    run_ticks(&mut from_file, 50);
    for r in &spent {
        assert_eq!(from_file.stock(*r), 0.0, "fixture: still none of {r:?}");
        assert!(
            recorded_resources(&from_file).contains(r),
            "{r:?} un-revealed after a reload"
        );
    }
}

/// A catch-up replays with the set the save carried: a store revealed and
/// empty at the save is revealed when the absence is over.
/// @spec SAVE-023
#[test]
fn a_reveal_survives_an_absence() {
    let mut s = GameState::new(23_003);
    for _ in 0..10 {
        s.apply(Command::GatherCatnip).unwrap();
    }
    let spent: Vec<ResourceId> = held_resources(&s).into_iter().collect();
    s.apply(Command::Forage).unwrap();
    let json = save_string(&s, 5_000);

    let woken = load_string(&json, 5_000 + 3_600)
        .expect("the save loads")
        .state;

    assert!(woken.ticks > s.ticks, "fixture: an hour was replayed");
    for r in &spent {
        assert_eq!(
            woken.stock(*r),
            0.0,
            "fixture: nothing refilled {r:?} while away"
        );
        assert!(
            recorded_resources(&woken).contains(r),
            "{r:?} un-revealed across an absence"
        );
    }
}

/// A v5 save migrates with its meadow, its owed count, and its report so far
/// exactly as written, and gains one thing: the revealed-resource set.
/// @spec SAVE-013
#[test]
fn a_v5_save_migrates_with_everything_else_exact() {
    let fixture = v5_fixture();
    let before: serde_json::Value = serde_json::from_str(&fixture).unwrap();
    let at = saved_at(&before);

    let c = begin_load(&fixture, at).expect("a v5 save loads");
    assert_eq!(
        c.total_ticks(),
        before["owed_ticks"].as_u64().unwrap(),
        "the owed count survives migration"
    );
    let sd = c.set_down();
    let mut resaved: serde_json::Value = serde_json::from_str(&save_string_carrying(
        &sd.state,
        at,
        sd.owed_ticks,
        Some(&sd.report_so_far),
    ))
    .unwrap();
    take_empty_record(&mut resaved);

    assert_eq!(
        resaved["save_version"], SAVE_VERSION,
        "migrated forward, then stamped current"
    );
    assert!(json_eq(&before["owed_ticks"], &resaved["owed_ticks"]));
    assert!(
        json_eq(&before["report_so_far"], &resaved["report_so_far"]),
        "the report so far changed across migration"
    );
    for (key, was) in before["state"].as_object().unwrap() {
        let now = &resaved["state"][key];
        assert!(
            kept(was, &profiles_set_aside(key, now)),
            "migrating a v5 save changed `{key}`: {was} became {now}"
        );
    }
    let added: Vec<&String> = resaved["state"]
        .as_object()
        .unwrap()
        .keys()
        .filter(|k| !before["state"].as_object().unwrap().contains_key(*k))
        .collect();
    assert_eq!(
        added,
        vec!["learned_goods", "revealed_resources"],
        "a v5 save gains the revealed-resource set, and through v7 what each colony has shown"
    );
    assert!(
        resaved["state"]["learned_goods"]
            .as_object()
            .is_some_and(|m| m.is_empty()),
        "what colonies have shown arrives empty (SAVE-026)"
    );
}

/// The migration fills the set by the engine's reveal rule, applied once to
/// the save's state as loaded: every resource it holds, every resource named
/// in a price the core would state for it, and no other.
/// @spec SAVE-024
#[test]
fn a_v5_save_migrates_revealing_what_it_holds_and_what_its_prices_name_and_no_other() {
    let fixture = v5_fixture();
    let before: serde_json::Value = serde_json::from_str(&fixture).unwrap();
    let state = loaded_unreplayed(&fixture, saved_at(&before));
    assert_eq!(
        state.ticks,
        before["state"]["ticks"].as_u64().unwrap(),
        "fixture: no tick was replayed"
    );

    let held = held_resources(&state);
    let priced = priced_resources(&state);
    let recorded = recorded_set(&state);

    for r in &held {
        assert!(
            recorded.contains(r),
            "the save holds {r:?} and its migration did not reveal it"
        );
    }
    for r in &priced {
        assert!(
            recorded.contains(r),
            "a price the core states for the save names {r:?} (stock {}) and its migration did \
             not reveal it — the save's evidence for a store met and since spent",
            state.stock(*r)
        );
    }
    for r in &recorded {
        assert!(
            held.contains(r) || priced.contains(r),
            "the migration revealed {r:?}, which the save neither holds nor shows in a price"
        );
    }
    // The set is written as the engine writes it: in id order, each once.
    let written = recorded_resources(&state);
    let mut sorted = written.clone();
    sorted.sort_by_key(|r| r.ix());
    sorted.dedup();
    assert_eq!(written, sorted);
}

/// Every prior version's fixture migrates with every store that version
/// listed for it still listed — a prior version listed exactly the stores
/// with a stock above zero — and with the set the rule gives its state and
/// no other.
/// @spec SAVE-013, SAVE-024
#[test]
fn every_prior_version_migrates_with_every_store_it_listed_still_listed() {
    let mut listed_somewhere = 0;
    for (version, fixture) in fixtures_without_the_set() {
        let before: serde_json::Value = serde_json::from_str(&fixture).unwrap();
        assert_eq!(
            before["save_version"], version,
            "fixture: the v{version} fixture is a v{version} save"
        );
        assert!(
            before["state"].get("revealed_resources").is_none(),
            "fixture: v{version} predates the set"
        );
        let state = loaded_unreplayed(&fixture, saved_at(&before));
        let recorded = recorded_set(&state);

        let listed = stores_a_prior_version_listed(&before);
        listed_somewhere += listed.len();
        for r in &listed {
            assert!(
                recorded.contains(r),
                "the v{version} fixture holds {r:?}, which its own version listed, and migration \
                 left it unrevealed: migration must hide nothing its player could see"
            );
        }
        assert_eq!(
            recorded,
            held_resources(&state)
                .union(&priced_resources(&state))
                .copied()
                .collect(),
            "the v{version} fixture's migrated set is not its holdings and the resources its \
             stated prices name (left: migrated, right: the rule)"
        );

        // And nothing is hidden once the catch-up has run either.
        let caught_up = load_string(&fixture, saved_at(&before) + 120)
            .expect("the fixture loads")
            .state;
        let after = recorded_set(&caught_up);
        for r in &recorded {
            assert!(
                after.contains(r),
                "v{version}: {r:?} was revealed by migration and lost in the catch-up"
            );
        }
    }
    assert!(listed_somewhere > 0, "fixture: the prior saves list stores");
}

/// The rule runs before any catch-up tick: a store the first replayed tick
/// empties is revealed all the same, because the save held some.
/// @spec SAVE-024
#[test]
fn migration_reveals_a_store_the_first_replayed_tick_empties() {
    let mut v = bare_v5_save();
    let morsel = ResourceId::Catnip;
    v["state"]["resources"][morsel.ix()] = serde_json::json!(0.05);
    let json = v.to_string();
    let at = saved_at(&v);

    let unreplayed = loaded_unreplayed(&json, at + 60);
    assert_eq!(
        held_resources(&unreplayed),
        [morsel].into_iter().collect(),
        "fixture: the save holds a morsel"
    );
    assert_eq!(
        recorded_set(&unreplayed),
        [morsel].into_iter().collect(),
        "the migration should reveal exactly what the bare save holds, before any tick"
    );

    let c = begin_load(&json, at + 60).unwrap();
    assert!(c.total_ticks() > 0, "fixture: an absence to replay");
    let one_tick_in = match c.advance(1) {
        Advance::Pending(c) => c.set_down().state,
        Advance::Finished(g) => g.state,
    };
    assert_eq!(
        one_tick_in.stock(morsel),
        0.0,
        "fixture: the first replayed tick eats the morsel"
    );
    assert!(
        recorded_set(&one_tick_in).contains(&morsel),
        "{morsel:?} was in the save and gone after the first replayed tick, and was never \
         revealed: the migration must record what the save holds before any catch-up tick"
    );

    let woken = load_string(&json, at + 60).unwrap().state;
    assert_eq!(
        woken.stock(morsel),
        0.0,
        "fixture: still none when the absence is over"
    );
    assert!(
        recorded_set(&woken).contains(&morsel),
        "{morsel:?} was lost by the end of the catch-up"
    );
}

/// The rule reads the state as normalized: a hand-edited stock below zero is
/// a zero, a stock in a store with no room is clamped away, and neither is a
/// holding; a stock above its cap is clamped to it and still is one.
/// @spec SAVE-024
#[test]
fn migration_judges_holdings_on_the_state_as_normalized() {
    let bare = bare_v5_save();
    let at = saved_at(&bare);
    let meadow = loaded_unreplayed(&bare.to_string(), at);
    let roomy: Vec<ResourceId> = ResourceId::ALL
        .into_iter()
        .filter(|r| meadow.cap(*r) > 0.0 && meadow.cap(*r).is_finite())
        .collect();
    let no_room: Vec<ResourceId> = ResourceId::ALL
        .into_iter()
        .filter(|r| meadow.cap(*r) == 0.0)
        .collect();
    assert!(
        roomy.len() >= 3 && !no_room.is_empty(),
        "fixture: stores with room and without"
    );
    let (negative, plain, brimming, roomless) = (roomy[0], roomy[1], roomy[2], no_room[0]);

    let mut v = bare.clone();
    v["state"]["resources"][negative.ix()] = serde_json::json!(-5.0);
    v["state"]["resources"][plain.ix()] = serde_json::json!(2.0);
    v["state"]["resources"][brimming.ix()] = serde_json::json!(1.0e12);
    v["state"]["resources"][roomless.ix()] = serde_json::json!(7.0);

    let state = loaded_unreplayed(&v.to_string(), at);
    assert_eq!(
        state.stock(negative),
        0.0,
        "fixture: a negative stock normalizes to zero"
    );
    assert_eq!(
        state.stock(roomless),
        0.0,
        "fixture: a stock with no room is clamped away"
    );
    assert_eq!(state.stock(plain), 2.0);
    assert_eq!(
        state.stock(brimming),
        state.cap(brimming),
        "fixture: a stock above its cap is clamped to it"
    );

    assert_eq!(
        recorded_set(&state),
        [plain, brimming].into_iter().collect::<BTreeSet<_>>(),
        "the migration should reveal the two stores the normalized save holds, and neither the \
         negative stock nor the one its store had no room for"
    );
}

/// A save as text with one state field edited by hand.
fn with_state_field(s: &GameState, key: &str, value: serde_json::Value) -> String {
    let mut v: serde_json::Value = serde_json::from_str(&save_string(s, 1_000)).unwrap();
    v["state"][key] = value;
    v.to_string()
}

/// A hand-edited wisdom below zero is floored before any cap reads it. Read first,
/// it would drive every cap negative and zero every stock, uncapped ones included.
/// @spec SAVE-005
#[test]
fn a_negative_wisdom_is_floored_before_any_cap_reads_it() {
    let mut s = GameState::new(30_032);
    s.resources[ResourceId::Catnip.ix()] = 150.0;
    s.resources[ResourceId::Feathers.ix()] = 7.0;
    for key in ["elder_wisdom", "passed_down_wisdom"] {
        let json = with_state_field(&s, key, serde_json::json!(-100_000.0));
        let state = load_string(&json, 1_000).unwrap().state;
        assert_eq!(state.stock(ResourceId::Catnip), 150.0, "{key}");
        assert_eq!(state.stock(ResourceId::Feathers), 7.0, "{key}");
    }
}

/// A version past 32 bits is newer than anything this binary knows, and can't wrap
/// round to a version it does.
/// @spec SAVE-002
#[test]
fn a_version_past_32_bits_is_too_new_rather_than_wrapped_round() {
    let mut v: serde_json::Value =
        serde_json::from_str(&save_string(&GameState::new(30_033), 1_000)).unwrap();
    v["save_version"] = serde_json::json!((1u64 << 32) + SAVE_VERSION as u64);
    assert!(
        matches!(
            load_string(&v.to_string(), 1_000),
            Err(LoadError::TooNew { found: u32::MAX })
        ),
        "should read as too new"
    );
}

/// A count too large to add to doesn't overflow.
/// @spec SAVE-005
#[test]
fn an_owed_count_too_large_to_add_to_saturates() {
    let mut v: serde_json::Value =
        serde_json::from_str(&save_string(&GameState::new(30_034), 1_000)).unwrap();
    v["owed_ticks"] = serde_json::json!(u64::MAX);
    let catch_up = begin_load(&v.to_string(), 2_000).expect("it loads");
    assert_eq!(catch_up.total_ticks(), u64::MAX);
}

/// A backwards clock replays nothing for the interval, and what the save owes was
/// earned already, so it is still replayed.
/// @spec SAVE-007, SAVE-017
#[test]
fn a_backwards_clock_still_replays_what_the_save_owes() {
    let s = GameState::new(30_035);
    let owed = save_string_owing(&s, 1_000, 123);
    assert_eq!(begin_load(&owed, 500).unwrap().total_ticks(), 123);
    let none = save_string(&s, 1_000);
    assert_eq!(begin_load(&none, 500).unwrap().total_ticks(), 0);
}

// =====================================================================
// A hand edit gets physics: every integer in the save, wherever it sits
// =====================================================================

type JsonPath = Vec<serde_json::Value>;

/// The path of every integer in a saved value.
fn integer_paths(value: &serde_json::Value) -> Vec<JsonPath> {
    fn walk(value: &serde_json::Value, path: &mut JsonPath, found: &mut Vec<JsonPath>) {
        match value {
            serde_json::Value::Object(fields) => {
                for (key, field) in fields {
                    path.push(serde_json::json!(key));
                    walk(field, path, found);
                    path.pop();
                }
            }
            serde_json::Value::Array(items) => {
                for (ix, item) in items.iter().enumerate() {
                    path.push(serde_json::json!(ix));
                    walk(item, path, found);
                    path.pop();
                }
            }
            serde_json::Value::Number(n) if n.is_u64() => found.push(path.clone()),
            _ => {}
        }
    }
    let mut found = Vec::new();
    walk(value, &mut Vec::new(), &mut found);
    found
}

fn at<'a>(
    value: &'a mut serde_json::Value,
    path: &[serde_json::Value],
) -> &'a mut serde_json::Value {
    path.iter().fold(value, |here, step| match step {
        serde_json::Value::String(key) => &mut here[key.as_str()],
        _ => &mut here[step.as_u64().unwrap() as usize],
    })
}

fn spelled(path: &[serde_json::Value]) -> String {
    path.iter()
        .map(|step| match step {
            serde_json::Value::String(key) => key.clone(),
            _ => "*".to_string(),
        })
        .collect::<Vec<_>>()
        .join(".")
}

/// A saved meadow with an integer in every field that holds one, and a tale set
/// down mid-replay carried beside it: one path per field, except that every
/// place of an array indexed by content keeps its own, since what a count does
/// depends on what it counts.
fn every_integer_held() -> (serde_json::Value, Vec<JsonPath>) {
    let mut s = late_meadow();
    s.pity = 1;
    s.famine_days = 1;
    s.recovery_days = 1;
    s.next_seq = 9;
    s.zoomies_until = 3;
    s.jamboree_until = 3;
    s.deja_vu_until = 2;
    s.pending_sky_event = Some(3);
    s.roster_high_water = 5;
    s.penpal = [2; ColonyId::ALL.len()];
    s.adventure_completions = [1; AdventureId::ALL.len()];
    s.whale_visit_years = 2;
    s.whale_contentment = 2;
    s.kittens[1].location = Location::NeighborsYard;
    s.kittens[1].away_seq = Some(3);
    s.family_album.push(ArchivedKitten {
        name: "Remembered".to_string(),
        trait_id: KittenTrait::None,
        rank: 1,
        generation: 1,
        profile: None,
    });
    s.achievements.insert(
        AchievementId::ALL[0],
        Earned {
            generation: 1,
            day: 2,
        },
    );
    for event in [
        Event::NewYear { year: 2 },
        Event::StructureBuilt {
            id: StructureId::CatnipPatch,
            count: 2,
        },
        Event::ZoomiesStarted { until_tick: 9 },
        Event::Flavor {
            template: 0,
            kitten: None,
        },
    ] {
        s.journal.record(5, &event);
    }
    let away = OfflineReport {
        elapsed_seconds: 90,
        seasons_passed: 1,
        arrived: vec![("Newcomer".to_string(), 2)],
        ..OfflineReport::default()
    };
    s.journal.record_away(6, &away);
    let json = save_string_carrying(&s, 1_000, 3, Some(&away));
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    let mut shapes = std::collections::BTreeMap::new();
    for path in integer_paths(&value) {
        let shape = spelled(&path);
        let by_place = [
            "state.structures.*",
            "state.rite_counts.*",
            "state.penpal.*",
            "state.adventure_completions.*",
        ]
        .contains(&shape.as_str());
        let key = if by_place {
            format!("{shape}{}", path.last().unwrap())
        } else {
            shape
        };
        shapes.entry(key).or_insert(path);
    }
    (value, shapes.into_values().collect())
}

/// Load a doctored save, replay a little of what it owes, and play on from
/// there, as a game would: each of these runs with overflow checks on.
fn load_and_play(doctored: &serde_json::Value) -> Result<GameState, LoadError> {
    let mut state = match begin_load(&doctored.to_string(), 1_000)?.advance(30) {
        Advance::Finished(game) => game.state,
        Advance::Pending(catch_up) => catch_up.set_down().state,
    };
    for _ in 0..25 {
        state.tick();
    }
    state.housing_capacity();
    state.happiness();
    state.energy_consumption();
    for r in ResourceId::ALL {
        state.cap(r);
    }
    Ok(state)
}

/// Reads an integer back after a load, by the same path it was edited at.
fn read_back(state: &GameState, path: &[serde_json::Value]) -> serde_json::Value {
    let mut saved: serde_json::Value = serde_json::from_str(&save_string(state, 0)).unwrap();
    at(&mut saved, path).take()
}

/// No integer's bad value costs the save, wherever in the save it sits, and
/// the meadow plays on from there. Walked rather than listed, so a field added
/// later is held to this without anyone adding it here.
// @spec SAVE-032, SAVE-035
#[test]
fn no_number_in_an_integer_field_costs_the_save() {
    let (saved, paths) = every_integer_held();
    assert!(
        paths.len() > 100,
        "the walk reaches the save's integers, found {}",
        paths.len()
    );
    for path in paths {
        if spelled(&path) == "save_version" {
            continue;
        }
        for bad in [
            serde_json::json!(-1),
            serde_json::json!(2.5),
            serde_json::json!(1e30),
            serde_json::json!(u64::MAX),
        ] {
            let mut doctored = saved.clone();
            *at(&mut doctored, &path) = bad.clone();
            if let Err(e) = load_and_play(&doctored) {
                panic!("{} at {bad} refuses the save: {e:?}", spelled(&path));
            }
        }
    }
}

/// What each kind of bad number reads as, read back from the loaded state.
// @spec SAVE-032
#[test]
fn a_bad_number_reads_as_the_nearest_the_field_can_hold() {
    let saved: serde_json::Value =
        serde_json::from_str(&save_string(&played_state(36), 1_000)).unwrap();
    let cases = [
        // A count of each width.
        ("state.pity", serde_json::json!(-1), 0u64),
        ("state.pity", serde_json::json!(-0.5), 0),
        ("state.pity", serde_json::json!(-1e30), 0),
        ("state.pity", serde_json::json!(2.5), 2),
        ("state.pity", serde_json::json!(300), 255),
        ("state.pity", serde_json::json!(1e30), 255),
        ("state.generation", serde_json::json!(-7), 0),
        ("state.generation", serde_json::json!(2.9), 2),
        ("state.structures.0", serde_json::json!(-3), 0),
        ("state.structures.0", serde_json::json!(4.5), 4),
        ("state.next_seq", serde_json::json!(-1), 0),
        ("state.next_seq", serde_json::json!(12.75), 12),
        // Reached through a list and an optional field.
        ("state.kittens.0.rank", serde_json::json!(-2), 0),
        ("state.kittens.0.arrived_day", serde_json::json!(-2), 0),
        ("state.kittens.0.job_seq", serde_json::json!(-2), 0),
    ];
    for (field, bad, expected) in cases {
        let path: JsonPath = field
            .split('.')
            .map(|step| match step.parse::<u64>() {
                Ok(ix) => serde_json::json!(ix),
                Err(_) => serde_json::json!(step),
            })
            .collect();
        let mut doctored = saved.clone();
        *at(&mut doctored, &path) = bad.clone();
        let loaded = begin_load(&doctored.to_string(), 1_000)
            .unwrap_or_else(|e| panic!("{field} at {bad} refuses the save: {e:?}"))
            .set_down()
            .state;
        assert_eq!(
            read_back(&loaded, &path),
            serde_json::json!(expected),
            "{field} at {bad}"
        );
    }
}

/// The same reading reaches the file's own fields, the journal, and a report.
// @spec SAVE-032
#[test]
fn the_reading_reaches_the_file_the_journal_and_the_report() {
    let (saved, _) = every_integer_held();

    let mut doctored = saved.clone();
    doctored["owed_ticks"] = serde_json::json!(-5);
    doctored["saved_at_unix"] = serde_json::json!(1_000.5);
    let catch_up = begin_load(&doctored.to_string(), 1_000).expect("it loads");
    assert_eq!(catch_up.total_ticks(), 0, "nothing owed, and no time away");

    let mut doctored = saved.clone();
    doctored["state"]["journal"]["recent"][0]["tick"] = serde_json::json!(-1);
    doctored["report_so_far"]["seasons_passed"] = serde_json::json!(-1);
    doctored["report_so_far"]["arrived"][0][1] = serde_json::json!(2.5);
    let set_down = begin_load(&doctored.to_string(), 1_000)
        .expect("it loads")
        .set_down();
    assert_eq!(set_down.state.journal.recent[0].tick, 0);
    assert_eq!(set_down.report_so_far.seasons_passed, 0);
    assert_eq!(set_down.report_so_far.arrived[0].1, 2);
}

/// A carried tale read at the top of its range still adds to what this load
/// tells, rather than overflowing.
// @spec SAVE-022, SAVE-032
#[test]
fn a_carried_tale_at_the_top_of_its_range_still_adds() {
    let (saved, _) = every_integer_held();
    let mut doctored = saved.clone();
    doctored["report_so_far"]["elapsed_seconds"] = serde_json::json!(1e30);
    doctored["report_so_far"]["seasons_passed"] = serde_json::json!(1e30);
    doctored["report_so_far"]["arrived"][0][1] = serde_json::json!(1e30);
    doctored["owed_ticks"] = serde_json::json!(0);
    let Advance::Finished(game) = begin_load(&doctored.to_string(), 1_010)
        .unwrap()
        .advance(u64::MAX)
    else {
        panic!("advancing through everything finishes");
    };
    assert_eq!(game.report.elapsed_seconds, u64::MAX);
    assert_eq!(game.report.seasons_passed, u32::MAX);
    assert_eq!(game.report.arrived[0].1, u32::MAX);
}

/// Inside an event in the journal too, a struct within an enum within a list.
// @spec SAVE-032
#[test]
fn the_reading_reaches_inside_a_journaled_event() {
    let mut s = GameState::new(36_002);
    s.journal.record(5, &Event::NewYear { year: 2 });
    let mut doctored: serde_json::Value = serde_json::from_str(&save_string(&s, 1_000)).unwrap();
    doctored["state"]["journal"]["recent"][0]["item"]["Happened"]["NewYear"]["year"] =
        serde_json::json!(-4);
    let loaded = begin_load(&doctored.to_string(), 1_000)
        .unwrap()
        .set_down()
        .state;
    assert_eq!(
        loaded.journal.recent[0].item,
        JournalItem::Happened(Event::NewYear { year: 0 })
    );
}

/// A version names a format, so it gets no nearest reading.
// @spec SAVE-002, SAVE-032
#[test]
fn a_version_that_is_not_a_whole_number_is_corrupt() {
    let saved: serde_json::Value =
        serde_json::from_str(&save_string(&played_state(36), 1_000)).unwrap();
    for bad in [serde_json::json!(-1), serde_json::json!(2.5)] {
        let mut doctored = saved.clone();
        doctored["save_version"] = bad.clone();
        assert!(
            matches!(
                begin_load(&doctored.to_string(), 1_000),
                Err(LoadError::Corrupt { .. })
            ),
            "a version of {bad}"
        );
    }
}

/// A string has no nearest integer, and an integer field holding one is still
/// corrupt: the reading is for a mistyped number, not for anything at all.
// @spec SAVE-004, SAVE-032
#[test]
fn something_that_is_not_a_number_where_one_belongs_is_still_corrupt() {
    let saved: serde_json::Value =
        serde_json::from_str(&save_string(&played_state(36), 1_000)).unwrap();
    for (field, bad) in [
        ("pity", serde_json::json!("three")),
        ("next_seq", serde_json::json!(null)),
        ("ticks", serde_json::json!([1])),
        ("generation", serde_json::json!({ "n": 1 })),
    ] {
        let mut doctored = saved.clone();
        doctored["state"][field] = bad.clone();
        assert!(
            matches!(
                begin_load(&doctored.to_string(), 1_000),
                Err(LoadError::Corrupt { .. })
            ),
            "{field} holding {bad}"
        );
    }
}

/// A negative Elder index is no Elder, not the first kitten in the chair, and
/// one past the roster is no Elder too.
// @spec SAVE-034
#[test]
fn a_negative_elder_is_no_elder() {
    let saved: serde_json::Value =
        serde_json::from_str(&save_string(&played_state(36), 1_000)).unwrap();
    for bad in [
        serde_json::json!(-1),
        serde_json::json!(-0.5),
        serde_json::json!(99),
    ] {
        let mut doctored = saved.clone();
        doctored["state"]["elder"] = bad.clone();
        let loaded = begin_load(&doctored.to_string(), 1_000)
            .unwrap_or_else(|e| panic!("an Elder index of {bad} refuses the save: {e:?}"))
            .set_down()
            .state;
        assert_eq!(loaded.elder, None, "an Elder index of {bad}");
    }
    let mut doctored = saved.clone();
    doctored["state"]["elder"] = serde_json::json!(1.5);
    let loaded = begin_load(&doctored.to_string(), 1_000)
        .unwrap()
        .set_down()
        .state;
    assert_eq!(
        loaded.elder,
        Some(1),
        "a fraction rounds down as any integer does"
    );
}

/// A number too large leaves room for what adds to it, and a price never reads a
/// negative count of copies.
// @spec SAVE-035
#[test]
fn what_keeps_counting_keeps_room() {
    let saved: serde_json::Value =
        serde_json::from_str(&save_string(&played_state(36), 1_000)).unwrap();
    let mut doctored = saved.clone();
    let huge = serde_json::json!(1e30);
    for field in [
        "ticks",
        "next_seq",
        "zoomies_until",
        "jamboree_until",
        "deja_vu_until",
    ] {
        doctored["state"][field] = huge.clone();
    }
    doctored["state"]["pending_sky"] = huge.clone();
    doctored["state"]["kittens"][0]["job_seq"] = huge.clone();
    doctored["state"]["kittens"][1]["away_seq"] = huge.clone();
    doctored["state"]["structures"][0] = huge.clone();
    doctored["state"]["generation"] = huge.clone();
    doctored["state"]["rite_counts"][0] = huge.clone();
    let s = begin_load(&doctored.to_string(), 1_000)
        .unwrap()
        .set_down()
        .state;

    let room = u64::MAX / 2;
    assert_eq!(s.ticks, room);
    assert_eq!(s.next_seq, room);
    assert_eq!(s.zoomies_until, room);
    assert_eq!(s.jamboree_until, room);
    assert_eq!(s.deja_vu_until, room);
    assert_eq!(s.pending_sky_event, Some(room));
    assert_eq!(s.kittens[0].job_seq, Some(room));
    assert_eq!(s.kittens[1].away_seq, Some(room));
    let small = i32::MAX as u32;
    assert_eq!(s.structures[0], small);
    assert_eq!(s.generation, small);
    assert_eq!(s.rite_counts[0], small);
    assert!(
        s.next_cost(StructureId::ALL[0])
            .iter()
            .all(|(_, amount)| *amount > 1e6),
        "the next copy of a great many costs a great deal"
    );
}

/// An ordinary save is untouched by any of this: a generator's whole 64-bit
/// range is its own, and a real save's numbers read as written.
// @spec SAVE-010, SAVE-032, SAVE-035
#[test]
fn numbers_in_range_read_exactly_as_written() {
    let mut s = played_state(36);
    let mut high = serde_json::to_value(&s.rng).unwrap();
    high["state"] = serde_json::json!(u64::MAX - 3);
    high["inc"] = serde_json::json!(u64::MAX);
    s.rng = serde_json::from_value(high).unwrap();
    let json = save_string(&s, 42_000);
    let loaded = load_string(&json, 42_000).unwrap();
    assert_eq!(save_string(&loaded.state, 42_000), json, "byte for byte");
}

/// What a save must hold, and what it may lose, by the name of each field the
/// format writes.
// @spec SAVE-004, SAVE-036
#[test]
fn only_what_every_version_wrote_is_required() {
    let (saved, _) = every_integer_held();
    let required_of = |container: &[&str]| -> Vec<String> {
        let path: JsonPath = container
            .iter()
            .map(|step| match step.parse::<u64>() {
                Ok(ix) => serde_json::json!(ix),
                Err(_) => serde_json::json!(step),
            })
            .collect();
        let mut whole = saved.clone();
        let keys: Vec<String> = at(&mut whole, &path)
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        let mut required: Vec<String> = keys
            .into_iter()
            .filter(|key| {
                let mut doctored = saved.clone();
                at(&mut doctored, &path)
                    .as_object_mut()
                    .unwrap()
                    .remove(key);
                match begin_load(&doctored.to_string(), 1_000) {
                    Err(LoadError::Corrupt { .. }) => true,
                    Err(e) => panic!("{key} missing fails as {e:?}"),
                    Ok(_) => false,
                }
            })
            .collect();
        required.sort();
        required
    };
    assert_eq!(required_of(&[]), ["save_version", "saved_at_unix", "state"]);
    assert_eq!(
        required_of(&["state"]),
        [
            "famine_days",
            "grumpy",
            "ideas",
            "kittens",
            "next_seq",
            "pity",
            "recovery_days",
            "resources",
            "rng",
            "structures",
            "ticks",
            "zoomies_until",
        ]
    );
    assert_eq!(
        required_of(&["state", "kittens", "0"]),
        ["arrived_day", "location", "name"]
    );
    assert_eq!(
        required_of(&["state", "family_album", "0"]),
        ["name", "trait_id"]
    );

    // Lost from a save of the current version, a field takes its new-game value.
    let mut doctored = saved.clone();
    doctored["state"]
        .as_object_mut()
        .unwrap()
        .remove("generation");
    doctored["state"]["kittens"][0]
        .as_object_mut()
        .unwrap()
        .remove("rank");
    let loaded = begin_load(&doctored.to_string(), 1_000)
        .unwrap()
        .set_down()
        .state;
    assert_eq!(loaded.generation, GameState::new(0).generation);
    assert_eq!(loaded.kittens[0].rank, 0);
}

// =====================================================================
// An absence the caller saw: the same catch-up, with no save read
// =====================================================================

/// Given the state, the ticks, and the span a load of the same absence would
/// work out, it comes to the same meadow, report, and journal, byte for byte.
// @spec SAVE-037
#[test]
fn an_absence_the_caller_saw_ends_as_a_load_of_it_would() {
    let s = played_state(37);
    let json = save_string(&s, 0);
    let by_load = load_string(&json, 300).unwrap();

    let state = load_string(&json, 0).unwrap().state;
    let Advance::Finished(seen) = begin_absence(state, 300 * 4, 300, false).advance(u64::MAX)
    else {
        panic!("advancing through everything finishes");
    };
    assert_eq!(save_string(&seen.state, 0), save_string(&by_load.state, 0));
    assert_eq!(seen.report, by_load.report);
    assert_eq!(seen.report.elapsed_seconds, 300);
}

/// Advanced a little at a time, set down part way, and told by its whole span.
// @spec SAVE-037
#[test]
fn an_absence_the_caller_saw_paces_and_sets_down_as_a_load_does() {
    let state = played_state(38);
    let catch_up = begin_absence(state.clone(), 100, 90, true);
    assert_eq!(catch_up.total_ticks(), 100);
    let Advance::Pending(catch_up) = catch_up.advance(40) else {
        panic!("sixty ticks are still owed");
    };
    assert_eq!(catch_up.remaining_ticks(), 60);
    let set_down = catch_up.set_down();
    assert_eq!(set_down.owed_ticks, 60);
    assert_eq!(set_down.report_so_far.elapsed_seconds, 90);
    assert!(set_down.report_so_far.truncated);
    assert_eq!(set_down.state.ticks, state.ticks + 40);
}

/// The report is retained under the rule a load's is, by its span.
// @spec SAVE-015, SAVE-037
#[test]
fn an_absence_the_caller_saw_is_retained_by_the_same_rule() {
    let away_entries = |seconds: u64| {
        let state = played_state(39);
        let Advance::Finished(game) = begin_absence(state, 8, seconds, false).advance(u64::MAX)
        else {
            panic!("advancing through everything finishes");
        };
        game.state
            .journal
            .kept
            .iter()
            .filter(|e| matches!(e.item, JournalItem::Away(_)))
            .count()
    };
    assert_eq!(away_entries(MIN_REPORTABLE_ABSENCE_SECONDS - 1), 0);
    assert_eq!(away_entries(MIN_REPORTABLE_ABSENCE_SECONDS), 1);
}

/// What the record held before any tick replayed, for telling what it earned.
// @spec SAVE-030, SAVE-037
#[test]
fn an_absence_the_caller_saw_answers_what_the_record_held() {
    let mut state = played_state(40);
    state.achievements.insert(
        AchievementId::ALL[0],
        Earned {
            generation: 0,
            day: 0,
        },
    );
    let held: BTreeSet<_> = state.achievements.keys().copied().collect();
    assert_eq!(begin_absence(state, 1, 60, false).earned_at_load(), &held);
}

/// An absence joining one in progress: its ticks owed, its span and truncation
/// in the one report, however the two divide it.
// @spec SAVE-038
#[test]
fn a_further_absence_joins_the_one_in_progress() {
    let state = played_state(41);
    let Advance::Finished(whole) = begin_absence(state.clone(), 360, 90, true).advance(u64::MAX)
    else {
        panic!("advancing through everything finishes");
    };

    let Advance::Pending(mut joined) = begin_absence(state, 200, 50, false).advance(100) else {
        panic!("some is still owed");
    };
    joined.add_absence(160, 40, true);
    assert_eq!(joined.total_ticks(), 360);
    assert_eq!(joined.remaining_ticks(), 260);
    let Advance::Finished(game) = joined.advance(u64::MAX) else {
        panic!("advancing through everything finishes");
    };
    assert_eq!(game.report, whole.report, "told as one absence");
    assert_eq!(save_string(&game.state, 0), save_string(&whole.state, 0));
}

// =====================================================================
// Clauses the first audit found no test for
// =====================================================================

/// A hand-edited speed comes back on the ladder, at whichever end it passed.
// @spec SAVE-005
#[test]
fn a_hand_edited_speed_clamps_to_the_ladder() {
    let saved: serde_json::Value =
        serde_json::from_str(&save_string(&GameState::new(37_001), 1_000)).unwrap();
    let speed_of = |edit: serde_json::Value| {
        let mut doctored = saved.clone();
        doctored["state"]["speed"] = edit;
        begin_load(&doctored.to_string(), 1_000)
            .unwrap()
            .set_down()
            .state
            .speed
    };
    let top = {
        let mut s = GameState::new(37_002);
        s.apply(Command::SetSpeed(f64::MAX)).unwrap();
        s.speed
    };
    let bottom = {
        let mut s = GameState::new(37_003);
        s.apply(Command::SetSpeed(0.0)).unwrap();
        s.speed
    };
    assert_eq!(speed_of(serde_json::json!(1_000.0)), top);
    assert_eq!(speed_of(serde_json::json!(0.0001)), bottom);
    assert_eq!(speed_of(serde_json::json!(-3.0)), bottom);
    assert_eq!(
        speed_of(serde_json::json!(2.0)),
        2.0,
        "one on the ladder stands"
    );
}

/// "At least": an absence of exactly the minimum is kept, and one a second
/// shorter is not, judged by the same load.
// @spec SAVE-015
#[test]
fn an_absence_of_exactly_the_minimum_is_kept() {
    let json = save_string(&GameState::new(37_004), 0);
    let kept_after = |seconds: u64| {
        load_string(&json, seconds)
            .unwrap()
            .state
            .journal
            .kept
            .iter()
            .filter(|e| matches!(e.item, JournalItem::Away(_)))
            .count()
    };
    assert_eq!(kept_after(MIN_REPORTABLE_ABSENCE_SECONDS), 1);
    assert_eq!(kept_after(MIN_REPORTABLE_ABSENCE_SECONDS - 1), 0);
}

/// An entry in a save of the current version, written before its event or its
/// report gained data, loads with that data at its default, and with no version
/// step: the save's own version is what says so.
// @spec SAVE-025
#[test]
fn an_entry_from_before_it_gained_data_loads_in_a_current_save() {
    let mut s = GameState::new(37_005);
    s.journal.record(
        5,
        &Event::TradeReturned {
            colony: ColonyId::Otters,
            caravans: 4,
            napped: 1,
            goods: vec![(ResourceId::Twigs, 3.0)],
        },
    );
    let earned = OfflineReport {
        elapsed_seconds: 90,
        earned: vec![AchievementId::ALL[0]],
        ..OfflineReport::default()
    };
    s.journal.record_away(6, &earned);
    let mut saved: serde_json::Value = serde_json::from_str(&save_string(&s, 1_000)).unwrap();
    assert_eq!(saved["save_version"], SAVE_VERSION);
    let journal = &mut saved["state"]["journal"];
    let trade = journal["recent"][0]["item"]["Happened"]["TradeReturned"]
        .as_object_mut()
        .expect("fixture: the trade's entry");
    trade
        .remove("napped")
        .expect("fixture: it held its nap count");
    trade.remove("goods").expect("fixture: it held its goods");
    let away = journal["kept"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find_map(|e| e["item"]["Away"].as_object_mut())
        .expect("fixture: the report's entry");
    away.remove("earned")
        .expect("fixture: it listed what it earned");

    let loaded = begin_load(&saved.to_string(), 1_000)
        .unwrap()
        .set_down()
        .state;
    assert_eq!(
        loaded.journal.recent[0].item,
        JournalItem::Happened(Event::TradeReturned {
            colony: ColonyId::Otters,
            caravans: 4,
            napped: 0,
            goods: vec![],
        })
    );
    let Some(JournalItem::Away(report)) = loaded.journal.kept.last().map(|e| &e.item) else {
        panic!("the report stands");
    };
    assert!(report.earned.is_empty());
}

// =====================================================================
// SAVE-039 — what a wake set down had earned still counts as the absence's
// =====================================================================

/// A meadow that earns its first achievements in the first days it replays.
fn about_to_earn() -> String {
    let mut s = played_state(39_000);
    assert!(!s.kittens.is_empty(), "fixture: the condition is met");
    s.achievements.clear();
    save_string(&s, 0)
}

/// A report lists what its replay earned, in the order earned; and one that
/// earned nothing writes no field for it, so such a save's bytes do not change.
// @spec SAVE-008, SAVE-039
#[test]
fn a_report_lists_what_its_replay_earned() {
    let loaded = load_string(&about_to_earn(), 300).unwrap();
    assert!(
        !loaded.report.earned.is_empty(),
        "fixture: it earned something"
    );
    let recorded: BTreeSet<_> = loaded.state.achievements.keys().copied().collect();
    let listed: BTreeSet<_> = loaded.report.earned.iter().copied().collect();
    assert_eq!(listed, recorded, "exactly what the record gained");
    assert_eq!(listed.len(), loaded.report.earned.len(), "each once");

    let none = OfflineReport::default();
    let written = serde_json::to_string(&none).unwrap();
    assert!(!written.contains("earned"), "no field: {written}");
}

/// A wake set down part way and taken up again answers the record as the absence
/// found it, and tells what the whole absence earned, as one sitting would.
// @spec SAVE-019, SAVE-030, SAVE-039
#[test]
fn a_wake_set_down_keeps_what_the_absence_earned_as_new() {
    let json = about_to_earn();
    let whole = load_string(&json, 300).unwrap();

    let first = begin_load(&json, 300).unwrap();
    assert!(
        first.earned_at_load().is_empty(),
        "fixture: nothing held yet"
    );
    let Advance::Pending(part_way) = first.advance(100) else {
        panic!("fixture: most of the absence is still owed");
    };
    let set_down = part_way.set_down();
    assert!(
        !set_down.report_so_far.earned.is_empty(),
        "fixture: the first part earned something"
    );
    assert!(
        !set_down.state.achievements.is_empty(),
        "and the record holds it, in the save"
    );

    let saved = save_string_carrying(
        &set_down.state,
        300,
        set_down.owed_ticks,
        Some(&set_down.report_so_far),
    );
    let taken_up = begin_load(&saved, 300).unwrap();
    assert!(
        taken_up.earned_at_load().is_empty(),
        "what the absence earned isn't what the player already knew"
    );
    let Advance::Finished(game) = taken_up.advance(u64::MAX) else {
        panic!("advancing through everything finishes");
    };
    assert_eq!(
        game.report.earned, whole.report.earned,
        "told as one absence"
    );
    assert_eq!(game.report.elapsed_seconds, whole.report.elapsed_seconds);
}

/// What the record held before the absence still counts as held after a set-down.
// @spec SAVE-030, SAVE-039
#[test]
fn a_wake_set_down_leaves_what_was_already_known_as_known() {
    let mut s = played_state(39_001);
    s.achievements.clear();
    let old = AchievementId::ALL[AchievementId::ALL.len() - 1];
    s.achievements.insert(
        old,
        Earned {
            generation: 0,
            day: 0,
        },
    );
    let json = save_string(&s, 0);
    let Advance::Pending(part_way) = begin_load(&json, 300).unwrap().advance(100) else {
        panic!("fixture: most of the absence is still owed");
    };
    let set_down = part_way.set_down();
    let saved = save_string_carrying(
        &set_down.state,
        300,
        set_down.owed_ticks,
        Some(&set_down.report_so_far),
    );
    let held = begin_load(&saved, 300).unwrap();
    assert!(held.earned_at_load().contains(&old));
    assert_eq!(
        held.earned_at_load().len(),
        1,
        "and nothing the absence earned"
    );
}

// =====================================================================
// SAVE-041 — a report lists what its replay revealed
// =====================================================================

/// A meadow whose next tick reveals exactly one structure and one idea, and
/// the two: everything else revealed and every other idea known, so nothing
/// else can reveal. The idea is the last, which leaves the nap's own idea known.
fn about_to_reveal() -> (GameState, StructureId, IdeaId) {
    let idea = *IdeaId::ALL.last().expect("ideas exist");
    assert_ne!(idea, IdeaId::DeepNapTheory, "fixture: naps stay open");
    for structure in StructureId::ALL {
        let mut s = GameState::new(41);
        for r in ResourceId::ALL {
            s.resources[r.ix()] = 1.0e9;
        }
        for b in StructureId::ALL {
            s.structures[b.ix()] = 2;
        }
        s.ideas = IdeaId::ALL.iter().copied().filter(|i| *i != idea).collect();
        s.revealed = StructureId::ALL
            .iter()
            .filter(|b| **b != structure)
            .map(|b| ContentId::Structure(*b))
            .chain(
                IdeaId::ALL
                    .iter()
                    .filter(|i| **i != idea)
                    .map(|i| ContentId::Idea(*i)),
            )
            .collect();
        // Through a save, so the core rebuilds what it derives from the set.
        let s = load_string(&save_string(&s, 0), 0).unwrap().state;
        let reveals: Vec<ContentId> = s
            .clone()
            .tick()
            .into_iter()
            .filter_map(|e| match e {
                Event::Revealed { id } => Some(id),
                _ => None,
            })
            .collect();
        if reveals == [ContentId::Structure(structure), ContentId::Idea(idea)] {
            return (s, structure, idea);
        }
    }
    panic!("fixture: no structure reveals beside the last idea on the next tick");
}

/// A load's report lists what its replay revealed, in the order revealed; and
/// one that revealed nothing writes no field for it.
// @spec SAVE-041
#[test]
fn a_report_lists_what_its_replay_revealed() {
    let (s, structure, idea) = about_to_reveal();
    let loaded = load_string(&save_string(&s, 0), 300).unwrap();
    assert_eq!(
        loaded.report.revealed,
        vec![ContentId::Structure(structure), ContentId::Idea(idea)],
        "the report lists the replay's reveals, in the order revealed"
    );

    let none = OfflineReport::default();
    let written = serde_json::to_string(&none).unwrap();
    assert!(!written.contains("revealed"), "no field: {written}");
}

/// A wake set down part way carries what it revealed, and the load that
/// finishes it tells the whole absence's reveals, as one sitting would.
// @spec SAVE-041, SAVE-022
#[test]
fn a_wake_set_down_carries_what_the_absence_revealed() {
    let (s, _, _) = about_to_reveal();
    let json = save_string(&s, 0);
    let whole = load_string(&json, 300).unwrap();
    assert_eq!(
        whole.report.revealed.len(),
        2,
        "fixture: the absence revealed two"
    );

    let Advance::Pending(part_way) = begin_load(&json, 300).unwrap().advance(1) else {
        panic!("fixture: most of the absence is still owed");
    };
    let set_down = part_way.set_down();
    assert_eq!(
        set_down.report_so_far.revealed, whole.report.revealed,
        "the first tick revealed both, and the report so far says so"
    );
    let saved = save_string_carrying(
        &set_down.state,
        300,
        set_down.owed_ticks,
        Some(&set_down.report_so_far),
    );
    let Advance::Finished(game) = begin_load(&saved, 300).unwrap().advance(u64::MAX) else {
        panic!("advancing through everything finishes");
    };
    assert_eq!(
        game.report.revealed, whole.report.revealed,
        "told as one absence"
    );
}

/// A nap's report lists what its sleep revealed, by the same fold.
// @spec SAVE-041
#[test]
fn a_naps_report_lists_what_it_revealed() {
    let (mut s, structure, idea) = about_to_reveal();
    let events = s.apply(Command::DeepNap(1)).expect("the nap is affordable");
    let report = events
        .iter()
        .find_map(|e| match e {
            Event::Napped { report, .. } => Some(report.clone()),
            _ => None,
        })
        .expect("a nap returns its report");
    assert_eq!(
        report.revealed,
        vec![ContentId::Structure(structure), ContentId::Idea(idea)]
    );
}
