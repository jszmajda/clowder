use crate::state::GameState;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const SAVE_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveFile {
    pub save_version: u32,
    pub saved_at_unix: u64,
    pub state: GameState,
}

#[derive(Debug)]
pub enum LoadError {
    /// No save file — the caller starts a new game.
    NotFound,
    /// The save was written by a newer clowder; never guess forward.
    TooNew { found: u32 },
    /// Unparseable JSON, missing fields, or unknown content ids. Core never
    /// deletes or rewrites the file; what to do next is the shell's call.
    Corrupt { detail: String },
}

/// What happened while you were away, folded from the catch-up replay.
/// Name lists are gross with per-name counts — a kitten may appear in more
/// than one list, and more than once per list.
// @spec SAVE-008
#[derive(Debug, Clone, PartialEq, Default)]
pub struct OfflineReport {
    pub elapsed_seconds: u64,
    pub truncated: bool,
    /// Net stock change per resource, indexed by `ResourceId::ix()`.
    pub resource_deltas: [f64; 4],
    pub arrived: Vec<(String, u32)>,
    pub wandered: Vec<(String, u32)>,
    pub returned: Vec<(String, u32)>,
    pub seasons_passed: u32,
}

#[derive(Debug)]
pub struct LoadedGame {
    pub state: GameState,
    pub report: OfflineReport,
}

/// Serialize a save to pretty JSON with a caller-supplied timestamp.
// @spec SAVE-001
pub fn save_string(_state: &GameState, _saved_at_unix: u64) -> String {
    todo!()
}

/// Atomic write: temp file in the same directory, fsync, rename.
// @spec SAVE-003
pub fn write_save(_path: &Path, _state: &GameState, _saved_at_unix: u64) -> std::io::Result<()> {
    todo!()
}

/// Parse a save and replay offline time through the standard tick().
// @spec SAVE-006
pub fn load_string(_s: &str, _now_unix: u64) -> Result<LoadedGame, LoadError> {
    todo!()
}

// @spec SAVE-004
pub fn read_save(_path: &Path, _now_unix: u64) -> Result<LoadedGame, LoadError> {
    todo!()
}

/// Platform data dir + clowder/save.json.
// @spec SAVE-012
pub fn default_save_path() -> Option<PathBuf> {
    todo!()
}
