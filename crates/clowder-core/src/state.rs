use crate::content;
use crate::rng::Pcg32;
use serde::{Deserialize, Deserializer, Serialize};
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

impl Season {
    pub const ALL: [Season; 4] = [Season::Spring, Season::Summer, Season::Autumn, Season::Winter];
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

/// Deserialize a counter wide and clamp — hand-edited saves get physics, not
/// type errors.
// @spec SAVE-005
fn saturating_u8<'de, D: Deserializer<'de>>(d: D) -> Result<u8, D::Error> {
    let wide = u64::deserialize(d)?;
    Ok(wide.min(u8::MAX as u64) as u8)
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
    #[serde(deserialize_with = "saturating_u8")]
    pub pity: u8,
    #[serde(deserialize_with = "saturating_u8")]
    pub famine_days: u8,
    #[serde(deserialize_with = "saturating_u8")]
    pub recovery_days: u8,
    pub grumpy: bool,
    pub next_seq: u64,
    pub zoomies_until: u64,
}

/// Dust guard for zero-vs-nonzero and affordability comparisons; float
/// residue is never load-bearing.
// @spec SIM-ECO-022
pub(crate) const EPSILON: f64 = 1e-9;

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
        self.ticks / content::TICKS_PER_DAY
    }

    pub fn season(&self) -> Season {
        season_of_day(self.day())
    }

    pub fn year(&self) -> u64 {
        self.day() / content::DAYS_PER_YEAR
    }

    /// The upcoming season — Some only once the season-forecast idea is researched.
    // @spec SIM-ENG-017
    pub fn season_forecast(&self) -> Option<Season> {
        if self.ideas.contains(&IdeaId::SeasonalAwareness) {
            let next = (season_index_of_day(self.day()) + 1) % 4;
            Some(Season::ALL[next])
        } else {
            None
        }
    }

    // ---- Derived economy queries ----

    /// Storage cap for a resource, including structure extensions.
    // @spec SIM-ECO-001, SIM-ECO-002
    pub fn cap(&self, r: ResourceId) -> f64 {
        let mut cap = content::BASE_CAPS[r.ix()];
        for s in StructureId::ALL {
            if let Some((cr, amount)) = content::structure_def(s).cap_bonus {
                if cr == r {
                    cap += amount * self.structures[s.ix()] as f64;
                }
            }
        }
        cap
    }

    pub fn stock(&self, r: ResourceId) -> f64 {
        self.resources[r.ix()]
    }

    /// Cost of the next copy of a structure: ceil(base × ratio^owned) per component.
    // @spec SIM-ECO-007
    pub fn next_cost(&self, s: StructureId) -> Vec<(ResourceId, f64)> {
        let def = content::structure_def(s);
        let owned = self.structures[s.ix()] as i32;
        def.base_costs
            .iter()
            .map(|(r, base)| (*r, (base * def.ratio.powi(owned)).ceil()))
            .collect()
    }

    pub fn structure_unlocked(&self, s: StructureId) -> bool {
        match content::structure_def(s).gate {
            None => true,
            Some(idea) => self.ideas.contains(&idea),
        }
    }

    pub fn job_unlocked(&self, j: JobId) -> bool {
        match content::job_def(j).gate {
            content::JobGate::Always => true,
            content::JobGate::Idea(idea) => self.ideas.contains(&idea),
            content::JobGate::RequiresBox => self.structures[StructureId::CardboardBox.ix()] >= 1,
        }
    }

    /// Ideas open once curiosity storage exists (the box is the gate) and the
    /// prerequisite is researched.
    // @spec SIM-CUR-001, SIM-CUR-002
    pub fn idea_unlocked(&self, i: IdeaId) -> bool {
        if self.cap(ResourceId::Curiosity) <= 0.0 {
            return false;
        }
        match content::idea_def(i).prereq {
            None => true,
            Some(prereq) => self.ideas.contains(&prereq),
        }
    }

    // ---- Derived kitten queries ----

    /// Total housing slots from structures.
    pub fn housing_capacity(&self) -> u32 {
        StructureId::ALL
            .iter()
            .map(|s| content::structure_def(*s).housing * self.structures[s.ix()])
            .sum()
    }

    /// Capacity minus the whole roster — every kitten holds a slot, Home or Away.
    // @spec SIM-KIT-006
    pub fn free_slots(&self) -> u32 {
        self.housing_capacity().saturating_sub(self.kittens.len() as u32)
    }

    /// Home kittens holding this job.
    // @spec SIM-KIT-010
    pub fn effective_workers(&self, j: JobId) -> u32 {
        self.kittens
            .iter()
            .filter(|k| k.location == Location::Home && k.job == Some(j))
            .count() as u32
    }

    fn home_count(&self) -> u32 {
        self.kittens.iter().filter(|k| k.location == Location::Home).count() as u32
    }

    /// Happiness in percentage points (floored at 25, no ceiling).
    // @spec SIM-KIT-011
    pub fn happiness(&self) -> f64 {
        let home = self.home_count();
        let loafers = self
            .kittens
            .iter()
            .filter(|k| k.location == Location::Home && k.job.is_none())
            .count() as f64;
        let crowding = home.saturating_sub(content::CROWDING_FREE) as f64;
        let posts = self.structures[StructureId::ScratchingPost.ix()] as f64;
        let biscuits = if self.ideas.contains(&IdeaId::BiscuitMaking) {
            content::BISCUIT_POINTS
        } else {
            0.0
        };
        let h = 100.0 - content::CROWDING_POINTS * crowding
            + content::POST_POINTS * posts
            + (content::LOAFER_POINTS * loafers).min(content::LOAFER_POINTS_MAX)
            + biscuits;
        h.max(content::HAPPINESS_FLOOR)
    }

    /// Multiplier applied to job production: happiness/100, ×0.25 while Grumpy.
    // @spec SIM-KIT-012
    pub fn mood_multiplier(&self) -> f64 {
        let base = self.happiness() / 100.0;
        if self.grumpy {
            base * content::GRUMPY_MULTIPLIER
        } else {
            base
        }
    }

    // @spec SIM-KIT-014
    pub fn mood_tier(&self) -> MoodTier {
        if self.grumpy {
            return MoodTier::Grumpy;
        }
        let h = self.happiness();
        if h >= 120.0 {
            MoodTier::Blissful
        } else if h >= 90.0 {
            MoodTier::Content
        } else if h >= 60.0 {
            MoodTier::Restless
        } else {
            MoodTier::Fussy
        }
    }

    /// Probability the next *eligible* arrival check succeeds (before pity).
    // @spec SIM-KIT-004
    pub fn arrival_chance(&self) -> f64 {
        let hundreds = (self.stock(ResourceId::Catnip) / 100.0).floor();
        let mut chance = (content::ARRIVAL_BASE_CHANCE + content::ARRIVAL_PER_100_CATNIP * hundreds)
            .min(content::ARRIVAL_CHANCE_CAP);
        if self.ideas.contains(&IdeaId::NeighborhoodWatch) {
            chance += content::ARRIVAL_WATCH_BONUS;
        }
        chance
    }

    pub fn zoomies_active(&self) -> bool {
        self.ticks < self.zoomies_until
    }
}

pub(crate) fn season_index_of_day(day: u64) -> usize {
    ((day / content::DAYS_PER_SEASON) % 4) as usize
}

pub(crate) fn season_of_day(day: u64) -> Season {
    Season::ALL[season_index_of_day(day)]
}
