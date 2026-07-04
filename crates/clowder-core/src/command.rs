use crate::events::Event;
use crate::state::{GameState, IdeaId, JobId, ResourceId, StructureId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Command {
    GatherCatnip,
    Forage,
    Build(StructureId),
    AssignJob(JobId),
    UnassignJob(JobId),
    ResearchIdea(IdeaId),
}

/// Why a command was rejected. Listed in precedence order: when several
/// reasons apply, the earliest variant is reported.
// @spec SIM-ENG-006
#[derive(Debug, Clone, PartialEq)]
pub enum CommandError {
    UnknownId,
    Locked,
    AlreadyResearched,
    NoIdleKittens,
    NoWorkersAssigned,
    StorageFull,
    InsufficientResources { missing: Vec<(ResourceId, f64)> },
}

impl GameState {
    /// Apply a command between ticks. All-or-nothing: a rejected command has
    /// mutated nothing, RNG included.
    // @spec SIM-ENG-005
    pub fn apply(&mut self, _cmd: Command) -> Result<Vec<Event>, CommandError> {
        todo!()
    }
}
