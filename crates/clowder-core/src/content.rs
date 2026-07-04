//! Typed content tables: every number, name, and flavor line in the game.
//! Content lives in Rust (HLD decision) — the compiler checks refactors and
//! there is no loader to maintain.

use crate::state::{IdeaId, JobId, ResourceId, Season, StructureId};

// ---- Time ----
pub const TICKS_PER_SECOND: u64 = 4;
pub const TICKS_PER_DAY: u64 = 10;
pub const DAYS_PER_SEASON: u64 = 25;
pub const DAYS_PER_YEAR: u64 = 100;

// ---- Resources ----
// @spec SIM-ECO-001
pub const BASE_CAPS: [f64; 4] = [200.0, 50.0, 25.0, 0.0];

// ---- Manual actions ----
pub const GATHER_YIELD: f64 = 1.0;
pub const FORAGE_COST: f64 = 10.0;
pub const FORAGE_YIELD: f64 = 1.0;

// ---- Kittens ----
pub const KITTEN_APPETITE: f64 = 0.085; // catnip per tick, per Home kitten
pub const ARRIVAL_MIN_CATNIP: f64 = 50.0;
pub const ARRIVAL_BASE_CHANCE: f64 = 0.15;
pub const ARRIVAL_PER_100_CATNIP: f64 = 0.05;
pub const ARRIVAL_CHANCE_CAP: f64 = 0.60;
pub const ARRIVAL_WATCH_BONUS: f64 = 0.15; // on top of the cap
pub const PITY_THRESHOLD: u8 = 4;
pub const FAMINE_WANDER_DAYS: u8 = 3;
pub const CROWDING_FREE: u32 = 5; // kittens beyond this each cost happiness
pub const CROWDING_POINTS: f64 = 3.0;
pub const POST_POINTS: f64 = 5.0;
pub const LOAFER_POINTS: f64 = 1.0;
pub const LOAFER_POINTS_MAX: f64 = 5.0;
pub const BISCUIT_POINTS: f64 = 10.0;
pub const HAPPINESS_FLOOR: f64 = 25.0;
pub const GRUMPY_MULTIPLIER: f64 = 0.25;

// ---- Zoomies & flavor ----
pub const ZOOMIES_DAILY_CHANCE: f64 = 0.04;
pub const ZOOMIES_MULT: f64 = 1.25;
pub const ZOOMIES_TICKS: u64 = 120;
pub const ZOOMIES_CHANNELED_MULT: f64 = 1.5;
pub const ZOOMIES_CHANNELED_TICKS: u64 = 240;
pub const FLAVOR_DAILY_CHANCE: f64 = 0.08;

// ---- Structures ----

pub struct StructureDef {
    pub name: &'static str,
    pub base_costs: &'static [(ResourceId, f64)],
    pub ratio: f64,
    /// (resource, amount per tick per copy); the patch alone is seasonal.
    pub production: Option<(ResourceId, f64)>,
    pub seasonal: bool,
    pub housing: u32,
    pub cap_bonus: Option<(ResourceId, f64)>,
    pub happiness_points: f64,
    pub gate: Option<IdeaId>,
}

// @spec SIM-ECO-008, SIM-ECO-009, SIM-ECO-010, SIM-ECO-011, SIM-ECO-012, SIM-ECO-013, SIM-ECO-014
pub fn structure_def(s: StructureId) -> &'static StructureDef {
    &STRUCTURES[s.ix()]
}

static STRUCTURES: [StructureDef; 7] = [
    StructureDef {
        name: "Catnip Patch",
        base_costs: &[(ResourceId::Catnip, 10.0)],
        ratio: 1.12,
        production: Some((ResourceId::Catnip, 0.125)),
        seasonal: true,
        housing: 0,
        cap_bonus: None,
        happiness_points: 0.0,
        gate: None,
    },
    StructureDef {
        name: "Cozy Cushion",
        base_costs: &[(ResourceId::Catnip, 25.0), (ResourceId::Twigs, 5.0)],
        ratio: 1.15,
        production: None,
        seasonal: false,
        housing: 1,
        cap_bonus: None,
        happiness_points: 0.0,
        gate: None,
    },
    StructureDef {
        name: "Cardboard Box",
        base_costs: &[(ResourceId::Twigs, 8.0)],
        ratio: 1.15,
        production: Some((ResourceId::Curiosity, 0.005)),
        seasonal: false,
        housing: 0,
        cap_bonus: Some((ResourceId::Curiosity, 50.0)),
        happiness_points: 0.0,
        gate: None,
    },
    StructureDef {
        name: "Pantry",
        base_costs: &[(ResourceId::Twigs, 20.0)],
        ratio: 1.15,
        production: None,
        seasonal: false,
        housing: 0,
        cap_bonus: Some((ResourceId::Catnip, 250.0)),
        happiness_points: 0.0,
        gate: Some(IdeaId::PantryOrganization),
    },
    StructureDef {
        name: "Twig Pile",
        base_costs: &[(ResourceId::Catnip, 50.0), (ResourceId::Twigs, 10.0)],
        ratio: 1.15,
        production: None,
        seasonal: false,
        housing: 0,
        cap_bonus: Some((ResourceId::Twigs, 40.0)),
        happiness_points: 0.0,
        gate: Some(IdeaId::TwigArchitecture),
    },
    StructureDef {
        name: "Scratching Post",
        base_costs: &[(ResourceId::Twigs, 15.0), (ResourceId::Fluff, 5.0)],
        ratio: 1.15,
        production: None,
        seasonal: false,
        housing: 0,
        cap_bonus: None,
        happiness_points: 5.0,
        gate: Some(IdeaId::ScratchingArts),
    },
    StructureDef {
        name: "Cat Tree",
        base_costs: &[(ResourceId::Twigs, 30.0), (ResourceId::Fluff, 10.0)],
        ratio: 1.15,
        production: None,
        seasonal: false,
        housing: 3,
        cap_bonus: Some((ResourceId::Fluff, 10.0)),
        happiness_points: 0.0,
        gate: Some(IdeaId::TwigArchitecture),
    },
];

// ---- Jobs ----

pub enum JobGate {
    Always,
    Idea(IdeaId),
    RequiresBox,
}

pub struct JobDef {
    pub name: &'static str,
    pub yield_per_tick: (ResourceId, f64),
    pub gate: JobGate,
}

// @spec SIM-ECO-015, SIM-ECO-016, SIM-ECO-017, SIM-ECO-018
pub fn job_def(j: JobId) -> &'static JobDef {
    &JOBS[j as usize]
}

static JOBS: [JobDef; 4] = [
    JobDef {
        name: "Gardener",
        yield_per_tick: (ResourceId::Catnip, 0.15),
        gate: JobGate::Always,
    },
    JobDef {
        name: "Twig Collector",
        yield_per_tick: (ResourceId::Twigs, 0.03),
        gate: JobGate::Always,
    },
    JobDef {
        name: "Groomer",
        yield_per_tick: (ResourceId::Fluff, 0.02),
        gate: JobGate::Idea(IdeaId::GroomedToPurrfection),
    },
    JobDef {
        name: "Observer",
        yield_per_tick: (ResourceId::Curiosity, 0.025),
        gate: JobGate::RequiresBox,
    },
];

// ---- Seasons ----

// @spec SIM-ECO-019
pub fn season_patch_modifier(season: Season, winter_eased: bool) -> f64 {
    match season {
        Season::Spring => 1.5,
        Season::Summer => 1.0,
        Season::Autumn => 0.85,
        Season::Winter => {
            if winter_eased {
                0.40
            } else {
                0.25
            }
        }
    }
}

// ---- Ideas ----

pub struct IdeaDef {
    pub name: &'static str,
    pub blurb: &'static str,
    pub cost: f64,
    pub prereq: Option<IdeaId>,
}

// @spec SIM-CUR-005
pub fn idea_def(i: IdeaId) -> &'static IdeaDef {
    &IDEAS[i as usize]
}

static IDEAS: [IdeaDef; 9] = [
    IdeaDef {
        name: "Pantry Organization",
        blurb: "Stack the catnip. Label nothing. Unlocks the Pantry.",
        cost: 15.0,
        prereq: None,
    },
    IdeaDef {
        name: "The Scratching Arts",
        blurb: "An ancient discipline. Unlocks the Scratching Post.",
        cost: 25.0,
        prereq: None,
    },
    IdeaDef {
        name: "Groomed to Purrfection",
        blurb: "Mutual grooming, industrialized. Unlocks the Groomer job.",
        cost: 40.0,
        prereq: Some(IdeaId::PantryOrganization),
    },
    IdeaDef {
        name: "Twig Architecture",
        blurb: "Load-bearing twigs. Unlocks the Cat Tree and Twig Pile.",
        cost: 60.0,
        prereq: Some(IdeaId::ScratchingArts),
    },
    IdeaDef {
        name: "Selective Hearing",
        blurb: "You call, they consider. Gathering yields double.",
        cost: 80.0,
        prereq: Some(IdeaId::PantryOrganization),
    },
    IdeaDef {
        name: "Biscuit Making",
        blurb: "Rhythmic paw-kneading raises everyone's spirits. +10% happiness.",
        cost: 120.0,
        prereq: Some(IdeaId::GroomedToPurrfection),
    },
    IdeaDef {
        name: "Seasonal Awareness",
        blurb: "Thick coats and forward planning. Softer winters, visible forecast.",
        cost: 150.0,
        prereq: Some(IdeaId::TwigArchitecture),
    },
    IdeaDef {
        name: "Zoomie Channeling",
        blurb: "Harness the 3 a.m. energy. Zoomies run stronger and longer.",
        cost: 200.0,
        prereq: Some(IdeaId::BiscuitMaking),
    },
    IdeaDef {
        name: "Neighborhood Watch",
        blurb: "Word gets around about this place. Kittens arrive more often.",
        cost: 250.0,
        prereq: Some(IdeaId::SeasonalAwareness),
    },
];

// ---- Names ----

pub static NAME_POOL: [&str; 40] = [
    "Biscuit", "Mochi", "Pixel", "Waffles", "Miso", "Clementine", "Tofu", "Pepper", "Marmalade",
    "Noodle", "Pretzel", "Juniper", "Bean", "Dumpling", "Ziggy", "Olive", "Paprika", "Maple",
    "Sprocket", "Turnip", "Gnocchi", "Butterscotch", "Fig", "Pancake", "Nimbus", "Tangerine",
    "Pickles", "Comet", "Bramble", "Cricket", "Fennel", "Sesame", "Plum", "Wasabi", "Doodle",
    "Crumpet", "Nutmeg", "Socks", "Clover", "Captain Whiskers",
];

// ---- Flavor templates ----

pub struct FlavorTemplate {
    pub text: &'static str,
    pub needs_kitten: bool,
}

pub static FLAVOR_TEMPLATES: [FlavorTemplate; 10] = [
    FlavorTemplate { text: "{name} knocked a pencil off the table. It was necessary.", needs_kitten: true },
    FlavorTemplate { text: "{name} is sitting in the fresh laundry. The laundry is warm now.", needs_kitten: true },
    FlavorTemplate { text: "{name} stared at the wall for a while. It knows something.", needs_kitten: true },
    FlavorTemplate { text: "{name} found the exact center of the sunniest spot and claimed it.", needs_kitten: true },
    FlavorTemplate { text: "{name} chirped at a bird through the window. Negotiations continue.", needs_kitten: true },
    FlavorTemplate { text: "{name} performed the ceremonial pre-nap stretch. Flawless form.", needs_kitten: true },
    FlavorTemplate { text: "A sunbeam moved; the whole clowder relocated accordingly.", needs_kitten: false },
    FlavorTemplate { text: "Somewhere, a cardboard box settled with a dignified whumpf.", needs_kitten: false },
    FlavorTemplate { text: "A moth got in. The meadow has never been so alert.", needs_kitten: false },
    FlavorTemplate { text: "It rained gently. The porch smelled like spring for an hour.", needs_kitten: false },
];
