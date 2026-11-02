//! The shell's one read of the monotonic clock: real time unless held, and held
//! only for the thread that holds it, for as long as the hold lasts.
//!
//! @spec TUI-RUN-066, TUI-RUN-067

use std::time::{Duration, Instant};

use clowder_tui::runtime::now;

/// With nothing holding it, the clock reads real time.
/// @spec TUI-RUN-066
#[test]
fn an_unheld_clock_reads_real_time() {
    let (before, read, after) = (Instant::now(), now::instant(), Instant::now());
    assert!(before <= read && read <= after);
}

/// Held, the clock stays put and moves only by hand, by exactly what it's given.
/// @spec TUI-RUN-067
#[test]
fn a_held_clock_moves_only_when_advanced() {
    let _clock = now::held();
    let start = now::instant();
    std::thread::sleep(Duration::from_millis(5));
    assert_eq!(now::instant(), start, "real time passing moves nothing");
    now::advance(Duration::from_secs(9));
    assert_eq!(now::since(start), Duration::from_secs(9));
}

/// A hold ends where it's dropped, and an inner one hands back the outer's place.
/// @spec TUI-RUN-067
#[test]
fn a_hold_ends_with_its_guard_and_nests() {
    let before = Instant::now();
    {
        let _outer = now::held();
        now::advance(Duration::from_secs(5));
        let outer = now::instant();
        {
            let _inner = now::held();
            now::advance(Duration::from_secs(60));
        }
        assert_eq!(now::instant(), outer, "the outer hold's place comes back");
    }
    let read = now::instant();
    assert!(
        read >= before && read < before + Duration::from_secs(5),
        "the clock reads real time again"
    );
}

/// A hold reaches no other thread, so tests side by side keep their own time.
/// @spec TUI-RUN-067
#[test]
fn a_hold_belongs_to_its_own_thread() {
    let _clock = now::held();
    now::advance(Duration::from_secs(3600));
    let held = now::instant();
    let other = std::thread::spawn(now::instant)
        .join()
        .expect("the other thread reads its clock");
    assert!(other < held, "another thread still reads real time");
}

/// Advancing a clock nothing holds is a mistake, and says so.
/// @spec TUI-RUN-067
#[test]
#[should_panic(expected = "nothing holds")]
fn advancing_an_unheld_clock_panics() {
    now::advance(Duration::from_secs(1));
}

/// Nothing in the shell reads the monotonic clock around the one function, or a
/// held clock wouldn't reach it. Only that function's own file names the real
/// read.
/// @spec TUI-RUN-066
#[test]
fn every_read_of_the_monotonic_clock_goes_through_the_one_function() {
    fn sources(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("the source tree reads") {
            let path = entry.expect("an entry").path();
            if path.is_dir() {
                sources(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    sources(&root, &mut files);
    assert!(files.len() > 10, "fixture: the shell's sources were found");
    let real_read = concat!("Instant", "::now");
    for file in files {
        let text = std::fs::read_to_string(&file).expect("a source file reads");
        let elapsed = concat!(".", "elapsed()");
        if file.ends_with("runtime/now.rs") {
            continue;
        }
        assert!(
            !text.contains(real_read) && !text.contains(elapsed),
            "{} reads the clock around the one function",
            file.display()
        );
    }
}
