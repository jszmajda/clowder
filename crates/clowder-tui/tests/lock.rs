//! One session per save — `runtime::lock`.
//!
//! @spec TUI-RUN-006, TUI-RUN-007, TUI-RUN-008, TUI-RUN-009, TUI-RUN-029
//!
//! **How this test reaches the code.** `clowder-tui` is a binary crate with no
//! library target, so an integration test cannot `use clowder_tui::…`.
//! `runtime::lock` depends on nothing else in the crate, so this file compiles
//! the module in directly by path. If the implementation later reaches for a
//! sibling module (`crate::…`), this include stops compiling and the crate
//! needs a `[lib]` target instead — flagged as an open question rather than
//! fixed here, since `Cargo.toml` belongs to another chunk.
//!
//! **What the harness assumes about the lock file, and why.** The design fixes
//! that the lock sits *beside the save* and records "the owning process's id
//! and start time"; it does not fix the file's name or its format. So these
//! tests never name the lock file — they discover it as the entry beside the
//! save that is not the save. And because staleness (TUI-RUN-008) can only be
//! exercised by presenting a lock that some *other* process wrote, the two
//! stale tests take a genuine lock file this process wrote, tamper with it, and
//! put it back. That tampering assumes the id and the start time appear as
//! plain decimal numbers in a text file. Every place that assumption is used
//! asserts it held, so a format that breaks it fails loudly with an explanation
//! rather than passing hollowly.
//!
//! **What is deliberately not tested here.** TUI-RUN-007 also requires the
//! refusal to *state how to override*; that message is composed on the startup
//! path (`runtime::run`), not in this module — `LockError` carries the holding
//! pid and no message — so it is not observable at this seam. TUI-RUN-029's
//! literal ordering claim ("acquire before reading the save") likewise lives in
//! `runtime::run`, which enters raw mode and belongs to another chunk; what is
//! checked here are the claim's observable consequences: the lock cannot depend
//! on the save being present or readable, simultaneous launches cannot both
//! pass, and a launch that refuses after acquiring leaves nothing behind.

#[path = "../src/runtime/lock.rs"]
#[allow(dead_code)]
mod lock;

use lock::{LockError, SessionLock};

use std::fs;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Scratch space
// ---------------------------------------------------------------------------

fn scratch_dir(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("clowder-lock-{}-{}", std::process::id(), test));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// The save the lock sits beside. Most tests never create it: the lock is taken
/// before the save is read, so it must not care whether the file is there.
fn save_path(dir: &Path) -> PathBuf {
    dir.join("clowder-save.json")
}

/// Everything beside the save. The lock's filename is the implementation's
/// choice, so the tests find it rather than assume it.
fn beside_the_save(dir: &Path, save: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = fs::read_dir(dir)
        .expect("scratch dir should be readable")
        .map(|e| e.expect("dir entry should be readable").path())
        .filter(|p| p != save)
        .collect();
    found.sort();
    found
}

fn the_lock_file(dir: &Path, save: &Path) -> PathBuf {
    let found = beside_the_save(dir, save);
    assert_eq!(
        found.len(),
        1,
        "exactly one lock should sit beside the save, found {found:?}"
    );
    found.into_iter().next().unwrap()
}

fn lock_text(dir: &Path, save: &Path) -> String {
    let path = the_lock_file(dir, save);
    fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "these tests read the lock at {path:?} as text in order to tamper with it: {e}\n\
             a non-text lock format needs a different harness, not a weaker assertion"
        )
    })
}

// ---------------------------------------------------------------------------
// Reading and tampering with a lock, without knowing its format
// ---------------------------------------------------------------------------

/// Every standalone run of decimal digits in `text`.
fn number_tokens(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut run = String::new();
    for ch in text.chars() {
        if ch.is_ascii_digit() {
            run.push(ch);
        } else if !run.is_empty() {
            tokens.push(std::mem::take(&mut run));
        }
    }
    if !run.is_empty() {
        tokens.push(run);
    }
    tokens
}

/// Rewrite standalone decimal runs. `f` returns a replacement or `None` to
/// leave the run alone. Token-wise rather than substring-wise, so a process id
/// whose digits also occur *inside* a longer number is not disturbed.
fn map_number_tokens(text: &str, f: impl Fn(&str) -> Option<String>) -> (String, usize) {
    let mut out = String::with_capacity(text.len());
    let mut changed = 0usize;
    let mut run = String::new();
    let flush = |run: &mut String, out: &mut String, changed: &mut usize| {
        if run.is_empty() {
            return;
        }
        match f(run) {
            Some(replacement) => {
                out.push_str(&replacement);
                *changed += 1;
            }
            None => out.push_str(run),
        }
        run.clear();
    };
    for ch in text.chars() {
        if ch.is_ascii_digit() {
            run.push(ch);
        } else {
            flush(&mut run, &mut out, &mut changed);
            out.push(ch);
        }
    }
    flush(&mut run, &mut out, &mut changed);
    (out, changed)
}

fn records_pid(text: &str, pid: u32) -> bool {
    let wanted = pid.to_string();
    number_tokens(text).contains(&wanted)
}

/// Larger than any process id either target platform can issue (Darwin caps
/// around 10^5, Linux at 2^22), so nothing can be running under it. A lock
/// naming it is a lock whose process is definitively gone.
const PID_THAT_CANNOT_EXIST: &str = "2147483646";

/// The same lock, attributed to a process that does not exist — what a crash or
/// a `kill -9` leaves behind.
fn attributed_to_a_dead_process(text: &str, pid: u32) -> String {
    let wanted = pid.to_string();
    let (out, changed) = map_number_tokens(text, |run| {
        (run == wanted).then(|| PID_THAT_CANNOT_EXIST.to_string())
    });
    assert!(
        changed > 0,
        "these tests need the owning process id to appear as a decimal number in the lock \
         so they can attribute it to a dead process; it does not appear in {text:?}"
    );
    out
}

/// The same lock, still naming *this* live process, but recording a start time
/// that is not this process's — what an id recycled by something unrelated
/// looks like. Only long runs are touched: a start time in any unit is a large
/// number, while a short run is more likely a schema version or a flag.
fn attributed_to_a_different_start_time(text: &str, pid: u32) -> String {
    let wanted = pid.to_string();
    let (out, changed) = map_number_tokens(text, |run| {
        if run == wanted || run.len() < 5 {
            return None;
        }
        let mut digits: Vec<char> = run.chars().collect();
        digits[0] = if digits[0] == '9' { '1' } else { '9' };
        Some(digits.into_iter().collect())
    });
    assert!(
        changed > 0,
        "these tests need the recorded start time to appear as a decimal number in the lock \
         so they can make it disagree with this process's real start time; no such number \
         appears in {text:?}"
    );
    out
}

/// A genuine lock file for *this* process — its real bytes and its real path —
/// obtained by taking a lock and letting it go. The stale tests tamper with a
/// real one rather than guessing at a format.
fn genuine_lock(dir: &Path, save: &Path) -> (PathBuf, String) {
    let held = SessionLock::acquire(save, false).expect("an unlocked save should lock");
    let path = the_lock_file(dir, save);
    let text = lock_text(dir, save);
    drop(held);
    (path, text)
}

// ---------------------------------------------------------------------------
// TUI-RUN-006 — the lock exists, names its owner, and is let go
// ---------------------------------------------------------------------------

/// The lock sits beside the save and records *who* holds it. Both halves are
/// load-bearing: "beside the save" is what makes the lock per-save rather than
/// per-machine, and the recorded id is what lets a second launch name the
/// session holding the game (TUI-RUN-007) instead of just refusing. The start
/// time recorded alongside it is what makes staleness decidable at all — an id
/// on its own is recycled.
///
/// @spec TUI-RUN-006
#[test]
fn the_lock_sits_beside_the_save_and_records_this_process() {
    let dir = scratch_dir("records-owner");
    let save = save_path(&dir);

    let _held = SessionLock::acquire(&save, false).expect("an unlocked save should lock");

    let text = lock_text(&dir, &save);
    let me = std::process::id();
    assert!(
        records_pid(&text, me),
        "the lock should record the owning process's id ({me}), got {text:?}"
    );

    let alongside_the_id = number_tokens(&text)
        .into_iter()
        .any(|t| t != me.to_string());
    assert!(
        alongside_the_id,
        "the lock should record a start time alongside the id — an id alone is recycled, \
         and refusing to start because an unrelated process inherited a number is the same \
         lockout by another route; got {text:?}"
    );
}

/// Releasing means the next launch is not locked out, and it means nothing is
/// left beside the save for that launch to inherit. A session that ended
/// cleanly must leave no trace that a later one has to reason about.
///
/// @spec TUI-RUN-006
#[test]
fn dropping_the_lock_releases_it() {
    let dir = scratch_dir("drop-releases");
    let save = save_path(&dir);

    let held = SessionLock::acquire(&save, false).expect("an unlocked save should lock");
    assert_eq!(
        beside_the_save(&dir, &save).len(),
        1,
        "a lock should be held"
    );
    drop(held);

    assert!(
        beside_the_save(&dir, &save).is_empty(),
        "a released lock should leave nothing beside the save for the next launch to inherit"
    );
    let _next = SessionLock::acquire(&save, false)
        .expect("the next launch should take the lock a finished session let go");
}

/// A panic is one of the exit paths the lock must survive: the drop guard is
/// what covers a session that ends by unwinding rather than by returning. If it
/// did not, a crash would hand the player a lockout — and a stderr message
/// about the panic on top of it.
///
/// (This test prints a panic message of its own; that is the panic being
/// caught, not a failure.)
///
/// @spec TUI-RUN-006
#[test]
fn a_panic_releases_the_lock() {
    let dir = scratch_dir("panic-releases");
    let save = save_path(&dir);

    let unwound = std::panic::catch_unwind(|| {
        let _held = SessionLock::acquire(&save, false).expect("an unlocked save should lock");
        panic!("a session ending the ugly way");
    });
    assert!(unwound.is_err(), "the panic should have been caught here");

    let _next = SessionLock::acquire(&save, false)
        .expect("a panicking session should release its lock as it unwinds");
}

// ---------------------------------------------------------------------------
// TUI-RUN-007 — a live holder refuses the second launch
// ---------------------------------------------------------------------------

/// Two sessions autosaving one save is silent progress loss — both hold a
/// divergent state and the later write erases the other's afternoon. So a
/// launch that finds a live holder refuses, and names the process holding the
/// game rather than reporting an anonymous failure.
///
/// @spec TUI-RUN-007
#[test]
fn a_live_holder_refuses_the_second_launch_and_names_it() {
    let dir = scratch_dir("live-refuses");
    let save = save_path(&dir);

    let _held = SessionLock::acquire(&save, false).expect("an unlocked save should lock");

    match SessionLock::acquire(&save, false) {
        Err(LockError::Held { pid }) => assert_eq!(
            pid,
            std::process::id(),
            "the refusal should name the process actually holding the game"
        ),
        Err(other) => panic!("a live holder should refuse with Held, got {other:?}"),
        Ok(_) => panic!(
            "a second launch found a live holder and started anyway — two sessions on one save \
             is exactly the silent progress loss the lock exists to prevent"
        ),
    }
}

/// The refusing launch has no lock of its own to release, and must not touch
/// the one it found. Clearing a live holder's lock on the way out would leave
/// the running session unprotected against a third launch — a refusal that
/// creates the very race it declined to join.
///
/// @spec TUI-RUN-007, TUI-RUN-029
#[test]
fn a_refused_launch_leaves_the_holders_lock_intact() {
    let dir = scratch_dir("refusal-keeps-lock");
    let save = save_path(&dir);

    let _held = SessionLock::acquire(&save, false).expect("an unlocked save should lock");
    let before = lock_text(&dir, &save);

    assert!(
        SessionLock::acquire(&save, false).is_err(),
        "a live holder should refuse the second launch"
    );

    let after = lock_text(&dir, &save);
    assert_eq!(
        before, after,
        "a refused launch should leave the holder's lock exactly as it found it"
    );
}

// ---------------------------------------------------------------------------
// TUI-RUN-008 — a stale lock clears itself
// ---------------------------------------------------------------------------

/// A lock outliving its process means clowder crashed or was killed. The
/// player's response to reopening their game must not be a lockout over a file
/// their previous session failed to clean up, so the lock clears itself and
/// play continues.
///
/// @spec TUI-RUN-008
#[test]
fn a_lock_whose_process_is_gone_is_cleared_and_play_continues() {
    let dir = scratch_dir("dead-pid-stale");
    let save = save_path(&dir);

    let (path, genuine) = genuine_lock(&dir, &save);
    let crashed = attributed_to_a_dead_process(&genuine, std::process::id());
    fs::write(&path, &crashed).expect("writing the crashed session's lock should succeed");

    let _held = SessionLock::acquire(&save, false).unwrap_or_else(|e| {
        panic!("a crash must not lock a player out of their own game, got {e:?}")
    });

    let now = lock_text(&dir, &save);
    assert_ne!(
        now, crashed,
        "the stale lock should have been cleared, not left in place"
    );
    assert!(
        records_pid(&now, std::process::id()),
        "the cleared lock should now record this session as the owner, got {now:?}"
    );
}

/// Staleness is decided on the id *and* the start time together. Process ids
/// are recycled, so a lock naming a live process is not evidence that clowder
/// is running: if that process started at a different time from the one the
/// lock recorded, the id was inherited by something unrelated. Refusing to
/// start over that would be the same lockout as trusting a dead id — just
/// reached by another route.
///
/// @spec TUI-RUN-008
#[test]
fn a_recycled_id_with_a_different_start_time_is_stale() {
    let dir = scratch_dir("recycled-id-stale");
    let save = save_path(&dir);

    // This process is unquestionably alive, so an implementation trusting the
    // id alone will refuse here. Only the start time reveals that the lock was
    // written by a different process that happened to hold the same number.
    let (path, genuine) = genuine_lock(&dir, &save);
    let recycled = attributed_to_a_different_start_time(&genuine, std::process::id());
    assert_ne!(
        recycled, genuine,
        "the tampered lock should disagree with this process's real start time"
    );
    fs::write(&path, &recycled).expect("writing the recycled-id lock should succeed");

    let _held = SessionLock::acquire(&save, false).unwrap_or_else(|e| {
        panic!(
            "the lock names a live process, but one that started at a different time from the \
             session that wrote it — the id was recycled, and an id alone must not lock a \
             player out of their own game; got {e:?}"
        )
    });

    let now = lock_text(&dir, &save);
    assert_ne!(
        now, recycled,
        "the stale lock should have been cleared and rewritten for this session"
    );
    assert!(
        records_pid(&now, std::process::id()),
        "the cleared lock should now record this session as the owner, got {now:?}"
    );
}

// ---------------------------------------------------------------------------
// TUI-RUN-009 — the override
// ---------------------------------------------------------------------------

/// The override exists for the case the liveness checks get it wrong. It force
/// -clears even a lock whose holder is demonstrably alive — otherwise a
/// misjudged check would be an unrecoverable lockout, which is the outcome the
/// whole stale-lock policy exists to avoid.
///
/// @spec TUI-RUN-009
#[test]
fn the_override_clears_a_live_lock() {
    let dir = scratch_dir("override-live");
    let save = save_path(&dir);

    let _held = SessionLock::acquire(&save, false).expect("an unlocked save should lock");
    assert!(
        SessionLock::acquire(&save, false).is_err(),
        "without the override, a live holder refuses — otherwise this test proves nothing"
    );

    let _forced = SessionLock::acquire(&save, true)
        .expect("the override should start even over a live holder");

    let now = lock_text(&dir, &save);
    assert!(
        records_pid(&now, std::process::id()),
        "the forced session should hold the lock afterwards, got {now:?}"
    );
}

/// Forcing past a lock is not the same as running without one. A session that
/// overrode a stale or misjudged lock still owns the save, and a third launch
/// arriving after it must be refused exactly as it would be against any other
/// live session.
///
/// @spec TUI-RUN-009, TUI-RUN-006
#[test]
fn a_forced_session_still_holds_the_lock() {
    let dir = scratch_dir("override-still-locks");
    let save = save_path(&dir);

    let _forced = SessionLock::acquire(&save, true)
        .expect("the override on a free save should still take the lock");

    assert_eq!(
        beside_the_save(&dir, &save).len(),
        1,
        "a forced session should hold a lock, not skip locking"
    );
    match SessionLock::acquire(&save, false) {
        Err(LockError::Held { .. }) => {}
        Ok(_) => {
            panic!("a launch arriving after a forced session should be refused like any other")
        }
        Err(other) => panic!("expected Held, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// TUI-RUN-029 — the lock comes first, and refusals leave nothing behind
// ---------------------------------------------------------------------------

/// The lock is the mutual-exclusion primitive, taken *before* the save is read.
/// That ordering is only possible if acquiring cannot depend on the save at
/// all: not on it existing, not on it parsing, not on it even being a file. An
/// acquire that needed any of those would have to run after the read, leaving
/// the window in which two launches both read the same save and both believe
/// they are alone. Each case here is also a real startup path — a first run, a
/// save that turns out to be corrupt, and a save path that cannot be read
/// (TUI-RUN-010, TUI-RUN-012, TUI-RUN-013) — every one of which must reach its
/// decision holding the lock.
///
/// @spec TUI-RUN-029
#[test]
fn the_lock_does_not_depend_on_the_save_being_readable() {
    let first_run = scratch_dir("before-read-absent");
    let absent = save_path(&first_run);
    let _a = SessionLock::acquire(&absent, false)
        .expect("a first run has no save yet, and must still take the lock before looking");

    let corrupt_dir = scratch_dir("before-read-corrupt");
    let corrupt = save_path(&corrupt_dir);
    fs::write(&corrupt, b"{ this is not a save").expect("writing a corrupt save should succeed");
    let _b = SessionLock::acquire(&corrupt, false).expect(
        "the lock is taken before the save is parsed, so unparseable contents cannot block it",
    );

    let unreadable_dir = scratch_dir("before-read-unreadable");
    let unreadable = save_path(&unreadable_dir);
    fs::create_dir(&unreadable).expect("creating a directory at the save path should succeed");
    let _c = SessionLock::acquire(&unreadable, false).expect(
        "a save path that is not a file is refused after the lock is taken, so acquiring must \
         not depend on reading it",
    );
}

/// A launch that takes the lock and then refuses to start — the save was
/// written by a newer clowder, say, or could not be read at all — must release
/// it on the way out. Otherwise the refusal leaves a lock for the next launch
/// to inherit, and a save that merely cannot be opened becomes a save that
/// cannot be opened *and* is locked.
///
/// @spec TUI-RUN-029
#[test]
fn a_launch_that_refuses_after_acquiring_leaves_no_lock() {
    let dir = scratch_dir("refusal-releases");
    let save = save_path(&dir);

    let refused: Result<(), &str> = {
        let _held = SessionLock::acquire(&save, false).expect("an unlocked save should lock");
        // The save turns out to be from a newer version; this launch stops here.
        Err("save written by a newer clowder")
    };
    assert!(refused.is_err(), "this launch refused to start");

    assert!(
        beside_the_save(&dir, &save).is_empty(),
        "a refusal should leave no lock beside the save"
    );
    let _next = SessionLock::acquire(&save, false)
        .expect("a launch that refused to start must not lock the next one out");
}

/// Two launches arriving at once must not both pass the check. Threads stand in
/// for processes here: they contend for the same lock file at the same moment,
/// which is the race an acquire-then-check implementation loses. Exactly one
/// may proceed to read the save; if more than one does, both go on to autosave
/// over each other, which is the loss the lock exists to prevent.
///
/// @spec TUI-RUN-029
#[test]
fn simultaneous_launches_cannot_both_pass_the_check() {
    let dir = scratch_dir("simultaneous");
    let save = save_path(&dir);

    // The window a losing implementation leaves open is a few instructions
    // wide, so one contest rarely lands in it. Many contests do.
    for round in 0..CONTESTS {
        assert_eq!(
            winners_of_a_contest(&save),
            1,
            "contest {round}: exactly one of {LAUNCHES} simultaneous launches may hold the save"
        );
        assert!(
            beside_the_save(&dir, &save).is_empty(),
            "contest {round}: the winner's release should leave nothing beside the save"
        );
    }
}

/// The same contest, against a lock a crashed session left behind. Every launch
/// finds it stale and every launch moves to clear it; a launch that clears on
/// what it read a moment ago, rather than on what is there now, removes the
/// lock the first of them has just taken and takes the save a second time.
///
/// @spec TUI-RUN-008, TUI-RUN-029
#[test]
fn simultaneous_launches_past_a_stale_lock_cannot_both_pass_the_check() {
    let dir = scratch_dir("simultaneous-stale");
    let save = save_path(&dir);
    let (path, text) = genuine_lock(&dir, &save);
    let stale = attributed_to_a_dead_process(&text, std::process::id());

    for round in 0..CONTESTS {
        fs::write(&path, &stale).unwrap();
        assert_eq!(
            winners_of_a_contest(&save),
            1,
            "contest {round}: exactly one of {LAUNCHES} simultaneous launches may clear a stale \
             lock and hold the save"
        );
    }
}

const LAUNCHES: usize = 8;
const CONTESTS: usize = 40;

/// Start `LAUNCHES` launches at the same moment and count how many came away
/// holding the save. Every outcome is collected before any is dropped, so the
/// winners are all still holding their locks when they are counted.
fn winners_of_a_contest(save: &Path) -> usize {
    let start_together = std::sync::Barrier::new(LAUNCHES);
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..LAUNCHES)
            .map(|_| {
                let start_together = &start_together;
                scope.spawn(move || {
                    start_together.wait();
                    SessionLock::acquire(save, false)
                })
            })
            .collect();
        let outcomes: Vec<_> = handles
            .into_iter()
            .map(|h| h.join().expect("an acquiring launch should not panic"))
            .collect();
        outcomes.iter().filter(|o| o.is_ok()).count()
    })
}
