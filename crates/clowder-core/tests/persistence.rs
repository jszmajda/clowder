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

    // The replay must be *the same code path* as live play: byte-identical.
    let mut manual = s.clone();
    for _ in 0..2400 {
        manual.tick();
    }
    assert_eq!(save_string(&loaded.state, 0), save_string(&manual, 0));
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
