//! Loading, saving, and what to do when a save will not load.
//!
//! The core reports three distinct failures and deliberately never rewrites
//! the file, leaving the policy here. @spec TUI-RUN-005, TUI-RUN-010..013,
//! TUI-RUN-026..028, TUI-RUN-031

use std::path::PathBuf;

/// Why startup stopped. @spec TUI-RUN-011, TUI-RUN-013
#[derive(Debug)]
pub enum StartupRefusal {
    /// Written by a newer clowder. Guessing forward would corrupt a save that
    /// is currently fine. The file is not touched. @spec TUI-RUN-011
    SaveIsNewer { found: u32 },
    /// Present but unreadable — a permission error, a path that is not a file,
    /// a transient I/O failure. Unreadable is *not* unparseable: the contents
    /// are unknown, and moving aside or replacing a file we merely could not
    /// open would destroy a perfectly good save. @spec TUI-RUN-013
    Unreadable { path: PathBuf, detail: String },
}

/// Resolve the save path, falling back beside the working directory where no
/// platform data directory resolves — and saying where it landed rather than
/// refusing to start. @spec TUI-RUN-005
pub fn resolve_save_path() -> PathBuf {
    clowder_core::default_save_path().unwrap_or_else(|| PathBuf::from("clowder-save.json"))
}

/// Move an unparseable save aside under a suffixed name, returning where it
/// went. Never overwritten in place: saves are human-readable precisely so a
/// broken one can be hand-rescued, and a shell that replaces it with a fresh
/// game destroys the only artifact that made that promise real.
/// @spec TUI-RUN-012
pub fn set_aside(path: &std::path::Path) -> std::io::Result<PathBuf> {
    let mut name = path
        .file_name()
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "save path has no file name",
            )
        })?
        .to_os_string();
    name.push(".broken");
    let base = path.with_file_name(&name);
    // A fixed suffix would rename over the previous rescue — destroying the one
    // file the player was told was safe, which is the whole point of moving it
    // aside rather than overwriting it.
    let mut moved_to = base.clone();
    let mut n = 1u32;
    while moved_to.exists() {
        let mut numbered = name.clone();
        numbered.push(format!(".{n}"));
        moved_to = path.with_file_name(numbered);
        n += 1;
    }
    std::fs::rename(path, &moved_to)?;
    Ok(moved_to)
}

/// The wall-clock interval between autosaves. The exact value is an open
/// tuning question per the runtime design's deferred decisions; this is a
/// placeholder pending measurement against a real long session.
pub const AUTOSAVE_INTERVAL: std::time::Duration = std::time::Duration::from_secs(60);

/// Whether the autosave cadence has elapsed since `since_last_save`.
/// @spec TUI-RUN-026
pub fn autosave_due(since_last_save: std::time::Duration) -> bool {
    since_last_save >= AUTOSAVE_INTERVAL
}
