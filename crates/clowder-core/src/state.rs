use crate::content;
use crate::rng::Pcg32;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeSet;

/// The full RES table, in id order. Append-only: serialized as an
/// enum-indexed array, so new ids only ever append. @spec SIM-ECO-001
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ResourceId {
    Catnip,
    Twigs,
    Fluff,
    Curiosity,
    Pebbles,
    Buttons,
    Embers,
    Shinies,
    Dew,
    Moonstone,
    Stardust,
    Hushfelt,
    Pep,
    Charm,
    Warmth,
    Skymaps,
    Feathers,
    Bells,
    Mint,
    Unicorns,
    Rainbows,
    Prisms,
    Starshards,
    Echoes,
    Dreamcorns,
    JarsOfPurrs,
    Reverie,
    Wands,
    Tiles,
    Badges,
    Brass,
    Cogs,
    Moonsteel,
    Lattices,
    Pavers,
    Quills,
    Stories,
    Anthologies,
    Schemes,
    Cairns,
    Kites,
    Balloons,
    Moonmilk,
    Dreamweave,
    Cometsilk,
    Cleverness,
}

pub const RESOURCE_COUNT: usize = 46;

impl ResourceId {
    pub const ALL: [ResourceId; RESOURCE_COUNT] = [
        ResourceId::Catnip,
        ResourceId::Twigs,
        ResourceId::Fluff,
        ResourceId::Curiosity,
        ResourceId::Pebbles,
        ResourceId::Buttons,
        ResourceId::Embers,
        ResourceId::Shinies,
        ResourceId::Dew,
        ResourceId::Moonstone,
        ResourceId::Stardust,
        ResourceId::Hushfelt,
        ResourceId::Pep,
        ResourceId::Charm,
        ResourceId::Warmth,
        ResourceId::Skymaps,
        ResourceId::Feathers,
        ResourceId::Bells,
        ResourceId::Mint,
        ResourceId::Unicorns,
        ResourceId::Rainbows,
        ResourceId::Prisms,
        ResourceId::Starshards,
        ResourceId::Echoes,
        ResourceId::Dreamcorns,
        ResourceId::JarsOfPurrs,
        ResourceId::Reverie,
        ResourceId::Wands,
        ResourceId::Tiles,
        ResourceId::Badges,
        ResourceId::Brass,
        ResourceId::Cogs,
        ResourceId::Moonsteel,
        ResourceId::Lattices,
        ResourceId::Pavers,
        ResourceId::Quills,
        ResourceId::Stories,
        ResourceId::Anthologies,
        ResourceId::Schemes,
        ResourceId::Cairns,
        ResourceId::Kites,
        ResourceId::Balloons,
        ResourceId::Moonmilk,
        ResourceId::Dreamweave,
        ResourceId::Cometsilk,
        ResourceId::Cleverness,
    ];
    pub fn ix(self) -> usize {
        self as usize
    }
}

/// The full BLD table, in id order. Append-only, like resources.
// @spec SIM-ECO-028
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum StructureId {
    CatnipPatch,
    CozyCushion,
    CardboardBox,
    Pantry,
    TwigPile,
    ScratchingPost,
    CatTree,
    WindowPerch,
    HerbGarden,
    CouchMine,
    GravelPit,
    Hearth,
    Kiln,
    Shed,
    Porch,
    StudyNook,
    StargazingLedge,
    WhiskerLab,
    CraftCorner,
    ToyWorkshop,
    GiftStand,
    StoryCircle,
    SunRoom,
    SunbeamAltar,
    TreatBakery,
    TeaHouse,
    DewCollector,
    Cogworks,
    DynamoWheel,
    SunningDeck,
    StreamWheel,
    StarBasin,
    PrismSpinner,
    BoxLabyrinth,
    MoonlightStage,
    MemoryChest,
    YarnComputer,
    UnicornMeadow,
}

pub const STRUCTURE_COUNT: usize = 38;

impl StructureId {
    pub const ALL: [StructureId; STRUCTURE_COUNT] = [
        StructureId::CatnipPatch,
        StructureId::CozyCushion,
        StructureId::CardboardBox,
        StructureId::Pantry,
        StructureId::TwigPile,
        StructureId::ScratchingPost,
        StructureId::CatTree,
        StructureId::WindowPerch,
        StructureId::HerbGarden,
        StructureId::CouchMine,
        StructureId::GravelPit,
        StructureId::Hearth,
        StructureId::Kiln,
        StructureId::Shed,
        StructureId::Porch,
        StructureId::StudyNook,
        StructureId::StargazingLedge,
        StructureId::WhiskerLab,
        StructureId::CraftCorner,
        StructureId::ToyWorkshop,
        StructureId::GiftStand,
        StructureId::StoryCircle,
        StructureId::SunRoom,
        StructureId::SunbeamAltar,
        StructureId::TreatBakery,
        StructureId::TeaHouse,
        StructureId::DewCollector,
        StructureId::Cogworks,
        StructureId::DynamoWheel,
        StructureId::SunningDeck,
        StructureId::StreamWheel,
        StructureId::StarBasin,
        StructureId::PrismSpinner,
        StructureId::BoxLabyrinth,
        StructureId::MoonlightStage,
        StructureId::MemoryChest,
        StructureId::YarnComputer,
        StructureId::UnicornMeadow,
    ];
    pub fn ix(self) -> usize {
        self as usize
    }
}

// @spec SIM-ECO-040, SIM-ECO-041, SIM-ECO-042, SIM-ECO-043
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum JobId {
    Gardener,
    TwigCollector,
    Groomer,
    Observer,
    Rummager,
    Pouncer,
    Sunwarden,
    Tinkerer,
}

impl JobId {
    pub const ALL: [JobId; 8] = [
        JobId::Gardener,
        JobId::TwigCollector,
        JobId::Groomer,
        JobId::Observer,
        JobId::Rummager,
        JobId::Pouncer,
        JobId::Sunwarden,
        JobId::Tinkerer,
    ];
}

/// The CRAFT table's recipe ids, in table order.
// @spec SIM-ECO-034
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CraftId {
    Wands,
    Tiles,
    Badges,
    Brass,
    Cogs,
    Moonsteel,
    Lattices,
    Pavers,
    Quills,
    Stories,
    Anthologies,
    Schemes,
    Cairns,
    Kites,
    Balloons,
    Moonmilk,
    Dreamweave,
    Cometsilk,
}

impl CraftId {
    pub const ALL: [CraftId; 18] = [
        CraftId::Wands,
        CraftId::Tiles,
        CraftId::Badges,
        CraftId::Brass,
        CraftId::Cogs,
        CraftId::Moonsteel,
        CraftId::Lattices,
        CraftId::Pavers,
        CraftId::Quills,
        CraftId::Stories,
        CraftId::Anthologies,
        CraftId::Schemes,
        CraftId::Cairns,
        CraftId::Kites,
        CraftId::Balloons,
        CraftId::Moonmilk,
        CraftId::Dreamweave,
        CraftId::Cometsilk,
    ];
}

/// The banded idea tree, ids append-only: 0–8 are the founding nine, 9–17
/// finish band 0, 18–27 band 1, 28–35 band 2, 36–43 band 3, 44–55 band 4,
/// and 56 (Pouncing Practice) is a band-0 idea appended late — ids never
/// reorder. @spec SIM-CUR-005
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
    CouchSpelunking,
    Horticulture,
    Emberkeeping,
    Notetaking,
    CozyCarpentry,
    Tinkering,
    PackRattery,
    UnicornLore,
    OralTradition,
    GiftEconomy,
    Storycraft,
    SunSalutations,
    Excavation,
    Cogwheels,
    TreatAlchemy,
    TeaCeremonies,
    SolarDevotion,
    SkyCharts,
    PorchLogistics,
    AnthologyBinding,
    DewHarvest,
    Toymaking,
    CeramicArts,
    DeepCuriosity,
    MoonlightArts,
    LabyrinthTheory,
    GrandSchemes,
    Dynamos,
    SolarBasking,
    Millworks,
    Moonmetallurgy,
    Starcatching,
    PrismCraft,
    BalloonCraft,
    YarnComputing,
    ParadeRoutes,
    PenPalNetworks,
    ScoutPicnicking,
    DreamTending,
    NightCourtEtiquette,
    HushfeltSeeking,
    MemoryWeaving,
    SkywardLonging,
    Dreamweaving,
    AdventurousSpirit,
    DreamLoomcraft,
    DeepNapTheory,
    PouncingPractice,
}

pub const IDEA_COUNT: usize = 57;

impl IdeaId {
    pub const ALL: [IdeaId; IDEA_COUNT] = [
        IdeaId::PantryOrganization,
        IdeaId::ScratchingArts,
        IdeaId::GroomedToPurrfection,
        IdeaId::TwigArchitecture,
        IdeaId::SelectiveHearing,
        IdeaId::BiscuitMaking,
        IdeaId::SeasonalAwareness,
        IdeaId::ZoomieChanneling,
        IdeaId::NeighborhoodWatch,
        IdeaId::CouchSpelunking,
        IdeaId::Horticulture,
        IdeaId::Emberkeeping,
        IdeaId::Notetaking,
        IdeaId::CozyCarpentry,
        IdeaId::Tinkering,
        IdeaId::PackRattery,
        IdeaId::UnicornLore,
        IdeaId::OralTradition,
        IdeaId::GiftEconomy,
        IdeaId::Storycraft,
        IdeaId::SunSalutations,
        IdeaId::Excavation,
        IdeaId::Cogwheels,
        IdeaId::TreatAlchemy,
        IdeaId::TeaCeremonies,
        IdeaId::SolarDevotion,
        IdeaId::SkyCharts,
        IdeaId::PorchLogistics,
        IdeaId::AnthologyBinding,
        IdeaId::DewHarvest,
        IdeaId::Toymaking,
        IdeaId::CeramicArts,
        IdeaId::DeepCuriosity,
        IdeaId::MoonlightArts,
        IdeaId::LabyrinthTheory,
        IdeaId::GrandSchemes,
        IdeaId::Dynamos,
        IdeaId::SolarBasking,
        IdeaId::Millworks,
        IdeaId::Moonmetallurgy,
        IdeaId::Starcatching,
        IdeaId::PrismCraft,
        IdeaId::BalloonCraft,
        IdeaId::YarnComputing,
        IdeaId::ParadeRoutes,
        IdeaId::PenPalNetworks,
        IdeaId::ScoutPicnicking,
        IdeaId::DreamTending,
        IdeaId::NightCourtEtiquette,
        IdeaId::HushfeltSeeking,
        IdeaId::MemoryWeaving,
        IdeaId::SkywardLonging,
        IdeaId::Dreamweaving,
        IdeaId::AdventurousSpirit,
        IdeaId::DreamLoomcraft,
        IdeaId::DeepNapTheory,
        IdeaId::PouncingPractice,
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

/// One id space across every kind of revealable content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ContentId {
    Structure(StructureId),
    Idea(IdeaId),
}

/// Advisory display state; command gating is unaffected.
// @spec SIM-ENG-023
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    Hidden,
    Teased,
    Available,
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

/// Serialize an enum-indexed array as a plain JSON sequence (serde's built-in
/// array impls stop at 32 elements).
fn serialize_array<S: serde::Serializer, T: Serialize, const N: usize>(
    v: &[T; N],
    s: S,
) -> Result<S::Ok, S::Error> {
    v[..].serialize(s)
}

/// Deserialize an enum-indexed array, zero-filling the tail: older saves
/// carry shorter arrays because ids only ever append. Longer-than-known
/// arrays are corrupt (a newer save is refused by the version check first).
// @spec SAVE-013
fn zero_filled_array<'de, D: Deserializer<'de>, T, const N: usize>(
    d: D,
) -> Result<[T; N], D::Error>
where
    T: Deserialize<'de> + Copy + Default,
{
    let v: Vec<T> = Vec::deserialize(d)?;
    if v.len() > N {
        return Err(serde::de::Error::custom(format!(
            "array carries {} entries but this build knows only {N}",
            v.len()
        )));
    }
    let mut arr = [T::default(); N];
    arr[..v.len()].copy_from_slice(&v);
    Ok(arr)
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
    /// Indexed by `ResourceId::ix()`. Older saves zero-fill forward.
    #[serde(serialize_with = "serialize_array", deserialize_with = "zero_filled_array")]
    pub resources: [f64; RESOURCE_COUNT],
    /// Indexed by `StructureId::ix()`. Older saves zero-fill forward.
    #[serde(serialize_with = "serialize_array", deserialize_with = "zero_filled_array")]
    pub structures: [u32; STRUCTURE_COUNT],
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
    /// Game-speed multiplier, [0.25, 8]. Read only at the boundary (shell
    /// cadence, offline catch-up) — never inside tick(). @spec SIM-ENG-024
    #[serde(default = "default_speed")]
    pub speed: f64,
    /// Permanently revealed content ids. @spec SIM-ENG-023
    #[serde(default)]
    pub revealed: BTreeSet<ContentId>,
    /// The most kites ever held — a persisted ratchet the Porch's cap
    /// multiplier reads, so spending kites never shrinks storage.
    /// @spec SIM-ECO-031
    #[serde(default)]
    pub kites_high_water: f64,
    /// Accumulated clutter from industry; a slowdown, never a loss.
    /// @spec SIM-ECO-037
    #[serde(default)]
    pub mess: f64,
    /// A visible sky event's observation window: Some(expiry tick) while one
    /// waits to be observed. At most one at a time. Serialized without the
    /// word "event" — the save carries canonical state, not occurrences.
    /// @spec SIM-ENG-025
    #[serde(default, rename = "pending_sky")]
    pub pending_sky_event: Option<u64>,
}

fn default_speed() -> f64 {
    1.0
}

pub(crate) const SPEED_MIN: f64 = 0.25;
pub(crate) const SPEED_MAX: f64 = 8.0;

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
            resources: [0.0; RESOURCE_COUNT],
            structures: [0; STRUCTURE_COUNT],
            kittens: Vec::new(),
            ideas: BTreeSet::new(),
            pity: 0,
            famine_days: 0,
            recovery_days: 0,
            grumpy: false,
            next_seq: 0,
            zoomies_until: 0,
            speed: 1.0,
            revealed: BTreeSet::new(),
            kites_high_water: 0.0,
            mess: 0.0,
            pending_sky_event: None,
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

    /// Storage cap for a resource: base plus every cap-extension effect.
    // @spec SIM-ECO-001, SIM-ECO-002
    pub fn cap(&self, r: ResourceId) -> f64 {
        content::BASE_CAPS[r.ix()] + self.effect(crate::effects::Effect::MaxCap(r))
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

    /// Ideas open once curiosity storage exists (the box is the gate) and
    /// every prerequisite is researched (multi-prereq ideas need all).
    // @spec SIM-CUR-001, SIM-CUR-002, SIM-CUR-008
    pub fn idea_unlocked(&self, i: IdeaId) -> bool {
        if self.cap(ResourceId::Curiosity) <= 0.0 {
            return false;
        }
        self.idea_prereqs_met(i)
    }

    /// The prerequisite half of `idea_unlocked`, for callers that have
    /// already checked the curiosity gate once (the per-tick reveal scan).
    pub(crate) fn idea_prereqs_met(&self, i: IdeaId) -> bool {
        content::idea_def(i).prereqs.iter().all(|p| self.ideas.contains(p))
    }

    /// A recipe rides the idea tree: Locked until its granting idea.
    // @spec SIM-ECO-033
    pub fn craft_unlocked(&self, c: CraftId) -> bool {
        match content::craft_def(c).gate {
            None => true,
            Some(idea) => self.ideas.contains(&idea),
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
        let mut h = 100.0 - content::CROWDING_POINTS * crowding
            + (content::LOAFER_POINTS * loafers).min(content::LOAFER_POINTS_MAX)
            + self.effect(crate::effects::Effect::HappinessPoints);
        // Deep clutter dampens the vibe: −5 points per mess level above 1.
        // @spec SIM-ECO-038
        let mess_level = self.mess_level();
        if mess_level >= 2 {
            h -= content::MESS_HAPPINESS_POINTS * (mess_level - 1) as f64;
        }
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
        // Serious clutter makes newcomers shy (level 1+). @spec SIM-ECO-038
        if self.mess_level() >= 1 {
            chance *= content::MESS_ARRIVAL_FACTOR;
        }
        chance
    }

    pub fn zoomies_active(&self) -> bool {
        self.ticks < self.zoomies_until
    }

    // ---- Derived energy queries (NRG facet) ----

    /// The Dynamo Wheel's fuel satisfaction (0..=1) over all active wheels.
    pub(crate) fn dynamo_fuel_fraction(&self) -> f64 {
        let wheels = self.structures[StructureId::DynamoWheel.ix()];
        if wheels == 0 {
            return 0.0;
        }
        let inputs = content::structure_def(StructureId::DynamoWheel)
            .converter
            .expect("the Dynamo Wheel draws fuel")
            .inputs;
        let demands: Vec<(ResourceId, f64)> =
            inputs.iter().map(|(r, d)| (*r, d * wheels as f64)).collect();
        self.throttle_fraction(&demands)
    }

    /// Energy produced by active structures: linear rates from the BLD table,
    /// the Sunning Deck breathing with the season, the Dynamo Wheel scaled by
    /// its fuel fraction.
    // @spec SIM-ECO-035
    pub fn energy_production(&self) -> f64 {
        let mut sum = 0.0;
        for s in StructureId::ALL {
            let count = self.structures[s.ix()] as f64;
            if count == 0.0 {
                continue;
            }
            let base = content::structure_def(s).energy_prod;
            sum += match s {
                StructureId::SunningDeck => {
                    base * count * content::sunning_deck_season_factor(self.season())
                }
                StructureId::DynamoWheel => base * count * self.dynamo_fuel_fraction(),
                _ => base * count,
            };
        }
        sum
    }

    /// Energy consumed by active structures; Yarn Computers draw
    /// 2n + n²/2 in total at n copies.
    // @spec SIM-ECO-035
    pub fn energy_consumption(&self) -> f64 {
        let mut sum = 0.0;
        for s in StructureId::ALL {
            let count = self.structures[s.ix()] as f64;
            if count == 0.0 {
                continue;
            }
            sum += match s {
                StructureId::YarnComputer => 2.0 * count + count * count / 2.0,
                _ => content::structure_def(s).energy_cons * count,
            };
        }
        sum
    }

    /// Brownout multiplier for production-stack layers 6–8:
    /// clamp(production/consumption, 0.25, 1.0); 1.0 on a quiet grid.
    // @spec SIM-ECO-036
    pub fn energy_deficit_ratio(&self) -> f64 {
        let consumption = self.energy_consumption();
        if consumption <= EPSILON {
            return 1.0;
        }
        (self.energy_production() / consumption).clamp(0.25, 1.0)
    }

    /// Surplus multiplier for effects that name it (the Moonlight Stage):
    /// clamp(production/max(consumption, 1), 0, 1.75).
    // @spec SIM-ECO-036
    pub fn energy_surplus_ratio(&self) -> f64 {
        (self.energy_production() / self.energy_consumption().max(1.0)).clamp(0.0, 1.75)
    }

    // ---- Derived mess queries (MESS facet) ----

    /// Mess level: floor(log10(mess/1e4)) at or above 1e4, else −1 (none).
    // @spec SIM-ECO-037
    pub fn mess_level(&self) -> i32 {
        if self.mess >= content::MESS_LEVEL_UNIT {
            (self.mess / content::MESS_LEVEL_UNIT).log10().floor() as i32
        } else {
            -1
        }
    }

    /// Catnip-growth multiplier from mess: ×0.8 per level step at level 0+,
    /// floored at ×0.25 — a slowdown, never a loss.
    // @spec SIM-ECO-038
    pub(crate) fn mess_catnip_factor(&self) -> f64 {
        let level = self.mess_level();
        if level < 0 {
            1.0
        } else {
            content::MESS_CATNIP_STEP
                .powi(level + 1)
                .max(content::MESS_CATNIP_FLOOR)
        }
    }

    /// Advisory display state for a content item. Reveals are sticky
    /// (scanned at the end of each tick's clamp phase).
    // @spec SIM-ENG-023
    pub fn availability(&self, id: ContentId) -> Availability {
        if !self.revealed.contains(&id) {
            return Availability::Hidden;
        }
        let actionable = match id {
            ContentId::Structure(s) => {
                self.structure_unlocked(s)
                    && self
                        .next_cost(s)
                        .iter()
                        .all(|(r, amount)| self.stock(*r) + EPSILON >= *amount)
            }
            ContentId::Idea(i) => {
                let def = content::idea_def(i);
                self.idea_unlocked(i)
                    && !self.ideas.contains(&i)
                    && self.stock(ResourceId::Curiosity) + EPSILON >= def.cost
                    && def
                        .gate
                        .iter()
                        .all(|(r, amount)| self.stock(*r) + EPSILON >= *amount)
            }
        };
        if actionable {
            Availability::Available
        } else {
            Availability::Teased
        }
    }

    /// Would this item reveal right now? (Sticky insertion happens in tick.)
    pub(crate) fn reveal_condition_met(&self, id: ContentId) -> bool {
        match id {
            ContentId::Structure(s) => {
                // Allocation-free next_cost: this runs in the per-tick scan.
                let def = content::structure_def(s);
                let owned = self.structures[s.ix()] as i32;
                self.structure_unlocked(s)
                    && def.base_costs.iter().all(|(r, base)| {
                        let amount = (base * def.ratio.powi(owned)).ceil();
                        self.stock(*r) + EPSILON >= def.reveal_fraction * amount
                    })
            }
            ContentId::Idea(i) => self.idea_unlocked(i),
        }
    }
}

pub(crate) fn season_index_of_day(day: u64) -> usize {
    ((day / content::DAYS_PER_SEASON) % 4) as usize
}

pub(crate) fn season_of_day(day: u64) -> Season {
    Season::ALL[season_index_of_day(day)]
}
