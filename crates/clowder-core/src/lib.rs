//! clowder-core: the deterministic, headless simulation for clowder.
//!
//! The shell (clowder-tui) drives ticks and applies commands; everything the
//! game *is* lives here. No terminal, no clocks, no threads — wall-clock time
//! enters only through the persistence API's caller-supplied timestamps.

mod command;
pub mod content;
mod effects;
mod events;
mod gating;
mod journal;
mod query;
mod rng;
mod save;
mod state;
mod tick;

pub use command::{Command, CommandError};
pub use effects::{limited_dr, unlimited_dr, Effect};
pub use events::{Event, EventClass};
pub use journal::{Journal, JournalEntry, JournalItem};
pub use query::{Act, Bench, FlowSource, LockReason, Offers, Source, Timer};
pub use rng::Pcg32;
pub use save::{
    begin_load, default_save_path, load_string, read_save, save_string, save_string_carrying,
    save_string_owing, write_save, write_save_carrying, write_save_owing, Advance, CatchUp,
    LoadError, LoadedGame, OfflineReport, SetDown, CATCHUP_CAP_SECONDS,
    MIN_REPORTABLE_ABSENCE_SECONDS, SAVE_VERSION,
};
pub use state::{
    AdventureId, ArchId, ArchivedKitten, Availability, ColonyId, ContentId, CraftId, DreamLoomId,
    GameState, IdeaId, JobId, Kitten, KittenTrait, Location, MoodTier, Promise, RefinementId,
    ResourceId, RiteId, RuleId, Season, SiteId, StructureGate, StructureId, TraditionId,
};
