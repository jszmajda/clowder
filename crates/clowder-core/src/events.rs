use crate::state::{ContentId, IdeaId, JobId, MoodTier, RefinementId, RuleId, Season, StructureId};
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
    /// A content item became permanently visible. @spec SIM-ENG-023
    Revealed { id: ContentId },
    /// Something bright crossed the sky and nobody happened to be looking —
    /// observable via ObserveSky until the end of the next day.
    /// @spec SIM-ENG-025
    SkyEventVisible,
    // @spec SIM-ENG-024
    SpeedChanged { speed: f64 },
    /// A pounce came home with its full haul — data, not prose. Shinies are
    /// reported post-clamp, exactly as the stock moved.
    /// @spec SIM-EXP-005
    PounceReturned {
        squads: u32,
        feathers: f64,
        bells: f64,
        shinies: f64,
        unicorns: u32,
    },
    // @spec SIM-KIT-025
    JamboreeStarted { until_tick: u64 },
    JamboreeEnded,
    // @spec SIM-KIT-022
    ElderAppointed { name: String },
    // @spec SIM-KIT-021
    KittenPromoted { name: String, rank: u8 },
    // @spec SIM-CUR-010
    RefinementBought { id: RefinementId },
    // @spec SIM-CUR-011
    RuleAdopted { id: RuleId },
}
