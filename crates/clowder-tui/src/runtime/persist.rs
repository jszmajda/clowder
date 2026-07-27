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
    todo!("TUI-RUN-005: platform data dir, else a stated fallback")
}

/// Move an unparseable save aside under a suffixed name, returning where it
/// went. Never overwritten in place: saves are human-readable precisely so a
/// broken one can be hand-rescued, and a shell that replaces it with a fresh
/// game destroys the only artifact that made that promise real.
/// @spec TUI-RUN-012
pub fn set_aside(_path: &std::path::Path) -> std::io::Result<PathBuf> {
    todo!("TUI-RUN-012: rename aside, report the new path")
}

/// Whether the autosave cadence has elapsed. @spec TUI-RUN-026
pub fn autosave_due() -> bool {
    todo!("TUI-RUN-026: wall-clock cadence")
}
