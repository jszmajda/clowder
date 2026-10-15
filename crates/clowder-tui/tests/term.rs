//! Terminal lifecycle and termination signals — `runtime::term`.
//!
//! @spec TUI-RUN-002, TUI-RUN-003, TUI-RUN-004, TUI-RUN-060, TUI-RUN-061
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
//! therefore left to a pty harness: the guard scenarios at the end of this
//! file run it in a re-executed child whose standard streams are a
//! pseudo-terminal this process opens, never the developer's own.
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
        // The guard scenarios run with this child's standard streams on a
        // pseudo-terminal the parent reads (`guard_pty` below).
        "guard-drop" => {
            let guard =
                term::TerminalGuard::enter().expect("entering the terminal on a pseudo-terminal");
            drop(guard);
            mark("dropped");
            return;
        }
        "guard-panic" => {
            let _guard =
                term::TerminalGuard::enter().expect("entering the terminal on a pseudo-terminal");
            panic!("a panic with the terminal taken");
        }
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

// ---------------------------------------------------------------------------
// TUI-RUN-060, TUI-RUN-061 — mouse reporting through the guard, on a
// pseudo-terminal
// ---------------------------------------------------------------------------
//
// The scenario child above enters the terminal through `TerminalGuard` with
// its standard streams on a pseudo-terminal this process opens and reads, in
// a session of its own so it can never reach the terminal the tests were
// started from. Then it either lets the guard go — the path every ordinary
// return and every error return takes — or panics with the guard held. What
// it wrote is read back as the DEC private modes it set and reset, in order.

#[cfg(any(target_os = "macos", target_os = "linux"))]
mod guard_pty {
    use std::ffi::CStr;
    use std::fs::{File, OpenOptions};
    use std::io::{self, Read};
    use std::os::fd::{AsRawFd, FromRawFd};
    use std::os::raw::{c_char, c_int};
    use std::os::unix::fs::OpenOptionsExt;
    use std::os::unix::process::CommandExt;
    use std::process::{Child, Command, Stdio};
    use std::time::{Duration, Instant};

    const O_RDWR: c_int = 2;
    const F_GETFL: c_int = 3;
    const F_SETFL: c_int = 4;
    #[cfg(target_os = "macos")]
    const O_NOCTTY: c_int = 0x0002_0000;
    #[cfg(target_os = "macos")]
    const O_NONBLOCK: c_int = 0x0004;
    #[cfg(target_os = "linux")]
    const O_NOCTTY: c_int = 0o400;
    #[cfg(target_os = "linux")]
    const O_NONBLOCK: c_int = 0o4000;

    extern "C" {
        fn posix_openpt(flags: c_int) -> c_int;
        fn grantpt(fd: c_int) -> c_int;
        fn unlockpt(fd: c_int) -> c_int;
        fn ptsname_r(fd: c_int, buf: *mut c_char, len: usize) -> c_int;
        fn fcntl(fd: c_int, cmd: c_int, ...) -> c_int;
        fn setsid() -> c_int;
    }

    /// A child killed and reaped on every path.
    struct Reaped(Child);

    impl Drop for Reaped {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    /// Run the scenario child with its standard streams on a fresh
    /// pseudo-terminal; everything it wrote there, read while it ran.
    pub fn run(scenario: &str) -> Vec<u8> {
        // SAFETY: libc calls on a descriptor this function has just been
        // given and wraps in a `File` before anything can fail.
        let (mut master, name) = unsafe {
            let fd = posix_openpt(O_RDWR | O_NOCTTY);
            assert!(fd >= 0, "posix_openpt: {}", io::Error::last_os_error());
            let master = File::from_raw_fd(fd);
            assert_eq!(grantpt(fd), 0, "grantpt: {}", io::Error::last_os_error());
            assert_eq!(unlockpt(fd), 0, "unlockpt: {}", io::Error::last_os_error());
            let mut buf = [0 as c_char; 256];
            assert_eq!(
                ptsname_r(fd, buf.as_mut_ptr(), buf.len()),
                0,
                "ptsname_r: {}",
                io::Error::last_os_error()
            );
            let flags = fcntl(fd, F_GETFL);
            assert!(
                fcntl(fd, F_SETFL, flags | O_NONBLOCK) >= 0,
                "F_SETFL: {}",
                io::Error::last_os_error()
            );
            (
                master,
                CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned(),
            )
        };
        let terminal = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(O_NOCTTY)
            .open(&name)
            .unwrap_or_else(|e| panic!("opening the terminal side {name}: {e}"));
        let exe = std::env::current_exe().expect("path of this test binary");
        let mut command = Command::new(exe);
        command
            .args([
                "--exact",
                "scenario_child",
                "--nocapture",
                "--test-threads",
                "1",
            ])
            .env(super::CHILD_ENV, scenario)
            .stdin(Stdio::from(terminal.try_clone().unwrap()))
            .stdout(Stdio::from(terminal.try_clone().unwrap()))
            .stderr(Stdio::from(terminal.try_clone().unwrap()));
        // SAFETY: only `setsid`, which is async-signal-safe, runs between
        // fork and exec.
        unsafe {
            command.pre_exec(|| {
                if setsid() < 0 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let mut child = Reaped(
            command
                .spawn()
                .expect("re-executing this test binary as a child"),
        );
        drop(command);
        drop(terminal);

        let mut out = Vec::new();
        let mut buf = [0u8; 16 * 1024];
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            loop {
                match master.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => out.extend_from_slice(&buf[..n]),
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                    Err(_) => break,
                }
            }
            if child.0.try_wait().expect("polling the child").is_some() {
                // Whatever it wrote on its way out.
                while let Ok(n) = master.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    out.extend_from_slice(&buf[..n]);
                }
                let _ = master.as_raw_fd();
                return out;
            }
            assert!(
                Instant::now() < deadline,
                "the {scenario} child was still running 20s later.\n{}",
                String::from_utf8_lossy(&out)
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// One DEC private mode set or reset, and where in the output it was.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct ModeChange {
        pub at: usize,
        pub mode: u32,
        pub on: bool,
    }

    /// Every DEC private mode the output sets or resets but the cursor's
    /// showing and hiding, in the order written.
    pub fn mode_changes(out: &[u8]) -> Vec<ModeChange> {
        let mut changes = Vec::new();
        let mut i = 0;
        while i + 2 < out.len() {
            if out[i] == 0x1b && out[i + 1] == b'[' && out[i + 2] == b'?' {
                let mut j = i + 3;
                while j < out.len() && (out[j].is_ascii_digit() || out[j] == b';') {
                    j += 1;
                }
                if j < out.len() && (out[j] == b'h' || out[j] == b'l') {
                    let params =
                        std::str::from_utf8(&out[i + 3..j]).expect("digits and semicolons");
                    for p in params.split(';') {
                        if let Ok(mode) = p.parse::<u32>() {
                            if mode != 25 {
                                changes.push(ModeChange {
                                    at: i,
                                    mode,
                                    on: out[j] == b'h',
                                });
                            }
                        }
                    }
                    i = j + 1;
                    continue;
                }
            }
            i += 1;
        }
        changes
    }
}

/// What the guard should have written, entering and restoring: the
/// alternate screen, then normal tracking and the SGR encoding; then, on the
/// way out, the SGR encoding off, normal tracking off, the alternate screen
/// left — each exactly once, in that order, all before `before` — and no
/// other mouse mode set or reset.
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn expect_entered_and_restored_with_reporting(out: &[u8], before: usize, what: &str) {
    use guard_pty::ModeChange;
    let changes = guard_pty::mode_changes(out);
    let mouse_and_screen: Vec<(u32, bool)> = changes
        .iter()
        .filter(|c| [9, 1000, 1001, 1002, 1003, 1005, 1006, 1015, 1016, 1049].contains(&c.mode))
        .map(|c| (c.mode, c.on))
        .collect();
    assert_eq!(
        mouse_and_screen,
        vec![
            (1049, true),
            (1000, true),
            (1006, true),
            (1006, false),
            (1000, false),
            (1049, false)
        ],
        "{what}: the guard should enter the alternate screen, turn on normal tracking and the SGR \
         encoding, and on the way out turn the SGR encoding then normal tracking off before it \
         leaves the alternate screen — and touch no other mouse mode.\n--- output ---\n{}",
        String::from_utf8_lossy(out)
    );
    let left: &ModeChange = changes.iter().rfind(|c| c.mode == 1049 && !c.on).unwrap();
    assert!(
        left.at < before,
        "{what}: the terminal was restored only after the point it should have been restored by.\n--- \
         output ---\n{}",
        String::from_utf8_lossy(out)
    );
}

/// Letting the guard go — the path a normal quit and an error return take —
/// turns reporting off with the rest of the terminal, before anything after
/// it is said.
/// @spec TUI-RUN-061, TUI-RUN-060, TUI-RUN-002
#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn letting_the_guard_go_turns_mouse_reporting_off_before_the_alternate_screen_is_left() {
    let out = guard_pty::run("guard-drop");
    let mark = out
        .windows(b"[mark] dropped".len())
        .position(|w| w == b"[mark] dropped")
        .unwrap_or_else(|| {
            panic!(
                "the guard-drop child never got past the guard\n{}",
                String::from_utf8_lossy(&out)
            )
        });
    expect_entered_and_restored_with_reporting(&out, mark, "the guard let go");
}

/// A panic with the terminal taken turns reporting off with the rest of the
/// terminal, before the panic message is printed, so the message lands on a
/// terminal that is no longer sending mouse reports.
/// @spec TUI-RUN-061, TUI-RUN-003
#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn a_panic_turns_mouse_reporting_off_before_the_panic_message() {
    let out = guard_pty::run("guard-panic");
    let message = out
        .windows(b"panicked at".len())
        .position(|w| w == b"panicked at")
        .unwrap_or_else(|| {
            panic!(
                "the guard-panic child printed no panic message\n{}",
                String::from_utf8_lossy(&out)
            )
        });
    expect_entered_and_restored_with_reporting(&out, message, "a panic");
}
