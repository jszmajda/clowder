//! The App container and the loop body — `runtime::run` and `runtime::App::step`.
//!
//! @spec TUI-RUN-001, TUI-RUN-014, TUI-RUN-015, TUI-RUN-016, TUI-RUN-017
//!
//! **How this test reaches the code.** Not by including a module: it runs the
//! real `clowder` binary as a child process. TUI-RUN-001 is an *ordering*
//! claim about startup — the save path is resolved and the load attempted
//! before raw mode and the alternate screen — and the only place that ordering
//! is observable is from outside the process, in what reaches the terminal
//! before the refusal does. A child process gives that observation without
//! faking a terminal and without putting the developer's own terminal into raw
//! mode, which is what `tests/term.rs` refuses to do for the same reason.
//!
//! The child's save is redirected by pointing `HOME` (and `XDG_DATA_HOME`) at a
//! scratch directory, because `runtime::persist::resolve_save_path` takes no
//! argument and reads no override — the same missing seam `tests/persist.rs`
//! already reports. The child is also given the scratch directory as its
//! working directory, so even the "no platform data directory" fallback lands
//! in scratch rather than in the repository. Every child is run under a
//! deadline and killed if it outlives it, so a redirect that failed on some
//! future platform shows up as a failing assertion rather than as a test that
//! quietly drove the player's real save.
//!
//! A startup *failure* is what makes TUI-RUN-001 observable at all, so these
//! tests necessarily lean on the load-failure policy (TUI-RUN-011,
//! TUI-RUN-013) that shares `runtime::run` with the loop. They assert nothing
//! about *what* that policy decides — only that whatever it says arrives on an
//! ordinary screen.
//!
//! **What has no seam and is therefore not tested here.** TUI-RUN-014's frame
//! ordering, TUI-RUN-015's commands-before-ticks, TUI-RUN-016's bounded drain
//! and resize coalescing, and TUI-RUN-017's advance-regardless-of-render are
//! all properties of one method — `App::step(&mut self) -> io::Result<()>` —
//! whose signature exposes none of them:
//!
//! - there is no input source to inject, so a bounded drain has nothing to
//!   drain and a resize flood has nothing to flood;
//! - there is no clock reading to inject, so a frame's elapsed interval cannot
//!   be chosen and "the sim advanced" has no expected value;
//! - `App` holds no terminal and `step` takes none, so the frame cannot be
//!   drawn into an in-memory backend — nor skipped, since nothing says whether
//!   it was drawn;
//! - and `step` returns `()`, so the order in which it did those things leaves
//!   no trace.
//!
//! Calling `step()` from a test would poll the developer's own terminal for
//! keys and then panic inside `render::frame`, which is `todo!()` in a leaf
//! this segment does not own. That is a test that can never pass honestly, so
//! it is not written. The seams that would make these four testable headlessly
//! are proposed in the open questions returned with this file.

use std::fmt;
use std::fs;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

/// The binary under test. Cargo exports this to integration tests of a package
/// that has a `[[bin]]`.
const CLOWDER: &str = env!("CARGO_BIN_EXE_clowder");

/// Generous: a refusal is prompt, and the only thing this bounds is how long a
/// *broken* implementation is allowed to hang before the test says so.
const CHILD_BUDGET: Duration = Duration::from_secs(15);

/// Every form of "switch to the alternate screen" a terminal library might
/// emit. Any of them appearing before a startup refusal means the refusal was
/// printed into a screen the player will never see again, which is precisely
/// what TUI-RUN-001 exists to prevent.
const ALTERNATE_SCREEN: [(&str, &str); 4] = [
    ("\x1b[?1049h", "enter alternate screen"),
    ("\x1b[?1049l", "leave alternate screen"),
    ("\x1b[?1047h", "enter alternate screen (legacy)"),
    ("\x1b[?47h", "enter alternate screen (legacy)"),
];

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
            .join(format!("clowder-loop-{}-{}-{}", std::process::id(), tag, n));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("a scratch directory should be creatable");
        TempDir { path }
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

/// Where the child will look for its save, given the home it is handed. This
/// mirrors the platform rule `clowder_core::default_save_path` follows; it is
/// the price of `resolve_save_path` having no injectable override.
#[cfg(unix)]
fn child_save_path(home: &Path) -> PathBuf {
    if cfg!(target_os = "macos") {
        home.join("Library/Application Support/clowder/save.json")
    } else {
        home.join(".local/share/clowder/save.json")
    }
}

/// What a child process did.
#[cfg(unix)]
struct Run {
    stdout: String,
    stderr: String,
    status: ExitStatus,
    /// True if the child outlived its deadline and had to be killed.
    timed_out: bool,
}

#[cfg(unix)]
impl fmt::Display for Run {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "child status: {}{}\n--- child stdout ---\n{}\n--- child stderr ---\n{}",
            self.status,
            if self.timed_out { " (killed at the deadline)" } else { "" },
            self.stdout.escape_debug(),
            self.stderr.escape_debug()
        )
    }
}

#[cfg(unix)]
impl Run {
    fn combined(&self) -> String {
        format!("{}{}", self.stdout, self.stderr)
    }

    /// The startup failure never reached the alternate screen — it was printed
    /// on the terminal the player was already looking at.
    fn expect_no_alternate_screen(&self) {
        for (sequence, what) in ALTERNATE_SCREEN {
            assert!(
                !self.stdout.contains(sequence) && !self.stderr.contains(sequence),
                "the shell emitted {what} ({}) — the load was attempted only after the \
                 terminal had been taken over, so the startup failure was reported onto a \
                 screen that disappears.\n{self}",
                sequence.escape_debug()
            );
        }
    }

    /// It refused: promptly, orderly, with something to say, and with a
    /// non-success code.
    ///
    /// "Orderly" is doing real work here. A startup failure has to *report
    /// itself*, and a panic dump is not a report — it is the shell falling over
    /// on its way to the decision, which tells the player nothing about their
    /// save and proves nothing about when the load was attempted. A crash also
    /// happens to produce a non-success exit and a non-empty stderr, so without
    /// this check the assertions below would pass for a shell that never
    /// reached the save at all.
    fn expect_refusal(&self) {
        assert!(
            !self.timed_out,
            "the shell never refused — it was still running at the deadline, so it \
             started a game on a save it should have stopped for.\n{self}"
        );
        assert_eq!(
            self.status.signal(),
            None,
            "the shell died by signal instead of refusing; a refusal to start is a return, \
             not a crash.\n{self}"
        );
        assert!(
            !self.stderr.contains("panicked at"),
            "the shell panicked instead of reporting a startup failure — a panic dump is \
             not the plain explanation TUI-RUN-001 promises, and it says nothing about \
             whether the save was ever read.\n{self}"
        );
        assert!(
            !self.status.success(),
            "the shell exited successfully on a save it cannot use; a refusal to start \
             must be a non-success code.\n{self}"
        );
        assert!(
            !self.combined().trim().is_empty(),
            "the shell refused to start and said nothing at all — there is no plain \
             explanation for the player to act on.\n{self}"
        );
    }
}

/// Run the real binary against a scratch home, under a deadline.
#[cfg(unix)]
fn run_clowder(home: &Path) -> Run {
    let mut child = Command::new(CLOWDER)
        .env_clear()
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join(".local/share"))
        .env("PATH", "/usr/bin:/bin")
        .env("TERM", "xterm-256color")
        // So that even the no-data-directory fallback ("beside the working
        // directory", TUI-RUN-005) writes into scratch rather than the repo.
        .current_dir(home)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawning {CLOWDER}: {e}"));

    let deadline = Instant::now() + CHILD_BUDGET;
    let mut timed_out = false;
    loop {
        match child.try_wait().expect("polling the child") {
            Some(_) => break,
            None if Instant::now() >= deadline => {
                timed_out = true;
                let _ = child.kill();
                break;
            }
            None => std::thread::sleep(Duration::from_millis(20)),
        }
    }

    let output = child.wait_with_output().expect("collecting the child's output");
    Run {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        status: output.status,
        timed_out,
    }
}

/// A save claiming a version this build has never heard of. The core's loader
/// version-checks the loose parse before deserializing, so the rest of the
/// envelope is never read — which is the point: this fixture forces a refusal
/// without depending on anything about the save format.
#[cfg(unix)]
fn write_a_save_from_the_future(path: &Path) {
    fs::create_dir_all(path.parent().expect("the save path has a parent"))
        .expect("creating the scratch data directory");
    let contents = format!(
        "{{\n  \"save_version\": {},\n  \"saved_at_unix\": 0,\n  \"state\": {{}}\n}}\n",
        clowder_core::SAVE_VERSION + 1
    );
    fs::write(path, contents).expect("writing the fixture save");
}

// ---------------------------------------------------------------------------
// TUI-RUN-001 — the load is attempted before the terminal is taken over
// ---------------------------------------------------------------------------

/// The whole reason the load comes first: when it fails, the explanation has to
/// land somewhere the player can read it. A shell that entered the alternate
/// screen first would print its refusal into a buffer that is torn down on the
/// way out, and the player would see an empty terminal and a process that
/// vanished.
///
/// A save from a newer version is the cleanest way to force that failure: the
/// core decides it from the version field alone, before any of the save is
/// deserialized, so nothing here depends on the shape of a save.
///
/// @spec TUI-RUN-001
#[cfg(unix)]
#[test]
fn a_startup_failure_is_reported_before_the_alternate_screen_is_entered() {
    let home = TempDir::new("too-new");
    let save = child_save_path(home.path());
    write_a_save_from_the_future(&save);

    let run = run_clowder(home.path());

    run.expect_refusal();
    run.expect_no_alternate_screen();
}

/// The same claim for the failure that carries no information at all. A save
/// path that is a directory cannot be read, so the shell knows nothing about
/// its contents — and still has to say so on an ordinary terminal rather than
/// discovering the problem after raw mode is on.
///
/// This is the harsher case: it fails at the read, earlier than any parse, so
/// an implementation that resolves the path early but defers the *read* until
/// after `TerminalGuard::enter()` is caught here and nowhere else.
///
/// @spec TUI-RUN-001
#[cfg(unix)]
#[test]
fn a_save_that_cannot_be_read_at_all_is_reported_before_the_alternate_screen() {
    let home = TempDir::new("unreadable");
    let save = child_save_path(home.path());
    fs::create_dir_all(&save).expect("creating a directory where the save file belongs");

    let run = run_clowder(home.path());

    run.expect_refusal();
    run.expect_no_alternate_screen();
}

/// The alternate screen is not the only way to take a terminal over. Clearing
/// it, hiding its cursor, or turning on mouse reporting all belong to a shell
/// that has already committed to owning the screen — and any of them landing
/// before a startup refusal means the load happened too late, whether or not
/// the alternate screen was used to do it.
///
/// Styling is deliberately not caught: a colour on an error message leaves the
/// terminal exactly as it found it. What is caught is state the player's
/// ordinary terminal would be left holding.
///
/// @spec TUI-RUN-001
#[cfg(unix)]
#[test]
fn a_startup_failure_does_not_take_the_terminal_over_in_any_other_way() {
    /// Sequence, and the take-over it represents.
    const TAKE_OVER: [(&str, &str); 7] = [
        ("\x1b[2J", "clear the screen"),
        ("\x1b[3J", "clear the scrollback"),
        ("\x1b[?25l", "hide the cursor"),
        ("\x1b[?1000h", "enable mouse reporting"),
        ("\x1b[?1002h", "enable mouse drag reporting"),
        ("\x1b[?1003h", "enable mouse motion reporting"),
        ("\x1b[?1006h", "enable SGR mouse encoding"),
    ];

    let home = TempDir::new("untouched");
    let save = child_save_path(home.path());
    write_a_save_from_the_future(&save);

    let run = run_clowder(home.path());
    run.expect_refusal();

    let combined = run.combined();
    for (sequence, what) in TAKE_OVER {
        assert!(
            !combined.contains(sequence),
            "before refusing to start the shell asked the terminal to {what} ({}) — it had \
             already taken the screen over, so the load came too late to be reported on the \
             terminal the player was looking at.\n{run}",
            sequence.escape_debug()
        );
    }
}
