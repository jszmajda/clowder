//! Loading, saving, and what to do when a save will not load.
//!
//! The core reports three distinct failures and deliberately never rewrites
//! the file, leaving the policy here.

use std::path::{Path, PathBuf};

/// Resolve the save path, falling back beside the working directory where no
/// platform data directory resolves, rather than refusing to start.
/// @spec TUI-RUN-005
pub fn resolve_save_path() -> PathBuf {
    save_path_given(
        clowder_core::default_save_path(),
        std::env::current_dir().ok(),
    )
}

/// The save path, given what the platform offers and where the game started.
/// Apart from [`resolve_save_path`] because no environment a test can build
/// makes the platform offer nothing: the home directory falls back to the
/// system's user database.
pub fn save_path_given(platform: Option<PathBuf>, working_dir: Option<PathBuf>) -> PathBuf {
    platform.unwrap_or_else(|| working_dir.unwrap_or_default().join("clowder-save.json"))
}

/// What to tell the player where the save has fallen back, so they can find it
/// again; nothing where the platform gave it a home. @spec TUI-RUN-005
pub fn fallback_notice(platform_resolves: bool, save_path: &Path) -> Option<String> {
    (!platform_resolves).then(|| {
        format!(
            "clowder found no data directory on this system, so it is keeping your meadow at {}.",
            save_path.display()
        )
    })
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
