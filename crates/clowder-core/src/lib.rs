//! clowder-core: the deterministic, headless simulation for clowder.
//!
//! The shell (clowder-tui) drives ticks and applies commands; everything the
//! game *is* lives here. No terminal, no clocks, no threads — wall-clock time
//! enters only through the persistence API's caller-supplied timestamps.

pub mod content;
mod command;
mod effects;
mod events;
mod rng;
mod save;
mod state;
mod tick;

pub use command::{Command, CommandError};
pub use effects::{limited_dr, unlimited_dr, Effect};
pub use events::Event;
pub use rng::Pcg32;
pub use save::{
    default_save_path, load_string, read_save, save_string, write_save, LoadError, LoadedGame,
    OfflineReport, SAVE_VERSION,
};
pub use state::{
    Availability, ContentId, CraftId, GameState, IdeaId, JobId, Kitten, KittenTrait, Location,
    MoodTier, RefinementId, ResourceId, RuleId, Season, StructureId,
};
