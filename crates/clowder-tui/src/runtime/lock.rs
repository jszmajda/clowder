//! One session per save.
//!
//! A player who lives in tmux is well placed to start clowder twice by
//! accident, and two sessions autosaving one path is silent progress loss. The
//! lock is taken *before* the save is read: acquiring it afterwards leaves a
//! window in which two launches both read the same save and both believe they
//! are alone. @spec TUI-RUN-006..009, TUI-RUN-029

use std::path::Path;

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
    _private: (),
}

impl SessionLock {
    /// Take the lock beside the save, recording this process's id and start
    /// time. A stale lock — one whose process is gone, or whose id now belongs
    /// to a process with a different start time — is cleared and play
    /// continues: a crash must not lock a player out of their own game, and an
    /// id alone can be recycled by something unrelated.
    /// @spec TUI-RUN-006, TUI-RUN-008
    pub fn acquire(_beside: &Path, _force: bool) -> Result<Self, LockError> {
        todo!("TUI-RUN-006/008/009: acquire, detect stale by pid+start time, force")
    }
}

impl Drop for SessionLock {
    fn drop(&mut self) {
        // @spec TUI-RUN-006
    }
}
