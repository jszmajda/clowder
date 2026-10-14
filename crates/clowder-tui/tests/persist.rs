//! Save path, load-failure policy, autosave cadence — `runtime::persist`, plus
//! the save entry point on `runtime::App`.
//!
//! @spec TUI-RUN-005, TUI-RUN-012, TUI-RUN-026, TUI-RUN-027, TUI-RUN-028
//!
//! **How this test reaches the code.** `clowder-tui` is a binary crate with no
//! library target, so an integration test cannot `use clowder_tui::…`; the
//! modules under test are compiled in by path, as the sibling runtime tests do.
//! Unlike `clock`, `lock`, and `term`, this file cannot include a single leaf
//! module: the save entry point the scaffold annotates with TUI-RUN-027 is
//! `App::save` in `runtime/mod.rs`, which reaches for `crate::nav` and
//! `crate::render`. Those two are pulled in only to make that include compile —
//! nothing here tests them. A `[lib]` target would make all of this
//! unnecessary; that belongs to `Cargo.toml`, which is another chunk's, and is
//! reported as an open question rather than fixed here.
//!
//! **Where the load-failure policy is tested instead.** TUI-RUN-010 (no save:
//! start fresh, no error), TUI-RUN-011 (a newer save: refuse, leave it alone)
//! and TUI-RUN-013 (an unreadable save: stop, leave it untouched) are decided
//! inside `runtime::run`, which enters raw mode and loops — so there is no
//! function here to call. They are observed from outside instead, in
//! `loop_order.rs`, by running the real binary against a scratch home and
//! looking at what it said and what it left on disk. That is also the only
//! place the distinction this design leans hardest on can be seen: a file that
//! could not be *read* is never moved aside, where an unparseable one is.
//!
//! That distinction is why `clowder_core::read_save` cannot make it alone: it
//! maps *every* non-`NotFound` read error onto `LoadError::Corrupt`, so a
//! permission error arrives looking exactly like unparseable JSON. Satisfying
//! TUI-RUN-013 means the shell reads the file itself and only then hands the
//! contents to `load_string`.
//!
//! TUI-RUN-026's cadence half is likewise unobservable: `persist::autosave_due`
//! takes no arguments and returns a bool, so it has no input to vary and no
//! state to advance — the elapsed interval it judges cannot be supplied, and
//! sleeping out a real autosave interval is not a test. What *is* checked below
//! is the other half of that spec: that what the shell writes is a save the
//! core can read back.

use clowder_tui::nav;
use clowder_tui::runtime;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use clowder_core::GameState;

use runtime::clock::Reading;
use runtime::persist::{resolve_save_path, set_aside};
use runtime::{App, TickAccumulator};

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

/// A scratch directory that cleans itself up. `clowder-tui` has no dev
/// dependency on a temp-dir crate and `Cargo.toml` belongs to another chunk, so
/// this is hand-rolled rather than pulled in.
struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(tag: &str) -> TempDir {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir()
            .join(format!("clowder-persist-{}-{}-{}", std::process::id(), tag, n));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("a scratch directory should be creatable");
        TempDir { path }
    }

    fn join(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn now_unix() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).expect("the clock is past 1970").as_secs()
}

/// Directory contents by name, sorted — so a test can assert what the shell did
/// *not* leave behind as well as what it did.
fn entries(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .expect("the scratch directory should be readable")
        .map(|e| e.expect("a directory entry").file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// A session whose only interesting parts are the save path and banked time.
/// Every other field is inert here: nothing below renders, navigates, or steps.
fn session(save_path: PathBuf, clock: TickAccumulator) -> App {
    let mut state = GameState::new(4242);
    // Pinned so the tick counts a load replays are a function of the elapsed
    // interval alone, not of whatever a fresh game's speed happens to be.
    state.speed = 1.0;
    App {
        state,
        view: nav::ViewState::default(),
        clock,
        save_path,
        last_save: Instant::now(),
        prev_layout: None,
        pointer_record: None,
        quitting: false,
        save_failed: false,
    }
}

/// Both clocks in step — an ordinary attended interval, with no suspend to
/// detect and no correction to reject.
fn steady(d: Duration) -> Reading {
    Reading { monotonic: d, wall: d }
}

/// Bytes that are emphatically not a save: unbalanced braces, no
/// `save_version`, and a note from a player who tried to hand-edit it. Used
/// wherever a test needs a file the loader cannot parse but a human could still
/// read — which is the entire premise of the set-aside policy.
const UNPARSEABLE: &[u8] = b"{ \"save_version\": 3, \"state\": { oops I edited this by hand\n";

// ---------------------------------------------------------------------------
// TUI-RUN-005 — where the save lives
// ---------------------------------------------------------------------------

/// The shell does not invent a location when the platform offers one, and does
/// not refuse to start when it does not. Both branches are asserted here rather
/// than skipping whichever the host does not exercise: the platform decides
/// which arm runs, and each arm is a real claim.
///
/// The fallback arm cannot be *forced* on a normal host — `default_save_path`
/// resolves through `dirs_sys::home_dir`, which falls back to `getpwuid` when
/// `HOME` is unset, so there is no environment a test can construct in-process
/// that makes it `None`. See the open question about giving `resolve_save_path`
/// an injectable input.
///
/// @spec TUI-RUN-005
#[test]
fn the_save_path_follows_the_platform_data_directory_or_falls_back_beside_the_cwd() {
    let resolved = resolve_save_path();

    match clowder_core::default_save_path() {
        Some(platform) => assert_eq!(
            resolved, platform,
            "when the platform resolves a data directory the save belongs in it, \
             not somewhere the shell chose for itself"
        ),
        None => {
            let cwd = std::env::current_dir().expect("a working directory");
            assert!(
                resolved.is_relative() || resolved.starts_with(&cwd),
                "with no platform data directory the save falls back *beside the working \
                 directory*, but landed at {resolved:?} (cwd {cwd:?})"
            );
        }
    }
}

/// Startup loads from this path and every later autosave writes to it. A path
/// that differed between calls — a timestamp in the fallback name, a re-read of
/// a mutable environment — would quietly load one file and save another, and
/// the player would lose the session they thought they were continuing.
///
/// @spec TUI-RUN-005
#[test]
fn the_save_path_is_the_same_on_every_call() {
    let first = resolve_save_path();
    let second = resolve_save_path();
    let third = resolve_save_path();

    assert_eq!(first, second, "the save path must not move between calls");
    assert_eq!(second, third, "the save path must not move between calls");
}

/// "State where the save landed" needs a nameable file, and the core's atomic
/// write needs a directory to put its temp file in and rename within. A path
/// with no file name (a bare directory) satisfies neither.
///
/// @spec TUI-RUN-005
#[test]
fn the_save_path_names_a_file_inside_a_directory() {
    let resolved = resolve_save_path();

    let name = resolved.file_name().unwrap_or_else(|| {
        panic!("the save path must name a file the player can be told about, got {resolved:?}")
    });
    assert!(
        !name.is_empty(),
        "the save path must name a file the player can be told about, got {resolved:?}"
    );

    let parent = resolved.parent().unwrap_or_else(|| {
        panic!("the save path needs a containing directory for the atomic temp-and-rename")
    });
    assert!(
        !parent.as_os_str().is_empty() || resolved.is_relative(),
        "the save path needs a containing directory for the atomic temp-and-rename, \
         got {resolved:?}"
    );
}

// ---------------------------------------------------------------------------
// TUI-RUN-012 — an unparseable save is moved aside, never overwritten
// ---------------------------------------------------------------------------

/// Moved, not copied. If the broken file were left at the live path, the fresh
/// game that follows would write straight over it at the first autosave — which
/// is exactly the outcome the policy exists to prevent.
///
/// @spec TUI-RUN-012
#[test]
fn an_unparseable_save_is_moved_off_the_live_path() {
    let dir = TempDir::new("moved");
    let save = dir.join("save.json");
    fs::write(&save, UNPARSEABLE).expect("writing the fixture");

    let moved_to = set_aside(&save).expect("setting a broken save aside should succeed");

    assert!(
        !save.exists(),
        "the unparseable save must not still sit at the live path after being set aside"
    );
    assert!(moved_to.exists(), "the set-aside save must exist where the player is told it went");
    assert_ne!(moved_to, save, "moving a file onto itself moves nothing");
}

/// Saves are human-readable JSON precisely so a broken one can be hand-rescued;
/// a set-aside that rewrote, truncated, or re-encoded the file would destroy the
/// only artifact that made that promise real. The bytes that come out must be
/// the bytes that went in, including the parts that made it unparseable.
///
/// @spec TUI-RUN-012
#[test]
fn the_bytes_set_aside_are_the_bytes_that_were_there() {
    let dir = TempDir::new("bytes");
    let save = dir.join("save.json");
    fs::write(&save, UNPARSEABLE).expect("writing the fixture");

    let moved_to = set_aside(&save).expect("setting a broken save aside should succeed");

    let rescued = fs::read(&moved_to).expect("the set-aside save should be readable");
    assert_eq!(
        rescued, UNPARSEABLE,
        "the set-aside save must be byte-identical to what was there — it is the artifact \
         a player hand-rescues"
    );
}

/// "Under a suffixed name": the rescued file keeps the name the player knows,
/// with something added, and stays in the directory they were already looking
/// in. A move to a temp directory or to an unrelated name would technically not
/// overwrite anything and would still lose the save in practice.
///
/// @spec TUI-RUN-012
#[test]
fn the_set_aside_save_is_the_original_name_plus_a_suffix_in_the_same_directory() {
    let dir = TempDir::new("suffix");
    let save = dir.join("save.json");
    fs::write(&save, UNPARSEABLE).expect("writing the fixture");

    let moved_to = set_aside(&save).expect("setting a broken save aside should succeed");

    assert_eq!(
        moved_to.parent(),
        save.parent(),
        "the set-aside save belongs beside the original, where the player is already looking; \
         it went to {moved_to:?}"
    );

    let original = save.file_name().expect("the fixture names a file").to_string_lossy().into_owned();
    let rescued = moved_to
        .file_name()
        .expect("the set-aside path names a file")
        .to_string_lossy()
        .into_owned();
    assert!(
        rescued.starts_with(&original) && rescued.len() > original.len(),
        "TUI-RUN-012 says a *suffixed* name: {rescued:?} should be {original:?} with something \
         appended"
    );
}

/// The whole point, stated as the sequence that actually happens: the broken
/// save is set aside, a fresh game starts and saves to the live path, and the
/// rescued file is still exactly what it was. A policy that overwrote in place
/// would leave one file here; a policy that set aside and then clobbered its own
/// rescue would leave two files with the wrong contents.
///
/// @spec TUI-RUN-012
#[test]
fn a_fresh_game_started_after_a_set_aside_never_touches_the_rescued_file() {
    let dir = TempDir::new("fresh");
    let save = dir.join("save.json");
    fs::write(&save, UNPARSEABLE).expect("writing the fixture");

    let moved_to = set_aside(&save).expect("setting a broken save aside should succeed");

    // Fresh game, written where the broken one used to be.
    let fresh = GameState::new(7);
    clowder_core::write_save(&save, &fresh, now_unix()).expect("a fresh save should write");

    assert_eq!(
        fs::read(&moved_to).expect("the rescued save should still be readable"),
        UNPARSEABLE,
        "starting fresh must not disturb the file the player was told to go and rescue"
    );
    assert!(
        clowder_core::read_save(&save, now_unix()).is_ok(),
        "the live path should now hold a loadable new game"
    );
}

/// A set-aside that quietly reported success without moving anything would let
/// startup tell the player "your save is now at X" while the broken file was
/// still live and about to be overwritten. Failure has to arrive as failure.
///
/// @spec TUI-RUN-012
#[test]
fn a_set_aside_that_cannot_happen_is_reported_rather_than_claimed() {
    let dir = TempDir::new("absent");
    let save = dir.join("save.json");
    assert!(!save.exists(), "fixture: nothing is at the save path");

    let result = set_aside(&save);

    assert!(
        result.is_err(),
        "set_aside reported success at {:?} for a file that was never there",
        result.ok()
    );
    assert_eq!(
        entries(dir.path()),
        Vec::<String>::new(),
        "a failed set-aside must not leave a stray file behind"
    );
}

// ---------------------------------------------------------------------------
// TUI-RUN-026 / TUI-RUN-027 — what a save says about when it is
// ---------------------------------------------------------------------------

/// The shell writes through the core's writer, so what it produces is a save the
/// core loads — not a shape only the shell understands — and the atomic
/// temp-and-rename leaves nothing half-written behind.
///
/// @spec TUI-RUN-026
#[test]
fn the_save_the_shell_writes_is_one_the_core_can_read_back() {
    let dir = TempDir::new("roundtrip");
    let save = dir.join("save.json");
    let app = session(save.clone(), TickAccumulator::default());

    app.save().expect("saving to a writable directory should succeed");

    clowder_core::read_save(&save, now_unix()).expect("the core must be able to load what the shell wrote");

    let leftovers: Vec<String> =
        entries(dir.path()).into_iter().filter(|n| n.ends_with(".tmp")).collect();
    assert!(
        leftovers.is_empty(),
        "the atomic write renames its temp file into place; found {leftovers:?}"
    );
}

/// With nothing banked, the simulation *is* at the present, and the stamp says
/// so: reloading immediately owes no catch-up at all.
///
/// @spec TUI-RUN-027
#[test]
fn a_save_with_nothing_banked_is_stamped_with_the_present() {
    let dir = TempDir::new("current");
    let save = dir.join("save.json");
    let app = session(save.clone(), TickAccumulator::default());
    assert_eq!(
        app.clock.unrun_span(),
        Duration::ZERO,
        "fixture: a fresh accumulator owes nothing"
    );

    app.save().expect("saving to a writable directory should succeed");
    let after = now_unix();

    let loaded = clowder_core::read_save(&save, after).expect("the save should load");
    assert!(
        loaded.report.elapsed_seconds <= 1,
        "a caught-up save reloaded at once should owe nothing, but owed {}s",
        loaded.report.elapsed_seconds
    );
}

/// The subtle one. The state in memory is deliberately behind real time —
/// ninety seconds of ticks are banked and unrun — so the save must carry those
/// ticks, or the next load will never run them.
///
/// This previously worked by back-dating the stamp so the interval re-derived
/// them. That re-priced the backlog at whatever speed was set at save time:
/// ticks earned fast and saved slow vanished, ticks earned slow and saved fast
/// were conjured, by the ratio between the two. The count is carried
/// explicitly now. The assertion is on the reload, because "what was banked is
/// owed again, exactly once" is the behaviour; the field is only how.
///
/// @spec TUI-RUN-027, SAVE-017
#[test]
fn a_save_taken_mid_catch_up_owes_exactly_what_it_banked() {
    let dir = TempDir::new("backdated");
    let save = dir.join("save.json");

    // Ninety seconds accrue and none of them run — a shutdown mid-backlog.
    let mut clock = TickAccumulator::default();
    clock.observe(steady(Duration::from_secs(90)), 1.0);
    let banked = clock.backlog();
    assert!(banked > 0, "fixture: ninety seconds should bank a backlog");

    let app = session(save.clone(), clock);
    let ticks_at_save = app.state.ticks;
    app.save().expect("saving to a writable directory should succeed");

    // Reloaded immediately: no new absence, so the only ticks that run are the
    // ones that were banked.
    let loaded = clowder_core::read_save(&save, now_unix()).expect("the save should load");
    assert_eq!(
        loaded.state.ticks,
        ticks_at_save + banked,
        "the reload must run exactly the {banked} banked ticks — no more, no fewer"
    );
}

/// The other half of "neither lost nor counted twice". Once the backlog has
/// actually run, the simulation *is* at the present and the stamp must stop
/// back-dating — otherwise every catch-up would be replayed a second time on the
/// next load and absence would pay twice.
///
/// @spec TUI-RUN-027
#[test]
fn an_interval_that_has_already_run_is_not_owed_again() {
    let dir = TempDir::new("drained");
    let save = dir.join("save.json");

    let mut clock = TickAccumulator::default();
    clock.observe(steady(Duration::from_secs(90)), 1.0);
    let drained = clock.take(u64::MAX);
    assert!(drained > 0, "fixture: the backlog should be drainable");
    assert_eq!(clock.backlog(), 0, "fixture: the whole backlog was taken");
    assert_eq!(
        clock.unrun_span(),
        Duration::ZERO,
        "a backlog that has run represents no unrun real time"
    );

    let app = session(save.clone(), clock);
    app.save().expect("saving to a writable directory should succeed");
    let after = now_unix();

    let loaded = clowder_core::read_save(&save, after).expect("the save should load");
    assert!(
        loaded.report.elapsed_seconds <= 1,
        "ticks that have already run must not be owed a second time, but the next load was \
         owed {}s",
        loaded.report.elapsed_seconds
    );
}

// ---------------------------------------------------------------------------
// TUI-RUN-028 — a failed save is survivable
// ---------------------------------------------------------------------------

/// A full disk, a revoked permission, a data directory that vanished: the write
/// fails and the session keeps going. Killing a healthy session over a transient
/// disk problem destroys far more progress than the failed write did, so the
/// failure has to arrive as a value the loop can carry, not as a panic or an
/// exit — and the game state has to be intact enough to save again.
///
/// The obstacle is a parent that is a regular file rather than a read-only
/// directory: it fails identically for every user, including root, so this test
/// has no environment in which it quietly stops checking anything.
///
/// @spec TUI-RUN-028
#[test]
fn a_failed_save_is_an_error_the_session_survives() {
    let dir = TempDir::new("blocked");
    let blocked = dir.join("data");
    fs::write(&blocked, b"this is a file, not a directory").expect("writing the obstacle");
    let save = blocked.join("save.json");

    let app = session(save, TickAccumulator::default());

    let result = app.save();
    assert!(result.is_err(), "a save into an impossible path must report failure, not succeed");

    // Still a live session: the same app can be asked again, and asking does
    // not panic or take the process down.
    assert!(app.save().is_err(), "the session survives the failure and can be asked again");
}

/// "Retry at the next cadence" is only meaningful if the retry can succeed. A
/// failure that latched — a poisoned path, a save disabled after the first
/// error — would turn one bad moment into a session that never saves again.
///
/// @spec TUI-RUN-028
#[test]
fn a_save_that_failed_once_succeeds_when_the_obstacle_clears() {
    let dir = TempDir::new("cleared");
    let blocked = dir.join("data");
    fs::write(&blocked, b"this is a file, not a directory").expect("writing the obstacle");
    let save = blocked.join("save.json");

    let app = session(save.clone(), TickAccumulator::default());
    assert!(app.save().is_err(), "fixture: the first save should fail");

    // The disk frees up, the directory comes back, the permission is restored —
    // whatever it was, the next cadence finds a path that works.
    fs::remove_file(&blocked).expect("clearing the obstacle");

    app.save().expect("the next cadence must be able to save once the obstacle is gone");
    clowder_core::read_save(&save, now_unix())
        .expect("the retried save should be a save the core can load");
}
