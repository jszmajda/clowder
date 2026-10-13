//! One session per save.
//!
//! A player who lives in tmux is well placed to start clowder twice by
//! accident, and two sessions autosaving one path is silent progress loss. The
//! lock is taken *before* the save is read: acquiring it afterwards leaves a
//! window in which two launches both read the same save and both believe they
//! are alone. @spec TUI-RUN-006..009, TUI-RUN-029

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Why a launch was refused. @spec TUI-RUN-007
#[derive(Debug)]
pub enum LockError {
    /// Another live session holds the game.
    Held { pid: u32 },
    Io(std::io::Error),
}

/// Held for the life of the session; released on every exit path, including
/// the ones that refuse to start. @spec TUI-RUN-006, TUI-RUN-029
#[derive(Debug)]
pub struct SessionLock {
    path: PathBuf,
}

/// A crude, dependency-free "process id + start time" identity, derived from
/// `ps`. Encoding the start time as a single decimal (`yyyymmddhhmmss`) rather
/// than keeping the raw string is what makes two readings of the same process
/// comparable for equality.
fn month_number(name: &str) -> Option<u64> {
    Some(match name {
        "Jan" => 1,
        "Feb" => 2,
        "Mar" => 3,
        "Apr" => 4,
        "May" => 5,
        "Jun" => 6,
        "Jul" => 7,
        "Aug" => 8,
        "Sep" => 9,
        "Oct" => 10,
        "Nov" => 11,
        "Dec" => 12,
        _ => return None,
    })
}

/// Parse `ps`'s `lstart` field — a fixed, English, `ctime`-shaped string such
/// as `"Thu Jul 24 13:45:02 2026"` (guaranteed by requesting it under
/// `LC_ALL=C`) — into a single comparable decimal.
fn encode_lstart(text: &str) -> Option<u64> {
    let parts: Vec<&str> = text.split_whitespace().collect();
    let [_dow, mon, day, time, year] = parts[..] else {
        return None;
    };
    let month = month_number(mon)?;
    let day: u64 = day.parse().ok()?;
    let year: u64 = year.parse().ok()?;
    let mut time_parts = time.split(':');
    let hour: u64 = time_parts.next()?.parse().ok()?;
    let minute: u64 = time_parts.next()?.parse().ok()?;
    let second: u64 = time_parts.next()?.parse().ok()?;
    Some(
        year * 10_000_000_000
            + month * 100_000_000
            + day * 1_000_000
            + hour * 10_000
            + minute * 100
            + second,
    )
}

/// The start time of a still-living process named by `pid`, or `None` if no
/// such process can be found. Backed by `ps`, which reports nothing (and a
/// non-zero exit) for a pid that is not currently running — that absence is
/// exactly the "process is gone" signal staleness needs, with no need to
/// distinguish "never existed" from "exited" from "recycled".
fn process_start_time(pid: u32) -> Option<u64> {
    let output = Command::new("ps")
        .env("LC_ALL", "C")
        .args(["-p", &pid.to_string(), "-o", "lstart="])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    encode_lstart(text)
}

/// Where the lock for a given save lives: beside it, named from it, never
/// colliding with it.
fn lock_path_for(beside: &Path) -> PathBuf {
    let dir = beside.parent().filter(|p| !p.as_os_str().is_empty());
    let dir = dir.unwrap_or_else(|| Path::new("."));
    let save_name = beside
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("clowder-save");
    dir.join(format!("{save_name}.lock"))
}

/// The two fields recorded in a lock file: the owning process's id and its
/// start time, encoded as `key=value` lines. The exact format is this
/// module's private choice; nothing outside it should depend on it.
fn render_lock(pid: u32, start: u64) -> String {
    format!("pid={pid}\nstart={start}\n")
}

/// Parse a lock file written by `render_lock` (or by anything shaped the same
/// way). `None` for anything that does not parse — treated the same as a lock
/// with no living owner, since a lock this module cannot make sense of is not
/// evidence of a live session either.
fn parse_lock(text: &str) -> Option<(u32, u64)> {
    let mut pid = None;
    let mut start = None;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("pid=") {
            pid = value.trim().parse().ok();
        } else if let Some(value) = line.strip_prefix("start=") {
            start = value.trim().parse().ok();
        }
    }
    Some((pid?, start?))
}

/// Attempt to create the lock file exclusively — the OS-level atomic
/// operation that makes simultaneous acquire attempts resolve to exactly one
/// winner. `Ok(true)` created it; `Ok(false)` means it already existed;
/// anything else is a genuine I/O failure.
fn try_create_exclusive(path: &Path, contents: &str) -> io::Result<bool> {
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(mut file) => {
            use std::io::Write;
            file.write_all(contents.as_bytes())?;
            Ok(true)
        }
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Ok(false),
        Err(e) => Err(e),
    }
}

/// Held while a launch decides whether the save is free. Reading a lock,
/// judging it stale, clearing it and writing a new one are several steps, and
/// two launches interleaving them each clear what the other has just written
/// and both come away holding the save. An advisory lock on the save's
/// directory makes the decision one step: the kernel grants it to one launch
/// at a time and lets it go when that launch returns or dies, so it cannot
/// itself go stale. It is taken on the directory, not on a file of its own, so
/// that nothing but the session lock ever sits beside the save.
///
/// Best effort: on a filesystem that cannot grant it, launches decide
/// unguarded, which is no worse than having no guard and better than refusing
/// a player their own game.
fn launch_guard(lock_path: &Path) -> Option<fs::File> {
    let dir = lock_path.parent().filter(|p| !p.as_os_str().is_empty());
    let dir = fs::File::open(dir.unwrap_or_else(|| Path::new("."))).ok()?;
    dir.lock().ok()?;
    Some(dir)
}

impl SessionLock {
    /// Take the lock beside the save, recording this process's id and start
    /// time. A stale lock — one whose process is gone, or whose id now belongs
    /// to a process with a different start time — is cleared and play
    /// continues: a crash must not lock a player out of their own game, and an
    /// id alone can be recycled by something unrelated.
    /// @spec TUI-RUN-006, TUI-RUN-008, TUI-RUN-009, TUI-RUN-029
    pub fn acquire(beside: &Path, force: bool) -> Result<Self, LockError> {
        let path = lock_path_for(beside);
        // A first run has no data directory yet — nothing has written a save
        // to create it, and the lock is deliberately taken before the save is
        // read. Without this, a brand-new player is refused at the door.
        // @spec TUI-RUN-010
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(LockError::Io)?;
            }
        }
        let my_pid = std::process::id();
        // If we cannot identify ourselves, we must not write a lock at all:
        // a recorded start of 0 mismatches every real start time, so every
        // later launcher would read this LIVE session's lock as stale and
        // clear it. Refusing is the safe failure; a spurious lockout is
        // recoverable with --force, a silently cleared live lock is not.
        let Some(my_start) = process_start_time(my_pid) else {
            return Err(LockError::Io(std::io::Error::other(
                "could not identify this process, so a session lock would be unsafe to write",
            )));
        };
        let contents = render_lock(my_pid, my_start);

        // One launch at a time from here to the return, so no launch reads a
        // lock another is halfway through writing or clears one another has
        // just taken.
        let _deciding = launch_guard(&path);

        loop {
            match try_create_exclusive(&path, &contents) {
                Ok(true) => return Ok(SessionLock { path }),
                Ok(false) => {
                    // Someone else's lock is already there. Force always wins;
                    // otherwise the decision is whether it is still live.
                    if force {
                        let _ = fs::remove_file(&path);
                        continue;
                    }

                    let existing = fs::read_to_string(&path);
                    let existing = match existing {
                        Ok(text) => text,
                        // The file vanished between the failed create and this
                        // read (another launch's lock came and went) — try again.
                        Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
                        Err(e) => return Err(LockError::Io(e)),
                    };

                    match parse_lock(&existing) {
                        Some((holder_pid, holder_start))
                            if process_start_time(holder_pid) == Some(holder_start) =>
                        {
                            // Same id, same start time: genuinely still that process.
                            return Err(LockError::Held { pid: holder_pid });
                        }
                        _ => {
                            // Gone, recycled, or unparseable: stale. Clear it and
                            // retry — another launch may win the race, in which
                            // case its lock is live and this loop will refuse
                            // against it on the next pass.
                            let _ = fs::remove_file(&path);
                            continue;
                        }
                    }
                }
                Err(e) => return Err(LockError::Io(e)),
            }
        }
    }
}

impl Drop for SessionLock {
    fn drop(&mut self) {
        // @spec TUI-RUN-006
        let _ = fs::remove_file(&self.path);
    }
}
