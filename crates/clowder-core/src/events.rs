use crate::state::{IdeaId, JobId, MoodTier, Season, StructureId};
use serde::{Deserialize, Serialize};

/// Typed simulation occurrences returned by `tick()` and command application.
/// Events carry data, never prose — presentation belongs to the shell.
// @spec SIM-ENG-009
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Event {
    KittenArrived { name: String },
    KittenWanderedOff { name: String },
    KittenReturned { name: String },
    JobAssigned { name: String, job: JobId },
    JobUnassigned { name: String, job: JobId },
    SeasonChanged { season: Season },
    NewYear { year: u64 },
    StructureBuilt { id: StructureId, count: u32 },
    IdeaResearched { id: IdeaId },
    ZoomiesStarted { until_tick: u64 },
    ZoomiesEnded,
    KittenMood { tier: MoodTier },
    /// Index into `content::FLAVOR_TEMPLATES`, plus the featured kitten when
    /// the template wants one. No mechanical effect by construction.
    Flavor { template: usize, kitten: Option<String> },
}
