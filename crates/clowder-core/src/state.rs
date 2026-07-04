use crate::rng::Pcg32;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceId {
    Catnip,
    Twigs,
    Fluff,
    Curiosity,
}

impl ResourceId {
    pub const ALL: [ResourceId; 4] = [
        ResourceId::Catnip,
        ResourceId::Twigs,
        ResourceId::Fluff,
        ResourceId::Curiosity,
    ];
    pub fn ix(self) -> usize {
        self as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StructureId {
    CatnipPatch,
    CozyCushion,
    CardboardBox,
    Pantry,
    TwigPile,
    ScratchingPost,
    CatTree,
}

impl StructureId {
    pub const ALL: [StructureId; 7] = [
        StructureId::CatnipPatch,
        StructureId::CozyCushion,
        StructureId::CardboardBox,
        StructureId::Pantry,
        StructureId::TwigPile,
        StructureId::ScratchingPost,
        StructureId::CatTree,
    ];
    pub fn ix(self) -> usize {
        self as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum JobId {
    Gardener,
    TwigCollector,
    Groomer,
    Observer,
}

impl JobId {
    pub const ALL: [JobId; 4] = [
        JobId::Gardener,
        JobId::TwigCollector,
        JobId::Groomer,
        JobId::Observer,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum IdeaId {
    PantryOrganization,
    ScratchingArts,
    GroomedToPurrfection,
    TwigArchitecture,
    SelectiveHearing,
    BiscuitMaking,
    SeasonalAwareness,
    ZoomieChanneling,
    NeighborhoodWatch,
}

impl IdeaId {
    pub const ALL: [IdeaId; 9] = [
        IdeaId::PantryOrganization,
        IdeaId::ScratchingArts,
        IdeaId::GroomedToPurrfection,
        IdeaId::TwigArchitecture,
        IdeaId::SelectiveHearing,
        IdeaId::BiscuitMaking,
        IdeaId::SeasonalAwareness,
        IdeaId::ZoomieChanneling,
        IdeaId::NeighborhoodWatch,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Season {
    Spring,
    Summer,
    Autumn,
    Winter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Location {
    Home,
    NeighborsYard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MoodTier {
    Blissful,
    Content,
    Restless,
    Fussy,
    Grumpy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kitten {
    pub name: String,
    pub job: Option<JobId>,
    pub location: Location,
    pub arrived_day: u64,
    pub job_seq: Option<u64>,
    pub away_seq: Option<u64>,
}

/// The single source of truth for a game. Canonical state only — everything
/// derivable (happiness, rates, season) is computed on demand. All collections
/// serialize in canonical order: enum-indexed arrays, append-ordered roster,
/// sorted idea set.
// @spec SIM-ENG-007
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameState {
    pub ticks: u64,
    pub rng: Pcg32,
    /// Indexed by `ResourceId::ix()`.
    pub resources: [f64; 4],
    /// Indexed by `StructureId::ix()`.
    pub structures: [u32; 7],
    pub kittens: Vec<Kitten>,
    pub ideas: BTreeSet<IdeaId>,
    pub pity: u8,
    pub famine_days: u8,
    pub recovery_days: u8,
    pub grumpy: bool,
    pub next_seq: u64,
    pub zoomies_until: u64,
}

impl GameState {
    /// A fresh, empty meadow.
    // @spec SIM-ENG-016
    pub fn new(seed: u64) -> Self {
        GameState {
            ticks: 0,
            rng: Pcg32::new(seed),
            resources: [0.0; 4],
            structures: [0; 7],
            kittens: Vec::new(),
            ideas: BTreeSet::new(),
            pity: 0,
            famine_days: 0,
            recovery_days: 0,
            grumpy: false,
            next_seq: 0,
            zoomies_until: 0,
        }
    }

    // ---- Calendar (derived from ticks alone) ----
    // @spec SIM-ENG-001

    pub fn day(&self) -> u64 {
        todo!()
    }

    pub fn season(&self) -> Season {
        todo!()
    }

    pub fn year(&self) -> u64 {
        todo!()
    }

    /// The upcoming season — Some only once the season-forecast idea is researched.
    // @spec SIM-ENG-017
    pub fn season_forecast(&self) -> Option<Season> {
        todo!()
    }

    // ---- Derived economy queries ----

    /// Storage cap for a resource, including structure extensions.
    // @spec SIM-ECO-001, SIM-ECO-002
    pub fn cap(&self, _r: ResourceId) -> f64 {
        todo!()
    }

    pub fn stock(&self, r: ResourceId) -> f64 {
        self.resources[r.ix()]
    }

    /// Cost of the next copy of a structure: ceil(base × ratio^owned) per component.
    // @spec SIM-ECO-007
    pub fn next_cost(&self, _s: StructureId) -> Vec<(ResourceId, f64)> {
        todo!()
    }

    pub fn structure_unlocked(&self, _s: StructureId) -> bool {
        todo!()
    }

    pub fn job_unlocked(&self, _j: JobId) -> bool {
        todo!()
    }

    pub fn idea_unlocked(&self, _i: IdeaId) -> bool {
        todo!()
    }

    // ---- Derived kitten queries ----

    /// Total housing slots from structures.
    pub fn housing_capacity(&self) -> u32 {
        todo!()
    }

    /// Capacity minus the whole roster — every kitten holds a slot, Home or Away.
    // @spec SIM-KIT-006
    pub fn free_slots(&self) -> u32 {
        todo!()
    }

    /// Home kittens holding this job.
    // @spec SIM-KIT-010
    pub fn effective_workers(&self, _j: JobId) -> u32 {
        todo!()
    }

    /// Happiness in percentage points (floored at 25, no ceiling).
    // @spec SIM-KIT-011
    pub fn happiness(&self) -> f64 {
        todo!()
    }

    /// Multiplier applied to job production: happiness/100, ×0.25 while Grumpy.
    // @spec SIM-KIT-012
    pub fn mood_multiplier(&self) -> f64 {
        todo!()
    }

    // @spec SIM-KIT-014
    pub fn mood_tier(&self) -> MoodTier {
        todo!()
    }

    /// Probability the next *eligible* arrival check succeeds (before pity).
    // @spec SIM-KIT-004
    pub fn arrival_chance(&self) -> f64 {
        todo!()
    }

    pub fn zoomies_active(&self) -> bool {
        todo!()
    }
}
