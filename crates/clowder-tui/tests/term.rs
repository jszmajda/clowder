//! Terminal lifecycle and termination signals — `runtime::term`.
//!
//! @spec TUI-RUN-002, TUI-RUN-003, TUI-RUN-004
//!
//! **How this test reaches the code.** `clowder-tui` is a binary crate with no
//! library target, so an integration test cannot `use clowder_tui::…`.
//! `runtime::term` depends on nothing else in the crate, so this file compiles
//! the module in directly by path. If the implementation later reaches for a
//! sibling module (`crate::…`), this include stops compiling and the crate
//! needs a `[lib]` target instead — flagged as an open question rather than
//! fixed here, since `Cargo.toml` belongs to another chunk.
//!
//! **What is deliberately not tested here.** Raw mode and the alternate screen
//! need a real terminal, and crossterm falls back to `/dev/tty` when stdin is
//! not one — so calling `TerminalGuard::enter()` from a test would put *the
//! developer's own terminal* into raw mode when run from a shell, and would
//! exercise only the error path in CI. Neither is a test of TUI-RUN-002, and
//! faking a terminal would prove nothing about the real one. TUI-RUN-002's
//! enter-and-restore and TUI-RUN-003's before-the-default-hook ordering are
//! therefore left to a pty harness and reported as honest gaps.
//!
//! What *is* testable headlessly is TUI-RUN-004's whole observable claim: a
//! termination signal is **recorded** and nothing else, so the process is still
//! alive afterwards to take its ordinary shutdown path. That claim is checked
//! in re-executed child processes, because a signal is process-global: sending
//! SIGTERM to the test binary itself would flip the flag under every other test
//! running in parallel, and would take the whole harness down if the handler
//! were missing.

use clowder_tui::runtime::term;

/// A signal handler that only records leaves the flag clear until a signal
/// actually arrives — the loop must not shut down on a process nobody asked to
/// terminate. Runs in the test process itself, which is never signalled.
///
/// @spec TUI-RUN-004
#[test]
fn a_fresh_process_records_no_termination_request() {
    term::install_signal_handlers().expect("installing SIGTERM/SIGHUP handlers should succeed");

    assert!(
        !term::termination_requested(),
        "no signal has been delivered, so nothing should be recorded as requesting termination"
    );
}

// ---------------------------------------------------------------------------
// Signal scenarios, each run in a re-executed child of this test binary.
// ---------------------------------------------------------------------------

#[cfg(unix)]
const CHILD_ENV: &str = "CLOWDER_TERM_TEST_SCENARIO";

/// SIGTERM is what `tmux kill-session`, a logout, and a shutdown send. The
/// handler must record it and return: the process is still running afterwards,
/// so the loop can save and restore the terminal on the ordinary shutdown path.
/// A handler that did that work itself — or that was never installed, letting
/// the default disposition kill the process — fails here.
///
/// @spec TUI-RUN-004
#[cfg(unix)]
#[test]
fn sigterm_is_recorded_and_the_process_lives_on_to_shut_itself_down() {
    let child = run_scenario_in_child("sigterm");

    child.expect_mark("installed", "the handlers were never installed");
    child.expect_survived("SIGTERM");
    child.expect_mark(
        "recorded",
        "SIGTERM was not recorded, so the loop would never learn to shut down",
    );
    child.expect_clean_exit();
}

/// SIGHUP arrives when the terminal that owns the pane goes away. It gets the
/// same treatment as SIGTERM: recorded, not acted on.
///
/// @spec TUI-RUN-004
#[cfg(unix)]
#[test]
fn sighup_is_recorded_and_the_process_lives_on_to_shut_itself_down() {
    let child = run_scenario_in_child("sighup");

    child.expect_mark("installed", "the handlers were never installed");
    child.expect_survived("SIGHUP");
    child.expect_mark(
        "recorded",
        "SIGHUP was not recorded, so the loop would never learn to shut down",
    );
    child.expect_clean_exit();
}

/// Nothing stops a pane from sending SIGTERM twice — a `kill` followed by an
/// impatient second `kill`. Because the handler only records, the second signal
/// is as harmless as the first: still recorded, still alive, still the loop's
/// job to shut down.
///
/// @spec TUI-RUN-004
#[cfg(unix)]
#[test]
fn a_second_termination_signal_is_still_only_recorded() {
    let child = run_scenario_in_child("twice");

    child.expect_mark("installed", "the handlers were never installed");
    child.expect_survived("a second SIGTERM");
    child.expect_mark("recorded", "the repeated SIGTERM left nothing recorded");
    child.expect_clean_exit();
}

/// The record is a fact about what has arrived, not a message consumed by the
/// first reader: the shutdown path observes it while saving and again while
/// restoring, and a flag that cleared on the first look would strand the loop
/// half-shut-down. (This encodes one reading of "record it" — see the open
/// questions returned with these tests.)
///
/// @spec TUI-RUN-004
#[cfg(unix)]
#[test]
fn the_recorded_request_survives_being_observed() {
    let child = run_scenario_in_child("sticky");

    child.expect_mark("installed", "the handlers were never installed");
    child.expect_mark(
        "recorded",
        "SIGTERM was not recorded, so the loop would never learn to shut down",
    );
    child.expect_mark(
        "sticky",
        "the recorded request cleared when it was read, so a later step of the \
         same shutdown would see no termination request",
    );
    child.expect_clean_exit();
}

/// Ctrl-C is deliberately outside this module. In raw mode the terminal hands
/// it to the program as an ordinary key event, which the navigation layer owns
/// (TUI-NAV-016) — that is what lets that layer promise Ctrl-C saves first.
/// Installing a SIGINT handler here would take that key away from it, so after
/// the runtime's handlers are installed SIGINT must still carry its default
/// disposition.
///
/// @spec TUI-RUN-004
#[cfg(unix)]
#[test]
fn ctrl_c_is_not_one_of_the_signals_this_module_handles() {
    let child = run_scenario_in_child("sigint");

    child.expect_mark("installed", "the handlers were never installed");
    assert!(
        !child.stdout.contains("[mark] survived"),
        "SIGINT was caught by the runtime's signal handlers; in raw mode Ctrl-C \
         is an ordinary key event belonging to the navigation layer.\n{child}"
    );
    assert_eq!(
        child.signal(),
        Some(SIGINT),
        "SIGINT should have kept its default disposition and ended the process.\n{child}"
    );
}

/// Not a test: the re-execution target for the scenarios above. In an ordinary
/// run the scenario variable is unset and this does nothing.
#[cfg(unix)]
#[test]
fn scenario_child() {
    let scenario = match std::env::var(CHILD_ENV) {
        Ok(scenario) => scenario,
        Err(_) => return,
    };

    term::install_signal_handlers().expect("installing SIGTERM/SIGHUP handlers should succeed");
    mark("installed");
    assert!(
        !term::termination_requested(),
        "a termination request was recorded before any signal was sent"
    );

    match scenario.as_str() {
        "sigterm" => send_to_self("TERM"),
        "sighup" => send_to_self("HUP"),
        "twice" => {
            send_to_self("TERM");
            send_to_self("TERM");
        }
        "sticky" => send_to_self("TERM"),
        "sigint" => {
            send_to_self("INT");
            // Only reached if something swallowed SIGINT.
            std::thread::sleep(std::time::Duration::from_millis(500));
            mark("survived");
            return;
        }
        other => panic!("unknown scenario {other:?}"),
    }

    await_record();
    // Printed only if the process outlived a signal whose default disposition
    // is to kill it — the observable half of "the handler only records".
    mark("survived");
    mark("recorded");

    if scenario == "sticky" {
        assert!(
            term::termination_requested(),
            "the recorded request cleared when it was read"
        );
        assert!(
            term::termination_requested(),
            "the recorded request cleared when it was read"
        );
        mark("sticky");
    }
}

#[cfg(unix)]
const SIGINT: i32 = 2;

/// Markers go to stdout with an explicit newline so they are flushed before a
/// signal can end the process; the parent reads them back out of the child's
/// captured output.
#[cfg(unix)]
fn mark(what: &str) {
    use std::io::Write;
    println!("[mark] {what}");
    let _ = std::io::stdout().flush();
}

#[cfg(unix)]
fn send_to_self(signal: &str) {
    let pid = std::process::id().to_string();
    let status = std::process::Command::new("kill")
        .args(["-s", signal, &pid])
        .status()
        .expect("running `kill` to signal this process");
    assert!(
        status.success(),
        "`kill -s {signal} {pid}` failed: {status}"
    );
}

/// Poll until the signal has been recorded. A signal is delivered
/// asynchronously, so a single immediate read would be a race.
#[cfg(unix)]
fn await_record() {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !term::termination_requested() {
        assert!(
            std::time::Instant::now() < deadline,
            "the signal was delivered but never recorded"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

#[cfg(unix)]
struct ChildRun {
    stdout: String,
    stderr: String,
    status: std::process::ExitStatus,
}

#[cfg(unix)]
impl std::fmt::Display for ChildRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "child status: {}\n--- child stdout ---\n{}--- child stderr ---\n{}",
            self.status, self.stdout, self.stderr
        )
    }
}

#[cfg(unix)]
impl ChildRun {
    fn signal(&self) -> Option<i32> {
        use std::os::unix::process::ExitStatusExt;
        self.status.signal()
    }

    fn expect_mark(&self, what: &str, why: &str) {
        assert!(
            self.stdout.contains(&format!("[mark] {what}")),
            "{why}\n{self}"
        );
    }

    /// The process outlived a signal whose default disposition would have
    /// killed it — the whole point of recording rather than acting.
    fn expect_survived(&self, signal: &str) {
        assert_eq!(
            self.signal(),
            None,
            "{signal} ended the process instead of being recorded for the loop \
             to act on.\n{self}"
        );
        self.expect_mark("survived", &format!("the process did not outlive {signal}"));
    }

    fn expect_clean_exit(&self) {
        assert!(
            self.status.success(),
            "the child did not exit cleanly.\n{self}"
        );
    }
}

/// Re-execute this test binary, running only the scenario child, with the
/// scenario named in the environment.
#[cfg(unix)]
fn run_scenario_in_child(scenario: &str) -> ChildRun {
    let exe = std::env::current_exe().expect("path of this test binary");
    let output = std::process::Command::new(exe)
        .args([
            "--exact",
            "scenario_child",
            "--nocapture",
            "--test-threads",
            "1",
        ])
        .env(CHILD_ENV, scenario)
        .output()
        .expect("re-executing this test binary as a child");

    ChildRun {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        status: output.status,
    }
}
