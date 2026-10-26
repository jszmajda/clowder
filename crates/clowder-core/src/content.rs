//! Typed content tables: every number, name, and flavor line in the game.
//! Content lives in Rust (HLD decision) — the compiler checks refactors and
//! there is no loader to maintain.

use crate::effects::Effect;
use crate::state::{
    AchievementId, AdventureId, CraftId, DreamLoomId, EyeColour, EyeLook, FavouriteKind, FurColour,
    FurPattern, IdeaId, JobId, KittenTrait, RefinementId, ResourceId, RuleId, Season, SiteId,
    StructureGate, StructureId, TraditionId,
};
use crate::state::{IDEA_COUNT, REFINEMENT_COUNT, RESOURCE_COUNT, STRUCTURE_COUNT};

// ---- Time ----
pub const TICKS_PER_SECOND: u64 = 4;
pub const TICKS_PER_DAY: u64 = 10;
pub const DAYS_PER_SEASON: u64 = 25;
pub const DAYS_PER_YEAR: u64 = 100;

// ---- Resources ----

/// Currency tiers (sub-HLD § Currency Tiers). Raw materials take the full
/// production stack; every other tier skips the global layers 6–8.
// @spec SIM-ECO-025
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    Raw,
    Transient,
    Luxury,
    Mystic,
    Dreamtime,
    Crafted,
}

// @spec SIM-ECO-025
pub fn tier(r: ResourceId) -> Tier {
    use ResourceId::*;
    match r {
        Catnip | Twigs | Fluff | Pebbles | Buttons | Embers | Shinies | Dew | Moonstone
        | Stardust | Hushfelt => Tier::Raw,
        Curiosity | Pep | Charm | Warmth | Skymaps | Cleverness => Tier::Transient,
        Feathers | Bells | Mint => Tier::Luxury,
        Unicorns | Rainbows | Prisms | Starshards | Echoes | Dreamcorns | JarsOfPurrs => {
            Tier::Mystic
        }
        Reverie => Tier::Dreamtime,
        Wands | Tiles | Badges | Brass | Cogs | Moonsteel | Lattices | Pavers | Quills
        | Stories | Anthologies | Schemes | Cairns | Kites | Balloons | Moonmilk | Dreamweave
        | Cometsilk => Tier::Crafted,
    }
}

const INF: f64 = f64::INFINITY;

/// Base storage caps in RES-table order; ∞ = uncapped (clamp skips it, and
/// the value is derived only — never serialized).
// @spec SIM-ECO-001
pub const BASE_CAPS: [f64; RESOURCE_COUNT] = [
    200.0, // catnip
    50.0,  // twigs
    25.0,  // fluff
    0.0,   // curiosity — cap 0 until a Cardboard Box exists
    60.0,  // pebbles
    40.0,  // buttons
    30.0,  // embers
    15.0,  // shinies
    0.0,   // dew — cap 0 until Dew Collectors
    8.0,   // moonstone
    0.0,   // stardust — cap 0 until Star Basins
    0.0,   // hushfelt — cap extender owed by the cosmic leaf
    100.0, // pep
    80.0,  // charm
    0.0,   // warmth — cap 0 until Sun Rooms
    0.0,   // skymaps — cap 0 until Stargazing Ledges
    INF,   // feathers
    INF,   // bells
    INF,   // mint
    INF,   // unicorns
    INF,   // rainbows
    INF,   // prisms
    INF,   // starshards
    INF,   // echoes
    INF,   // dreamcorns
    0.0,   // jars of purrs — capped by mystic content (wave 4)
    INF,   // reverie
    INF, INF, INF, INF, INF, INF, INF, INF, INF, // wands..quills (crafted)
    INF, INF, INF, INF, INF, INF, INF, INF, INF, // stories..cometsilk (crafted)
    INF, // cleverness — the Yarn Computer's wit has no ceiling
];

/// Fraction of each cost component that must be held for a content item to
/// reveal (become visible as a silhouette). @spec SIM-ENG-023
pub const REVEAL_FRACTION: f64 = 0.30;

// ---- Manual actions ----
pub const GATHER_YIELD: f64 = 1.0;
pub const FORAGE_COST: f64 = 10.0;
pub const FORAGE_YIELD: f64 = 1.0;
/// Rummage: idea-gated pebble foraging. @spec SIM-ECO-027
pub const RUMMAGE_YIELD: f64 = 0.5;

// ---- Luxuries ----
// Per-Home-kitten drain rates, scaled by (1 + LuxuryDemandRatio effects).
// @spec SIM-ECO-026
pub const LUXURY_DRAINS: [(ResourceId, f64); 3] = [
    (ResourceId::Feathers, 0.001),
    (ResourceId::Bells, 0.0007),
    (ResourceId::Mint, 0.0001),
];

// ---- Mess ----
// @spec SIM-ECO-037, SIM-ECO-038
pub const MESS_DISSIPATION: f64 = 1e-4; // × current mess, per tick
pub const MESS_LEVEL_UNIT: f64 = 1e4; // level = floor(log10(mess / unit))
pub const MESS_CATNIP_STEP: f64 = 0.8; // per level step, level 0 up
pub const MESS_CATNIP_FLOOR: f64 = 0.25;
pub const MESS_ARRIVAL_FACTOR: f64 = 0.75; // level 1 up
pub const MESS_HAPPINESS_POINTS: f64 = 5.0; // per level above 1, level 2 up

// ---- Sky events ----
// @spec SIM-ENG-025
pub const SKY_EVENT_CHANCE_PER_LEDGE: f64 = 0.002; // per day close
pub const SKY_AUTO_OBSERVE_PER_LEDGE: f64 = 0.01;
pub const SKY_OBSERVE_CURIOSITY: f64 = 25.0; // × (1 + curiosity ratios)
pub const SKY_OBSERVE_SKYMAPS: f64 = 1.0;
/// How long a visible sky event stays observable: one game year from the
/// day close that raised it — minutes of real time, long enough for a glance.
pub const SKY_WINDOW_TICKS: u64 = DAYS_PER_YEAR * TICKS_PER_DAY;

// ---- Energy ----
// @spec SIM-ECO-035
pub const SUNNING_DECK_WINTER: f64 = 0.75;
pub const SUNNING_DECK_SUMMER: f64 = 1.25;

pub fn sunning_deck_season_factor(season: Season) -> f64 {
    match season {
        Season::Winter => SUNNING_DECK_WINTER,
        Season::Summer => SUNNING_DECK_SUMMER,
        _ => 1.0,
    }
}

/// Treat Bakery output rate: mpr = pep cap × 0.007 / 100 — storage feeding
/// production. @spec SIM-ECO-030
pub const BAKERY_MPR_PER_PEP_CAP: f64 = 0.007 / 100.0;
pub const BAKERY_FEATHERS_MPR: f64 = 1.25;
pub const BAKERY_BELLS_MPR: f64 = 0.3;

// ---- Global production layer (stack layer 6) ----
// @spec SIM-ECO-032
pub const DYNAMO_GLOBAL_RATIO: f64 = 0.02; // per active wheel, × fuel fraction
pub const COGWORKS_DYNAMO_BOOST: f64 = 0.15; // × owned Cogworks, on the dynamo term
pub const STAR_BASIN_GLOBAL_RATIO: f64 = 0.05; // per active basin

// ---- Porch kite ratchet ----
// Porch caps × (1 + limited_dr(0.01 × kite high-water, limit)).
// @spec SIM-ECO-031
pub const PORCH_KITE_PER: f64 = 0.01;
pub const PORCH_KITE_DR_LIMIT: f64 = 2.25;

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

// ---- Skills, ranks, the Elder ----
// @spec SIM-KIT-019, SIM-KIT-020, SIM-KIT-021, SIM-KIT-022
pub const SKILL_XP_PER_TICK: f64 = 0.01; // × (1 + skill-XP effects)
pub const SKILL_DECAY_PER_TICK: f64 = 0.001; // every other job's skill, floor 0
pub const PROMOTE_EXP_BASE: f64 = 500.0; // × 1.75^rank
pub const PROMOTE_EXP_RATIO: f64 = 1.75;
pub const PROMOTE_SHINIES_STEP: f64 = 25.0; // × (rank + 1)
pub const ELDER_RANK_DIVISOR: f64 = 1.4; // own-job × max(1, (rank+1)/1.4)

/// Skill tiers multiply the kitten's own-job yield. @spec SIM-KIT-020
/// The skill-tier bonus with the Nap Time reward applied: only the master tier
/// (≥ 9000 XP) scales by the master-skill multiplier; the lower tiers are
/// untouched. `master_mult` is 1.0 (a no-op) until Nap Time is completed.
/// @spec SIM-KIT-020, SIM-ADV-005
pub fn skill_tier_bonus_scaled(xp: f64, master_mult: f64) -> f64 {
    let base = skill_tier_bonus(xp);
    if xp >= 9000.0 {
        base * master_mult
    } else {
        base
    }
}

pub fn skill_tier_bonus(xp: f64) -> f64 {
    if xp >= 9000.0 {
        0.1875
    } else if xp >= 5000.0 {
        0.125
    } else if xp >= 2500.0 {
        0.075
    } else if xp >= 1200.0 {
        0.045
    } else if xp >= 500.0 {
        0.025
    } else if xp >= 100.0 {
        0.0125
    } else {
        0.0
    }
}

// ---- Jamborees ----
// @spec SIM-KIT-025
pub const JAMBOREE_PEP: f64 = 800.0;
pub const JAMBOREE_CHARM: f64 = 1500.0;
pub const JAMBOREE_QUILLS: f64 = 40.0;
pub const JAMBOREE_TICKS: u64 = 1000; // 100 days — one game year
pub const JAMBOREE_HAPPINESS: f64 = 30.0; // × (1 + jamboree-ratio effects)
pub const JAMBOREE_ARRIVAL_BASE: f64 = 2.0; // + jamboree-arrival effects
pub const JAMBOREE_LUXURY_DEMAND: f64 = 0.2; // celebrations consume

// ---- Pouncing expeditions ----
// @spec SIM-EXP-001, SIM-EXP-002, SIM-EXP-003
pub const POUNCE_PEP_PER_SQUAD: f64 = 100.0;
pub const POUNCE_FEATHERS_BASE: f64 = 8.0;
pub const POUNCE_FEATHERS_PER_RATIO: f64 = 6.5;
pub const POUNCE_BELLS_CHANCE: f64 = 0.45; // + 0.02 × pounce_ratio
pub const POUNCE_BELLS_CHANCE_PER_RATIO: f64 = 0.02;
pub const POUNCE_BELLS_BASE: f64 = 5.0;
pub const POUNCE_BELLS_PER_RATIO: f64 = 4.0;
pub const POUNCE_SHINIES_CHANCE: f64 = 0.40;
pub const POUNCE_SHINIES_BASE: f64 = 1.0;
pub const POUNCE_SHINIES_PER_RATIO: f64 = 1.0;
pub const POUNCE_UNICORN_CHANCE: f64 = 0.05;

// ---- Tinkerer background crafting ----
// @spec SIM-ECO-043, SIM-KIT-026
pub const TINKER_BATCHES_PER_TICK: f64 = 0.0017; // × (1 + speed) × effectiveness
pub const TINKER_TIER_PENALTY: f64 = 0.15; // per tier above rank + 1
pub const TINKER_EFFECTIVENESS_FLOOR: f64 = 0.25;

// ---- The Cogworks year-close auto-craft ----
// @spec SIM-ECO-039
pub const COGWORKS_CONVERT_PER: f64 = 0.02; // of stock, per Cogworks
pub const COGWORKS_CONVERT_MAX: f64 = 0.5; // never more than half the pile
pub const COGWORKS_THRESHOLD: f64 = 0.98; // of cap; below it, skipped

/// The single-input raw → craft pairs the Cogworks converts, in fixed order.
pub const COGWORKS_AUTOCRAFT: [(ResourceId, CraftId); 5] = [
    (ResourceId::Twigs, CraftId::Wands),
    (ResourceId::Pebbles, CraftId::Tiles),
    (ResourceId::Buttons, CraftId::Badges),
    (ResourceId::Dew, CraftId::Moonmilk),
    (ResourceId::Stardust, CraftId::Cometsilk),
];

/// The (structure, resource) pairs any StructureOutputAdd effect may target;
/// the production phase consults this list so refinement-granted outputs
/// land without scanning every key combination.
pub const STRUCTURE_OUTPUT_ADD_TARGETS: [(StructureId, ResourceId); 3] = [
    (StructureId::Hearth, ResourceId::Embers),
    (StructureId::Cogworks, ResourceId::Stories),
    // Stained Sunlight / Grand Basking rites add charm at the Sunbeam Altar.
    // @spec SIM-SUN-005
    (StructureId::SunbeamAltar, ResourceId::Charm),
];

// ---- Wave 4: ceremonies / mystic / colonies ----
// @spec SIM-UNI-006
pub const RAINBOW_ARCH_PRISM_DESCENT: f64 = 0.0002; // per day per Rainbow Arch structure
                                                    // @spec SIM-UNI-001
pub const PARADE_UNICORN_COST: f64 = 2500.0;
pub const PARADE_RAINBOWS_PER_ARCH: f64 = 2500.0;
// @spec SIM-UNI-003
pub const REFINE_INPUT: f64 = 25.0; // 25 prisms → 1 starshard, 25 starshards → 1 echo
                                    // @spec SIM-UNI-004
pub const JAR_RAINBOW_COST: f64 = 10000.0;
// @spec SIM-UNI-007
pub const DREAM_DRIFT_PER_ARCH: f64 = 0.000001; // per tick per Memory Arch
pub const DREAM_DRIFT_THROTTLE: f64 = 0.25; // while any dreamcorn already dreams
pub const SKY_ARCH_PRISM_DRIFT: f64 = 0.00002; // per tick, the Sky Arch's steady drift
                                               // @spec SIM-UNI-005
pub const RAINBOW_SHOWER_AMOUNT: f64 = 5000.0;
/// Prism Sight's brightening of the prism and rainbow chances. @spec SIM-MOVE-013
pub const PRISM_SIGHT_RATIO: f64 = 1.1; // a windfall on a shower day
                                        // @spec SIM-UNI-009, SIM-UNI-010
pub const PROMISE_ECHO_COST: f64 = 1.0;
pub const PROMISE_UPKEEP_DREAMCORN: f64 = 0.0005; // per day, per kept Promise
pub const PROMISE_GLOBAL_RATIO: f64 = 0.0005; // each kept Promise's global effect
pub const PROMISE_SLOTS_PER_EVERDREAM: usize = 3;
// @spec SIM-TRADE-001
pub const TRADE_PEP_PER_CARAVAN: f64 = 50.0;
pub const TRADE_SHINIES_PER_CARAVAN: f64 = 15.0;
pub const TRADE_FUZZ_WIDTH: f64 = 0.15; // ± width on the basket
pub const TRADE_BONUS_MULT: f64 = 1.25; // a bonus (warm) outcome
                                        // @spec SIM-TRADE-005, SIM-TRADE-006
pub const PENPAL_STANDING_PER: f64 = 0.01; // standing warmed per Pen Pal level
pub const PENPAL_STANDING_CAP: f64 = 0.75; // soft cap on Pen Pal warming
pub const PENPAL_BASE_CHARM: f64 = 100.0; // × 1.15^level
pub const PENPAL_CHARM_RATIO: f64 = 1.15;
// @spec SIM-TRADE-004 (Foxes' kite-scaled moonstone)
pub const FOX_MOONSTONE_CHANCE_BASE: f64 = 0.15;
pub const FOX_MOONSTONE_CHANCE_PER_KITE: f64 = 0.0035;
pub const FOX_MOONSTONE_AMOUNT_BASE: f64 = 1.5;
pub const FOX_MOONSTONE_AMOUNT_PER_KITE: f64 = 0.03;
// @spec SIM-TRADE-007
pub const SCOUT_PEP_COST: f64 = 1000.0;
pub const SCOUT_PEP_REFUND: f64 = 950.0; // returned on a quiet day
pub const SCOUT_FIND_CHANCE: f64 = 0.5; // chance a picnic finds anyone
pub const SCOUT_FOX_CHARM: f64 = 1500.0;
pub const SCOUT_OWL_KITES: f64 = 100.0;
pub const SCOUT_OWL_CURIOSITY_CAP: f64 = 125000.0;
pub const DEER_STARDUST_MIN_PENPAL: u32 = 5;

// ---- Crowding relief & luxuries (kittens leaf consumes) ----
// @spec SIM-KIT-011
pub const CROWDING_RELIEF_DR_LIMIT: f64 = 0.9;
pub const LUXURY_HAPPINESS_POINTS: f64 = 10.0;

// ---- Zoomies & flavor ----
pub const ZOOMIES_DAILY_CHANCE: f64 = 0.04;
pub const ZOOMIES_MULT: f64 = 1.25;
pub const ZOOMIES_TICKS: u64 = 120;
pub const ZOOMIES_CHANNELED_MULT: f64 = 1.5;
pub const ZOOMIES_CHANNELED_TICKS: u64 = 240;
pub const FLAVOR_DAILY_CHANCE: f64 = 0.08;

// ---- Structures ----

/// A converter draws inputs and emits outputs each tick, per active copy,
/// throttled together by the worst-supplied input. @spec SIM-ECO-029
#[derive(Debug, Clone, Copy)]
pub struct Converter {
    pub inputs: &'static [(ResourceId, f64)],
    pub outputs: &'static [(ResourceId, f64)],
}

pub struct StructureDef {
    pub name: &'static str,
    pub base_costs: &'static [(ResourceId, f64)],
    pub ratio: f64,
    /// Fraction of each cost component held before an ungated structure
    /// reveals. A gated structure reveals with its gate, so this goes unread
    /// there.
    pub reveal_fraction: f64,
    /// (resource, amount per tick per copy). Structure production takes only
    /// the ☆ stack layers: base, per-source ratio, season.
    pub productions: &'static [(ResourceId, f64)],
    /// The patch alone is seasonal.
    pub seasonal: bool,
    /// The Moonlight Stage alone scales by the energy surplus ratio.
    pub surplus_scaled: bool,
    pub housing: u32,
    pub cap_bonuses: &'static [(ResourceId, f64)],
    pub happiness_points: f64,
    /// Pooled crowding relief per copy (kittens leaf consumes, LDR cap 0.9).
    pub crowding_relief: f64,
    /// Per-source production ratios: +X% on every flow of that resource.
    pub resource_ratios: &'static [(ResourceId, f64)],
    pub craft_ratio: f64,
    /// Luxury-demand delta per copy (Gift Stands ease the drain).
    pub luxury_demand: f64,
    /// Bonus on the jamboree happiness term per copy (Tea Houses).
    pub jamboree_ratio: f64,
    /// Bonus on the jamboree arrival factor per copy (Tea Houses).
    pub jamboree_arrival: f64,
    /// Layer-6 contribution per active copy (dynamo term is fuel-scaled and
    /// Cogworks-amplified in the effect sum).
    pub global_ratio: f64,
    pub converter: Option<Converter>,
    /// Mess per active copy per tick; converters scale by their throttle.
    pub mess: f64,
    pub energy_prod: f64,
    pub energy_cons: f64,
    pub gate: StructureGate,
    /// A cosmic outpost: its raw material output routes through the outdoor
    /// channel (the global layers at a reduced transfer). @spec SIM-SKY-004
    pub outdoor: bool,
    /// Per-copy outdoor-production ratio contribution (Ranger Station, Cairn
    /// Circle, Aurora Lantern, Far Beacon, Wishing Star). @spec SIM-SKY-004
    pub outdoor_ratio: f64,
    /// Per-copy Long Sun cap extension (Aurora Lantern). @spec SIM-SKY-005
    pub long_sun_cap: f64,
    /// Tinkerer seats per copy (Toy Workshop). @spec SIM-ECO-045
    pub tinkerer_seats: u32,
}

/// Baseline for the table: every field its "nothing special" value.
const BLD: StructureDef = StructureDef {
    name: "",
    base_costs: &[],
    ratio: 1.15,
    reveal_fraction: REVEAL_FRACTION,
    productions: &[],
    seasonal: false,
    surplus_scaled: false,
    housing: 0,
    cap_bonuses: &[],
    happiness_points: 0.0,
    crowding_relief: 0.0,
    resource_ratios: &[],
    craft_ratio: 0.0,
    luxury_demand: 0.0,
    jamboree_ratio: 0.0,
    jamboree_arrival: 0.0,
    global_ratio: 0.0,
    converter: None,
    mess: 0.0,
    energy_prod: 0.0,
    energy_cons: 0.0,
    gate: StructureGate::Always,
    outdoor: false,
    outdoor_ratio: 0.0,
    long_sun_cap: 0.0,
    tinkerer_seats: 0,
};

// @spec SIM-ECO-008, SIM-ECO-009, SIM-ECO-010, SIM-ECO-011, SIM-ECO-012, SIM-ECO-013, SIM-ECO-014
pub fn structure_def(s: StructureId) -> &'static StructureDef {
    &STRUCTURES[s.ix()]
}

static STRUCTURES: [StructureDef; STRUCTURE_COUNT] = [
    StructureDef {
        name: "Catnip Patch",
        base_costs: &[(ResourceId::Catnip, 10.0)],
        ratio: 1.12,
        productions: &[(ResourceId::Catnip, 0.125)],
        seasonal: true,
        ..BLD
    },
    StructureDef {
        name: "Cozy Cushion",
        base_costs: &[(ResourceId::Catnip, 25.0), (ResourceId::Twigs, 5.0)],
        housing: 1,
        cap_bonuses: &[(ResourceId::Pep, 25.0)], // @spec SIM-ECO-044
        ..BLD
    },
    StructureDef {
        name: "Cardboard Box",
        base_costs: &[(ResourceId::Twigs, 8.0)],
        productions: &[(ResourceId::Curiosity, 0.005)],
        cap_bonuses: &[(ResourceId::Curiosity, 50.0)],
        ..BLD
    },
    StructureDef {
        name: "Pantry",
        base_costs: &[(ResourceId::Twigs, 20.0)],
        cap_bonuses: &[(ResourceId::Catnip, 250.0)],
        gate: StructureGate::Idea(IdeaId::PantryOrganization),
        ..BLD
    },
    StructureDef {
        name: "Twig Pile",
        base_costs: &[(ResourceId::Catnip, 50.0), (ResourceId::Twigs, 10.0)],
        cap_bonuses: &[(ResourceId::Twigs, 40.0)],
        gate: StructureGate::Idea(IdeaId::TwigArchitecture),
        ..BLD
    },
    StructureDef {
        name: "Scratching Post",
        base_costs: &[(ResourceId::Twigs, 15.0), (ResourceId::Fluff, 5.0)],
        happiness_points: 5.0,
        gate: StructureGate::Idea(IdeaId::ScratchingArts),
        ..BLD
    },
    StructureDef {
        name: "Cat Tree",
        base_costs: &[(ResourceId::Twigs, 30.0), (ResourceId::Fluff, 10.0)],
        housing: 3,
        // @spec SIM-ECO-044
        cap_bonuses: &[(ResourceId::Fluff, 10.0), (ResourceId::Pep, 75.0)],
        gate: StructureGate::Idea(IdeaId::TwigArchitecture),
        ..BLD
    },
    StructureDef {
        name: "Window Perch",
        base_costs: &[(ResourceId::Catnip, 40.0), (ResourceId::Twigs, 8.0)],
        housing: 1,
        cap_bonuses: &[(ResourceId::Pep, 25.0)], // @spec SIM-ECO-044
        productions: &[(ResourceId::Curiosity, 0.001)],
        gate: StructureGate::Idea(IdeaId::CozyCarpentry),
        ..BLD
    },
    StructureDef {
        name: "Herb Garden",
        base_costs: &[(ResourceId::Pebbles, 60.0)],
        ratio: 1.12,
        // A corner of every garden grows mint, a luxury every kitten uses.
        productions: &[(ResourceId::Mint, 0.00015)],
        resource_ratios: &[(ResourceId::Catnip, 0.03)],
        gate: StructureGate::Idea(IdeaId::Horticulture),
        ..BLD
    },
    StructureDef {
        name: "Couch Mine",
        base_costs: &[(ResourceId::Twigs, 90.0)],
        // What turns up under the couch: pebbles, and now and then something
        // that glints — the steady supply of shinies, pouncing the bonus.
        productions: &[(ResourceId::Pebbles, 0.01), (ResourceId::Shinies, 0.001)],
        resource_ratios: &[(ResourceId::Pebbles, 0.20)],
        gate: StructureGate::Idea(IdeaId::CouchSpelunking),
        ..BLD
    },
    StructureDef {
        name: "Gravel Pit",
        base_costs: &[(ResourceId::Tiles, 80.0), (ResourceId::Brass, 15.0)],
        productions: &[(ResourceId::Embers, 0.004)],
        resource_ratios: &[(ResourceId::Pebbles, 0.35)],
        mess: 0.25,
        gate: StructureGate::Idea(IdeaId::Excavation),
        ..BLD
    },
    StructureDef {
        name: "Hearth",
        base_costs: &[(ResourceId::Pebbles, 45.0), (ResourceId::Twigs, 25.0)],
        converter: Some(Converter {
            inputs: &[(ResourceId::Twigs, 0.05), (ResourceId::Pebbles, 0.10)],
            outputs: &[(ResourceId::Buttons, 0.02), (ResourceId::Embers, 0.001)],
        }),
        happiness_points: 2.0,
        mess: 0.1,
        gate: StructureGate::Idea(IdeaId::Emberkeeping),
        ..BLD
    },
    StructureDef {
        name: "Kiln",
        // No moonstone: the Kiln is where a meadow's own moonstone comes from.
        base_costs: &[
            (ResourceId::Tiles, 60.0),
            (ResourceId::Dew, 150.0),
            (ResourceId::Brass, 30.0),
        ],
        converter: Some(Converter {
            inputs: &[(ResourceId::Pebbles, 0.5), (ResourceId::Dew, 0.024)],
            outputs: &[(ResourceId::Buttons, 0.15), (ResourceId::Moonstone, 0.0005)],
        }),
        mess: 1.0,
        gate: StructureGate::Idea(IdeaId::CeramicArts),
        ..BLD
    },
    StructureDef {
        name: "Shed",
        base_costs: &[(ResourceId::Wands, 2.0), (ResourceId::Tiles, 3.0)],
        cap_bonuses: &[
            (ResourceId::Twigs, 150.0),
            (ResourceId::Pebbles, 200.0),
            (ResourceId::Buttons, 25.0),
            (ResourceId::Embers, 30.0),
            (ResourceId::Moonstone, 10.0),
            (ResourceId::Shinies, 5.0),
        ],
        gate: StructureGate::Idea(IdeaId::PackRattery),
        ..BLD
    },
    StructureDef {
        name: "Porch",
        base_costs: &[
            (ResourceId::Tiles, 40.0),
            (ResourceId::Badges, 60.0),
            (ResourceId::Lattices, 4.0),
        ],
        // All Porch caps ratchet on the kite high-water. @spec SIM-ECO-031
        cap_bonuses: &[
            (ResourceId::Catnip, 1000.0),
            (ResourceId::Twigs, 400.0),
            (ResourceId::Pebbles, 500.0),
            (ResourceId::Buttons, 80.0),
            (ResourceId::Embers, 50.0),
            (ResourceId::Moonstone, 25.0),
            (ResourceId::Shinies, 12.0),
        ],
        gate: StructureGate::Idea(IdeaId::PorchLogistics),
        ..BLD
    },
    StructureDef {
        name: "Study Nook",
        base_costs: &[
            (ResourceId::Twigs, 30.0),
            (ResourceId::Pebbles, 45.0),
            (ResourceId::Curiosity, 80.0),
        ],
        resource_ratios: &[(ResourceId::Curiosity, 0.20)],
        cap_bonuses: &[(ResourceId::Curiosity, 200.0)],
        gate: StructureGate::Idea(IdeaId::Notetaking),
        ..BLD
    },
    StructureDef {
        name: "Stargazing Ledge",
        base_costs: &[
            (ResourceId::Buttons, 250.0),
            (ResourceId::Curiosity, 800.0),
            (ResourceId::Tiles, 20.0),
            (ResourceId::Lattices, 40.0),
        ],
        ratio: 1.10,
        resource_ratios: &[(ResourceId::Curiosity, 0.25)],
        cap_bonuses: &[(ResourceId::Curiosity, 800.0), (ResourceId::Skymaps, 10.0)],
        gate: StructureGate::Idea(IdeaId::SkyCharts),
        ..BLD
    },
    StructureDef {
        name: "Whisker Lab",
        base_costs: &[
            (ResourceId::Curiosity, 1200.0),
            (ResourceId::Tiles, 60.0),
            (ResourceId::Brass, 40.0),
        ],
        ratio: 1.10,
        resource_ratios: &[(ResourceId::Curiosity, 0.35)],
        cap_bonuses: &[(ResourceId::Curiosity, 1200.0)],
        energy_cons: 1.0,
        gate: StructureGate::Idea(IdeaId::DeepCuriosity),
        ..BLD
    },
    StructureDef {
        name: "Craft Corner",
        base_costs: &[(ResourceId::Twigs, 80.0), (ResourceId::Pebbles, 300.0)],
        craft_ratio: 0.06,
        gate: StructureGate::Idea(IdeaId::Tinkering),
        ..BLD
    },
    StructureDef {
        name: "Toy Workshop",
        base_costs: &[
            (ResourceId::Buttons, 250.0),
            (ResourceId::Pavers, 12.0),
            (ResourceId::Moonstone, 18.0),
        ],
        craft_ratio: 0.05,
        energy_cons: 2.0,
        mess: 2.0, // the Tidy rota flips this — society wave
        tinkerer_seats: 1,
        gate: StructureGate::Idea(IdeaId::Toymaking),
        ..BLD
    },
    StructureDef {
        name: "Gift Stand",
        base_costs: &[
            (ResourceId::Twigs, 400.0),
            (ResourceId::Pebbles, 150.0),
            (ResourceId::Shinies, 8.0),
        ],
        // Trade ratio +1.5%/bld lands with the colonies leaf.
        luxury_demand: -0.04,
        gate: StructureGate::Idea(IdeaId::GiftEconomy),
        ..BLD
    },
    StructureDef {
        name: "Story Circle",
        base_costs: &[
            (ResourceId::Twigs, 160.0),
            (ResourceId::Pebbles, 900.0),
            (ResourceId::Quills, 3.0),
        ],
        crowding_relief: 0.048,
        productions: &[(ResourceId::Charm, 0.005)],
        cap_bonuses: &[(ResourceId::Charm, 40.0)],
        gate: StructureGate::Idea(IdeaId::OralTradition),
        ..BLD
    },
    StructureDef {
        name: "Sun Room",
        base_costs: &[
            (ResourceId::Pebbles, 1600.0),
            (ResourceId::Charm, 200.0),
            (ResourceId::Quills, 200.0),
        ],
        productions: &[(ResourceId::Charm, 0.04), (ResourceId::Warmth, 0.004)],
        cap_bonuses: &[(ResourceId::Warmth, 75.0), (ResourceId::Charm, 60.0)],
        gate: StructureGate::Idea(IdeaId::SunSalutations),
        ..BLD
    },
    StructureDef {
        name: "Sunbeam Altar",
        base_costs: &[
            (ResourceId::Shinies, 40.0),
            (ResourceId::Tiles, 20.0),
            (ResourceId::Badges, 12.0),
            (ResourceId::Stories, 8.0),
        ],
        productions: &[(ResourceId::Charm, 0.08)],
        // +100 shinies cap: offerings collect at the altar, and the upper rites
        // are shinies-priced, so this is what keeps them reachable (SIM-ECO-024).
        cap_bonuses: &[
            (ResourceId::Warmth, 120.0),
            (ResourceId::Charm, 100.0),
            (ResourceId::Shinies, 100.0),
        ],
        gate: StructureGate::Idea(IdeaId::SolarDevotion),
        ..BLD
    },
    StructureDef {
        name: "Treat Bakery",
        base_costs: &[
            (ResourceId::Pebbles, 4000.0),
            (ResourceId::Shinies, 150.0),
            (ResourceId::Badges, 150.0),
        ],
        // Outputs scale with the pep cap (mpr) — see the converter phase.
        converter: Some(Converter {
            inputs: &[(ResourceId::Shinies, 0.004), (ResourceId::Pep, 0.6)],
            outputs: &[], // computed: feathers 1.25·mpr, bells 0.3·mpr
        }),
        gate: StructureGate::Idea(IdeaId::TreatAlchemy),
        ..BLD
    },
    StructureDef {
        name: "Tea House",
        base_costs: &[
            (ResourceId::Twigs, 800.0),
            (ResourceId::Charm, 600.0),
            (ResourceId::Mint, 4.0),
            (ResourceId::Quills, 300.0),
        ],
        ratio: 1.5, // a deliberate wall, in the price-band vocabulary
        // @spec SIM-KIT-025
        jamboree_ratio: 0.01,
        jamboree_arrival: 0.001,
        gate: StructureGate::Idea(IdeaId::TeaCeremonies),
        ..BLD
    },
    StructureDef {
        name: "Dew Collector",
        base_costs: &[
            (ResourceId::Brass, 40.0),
            (ResourceId::Cogs, 20.0),
            (ResourceId::Lattices, 20.0),
        ],
        productions: &[(ResourceId::Dew, 0.02)],
        cap_bonuses: &[(ResourceId::Dew, 1200.0)],
        gate: StructureGate::Idea(IdeaId::DewHarvest),
        ..BLD
    },
    StructureDef {
        name: "Cogworks",
        base_costs: &[(ResourceId::Brass, 50.0), (ResourceId::Cogs, 15.0)],
        ratio: 1.25,
        // Amplifies dynamos (COGWORKS_DYNAMO_BOOST); converts capped raws at
        // year close (SIM-ECO-039).
        energy_prod: 3.0,
        mess: 0.5,
        gate: StructureGate::Idea(IdeaId::Cogwheels),
        ..BLD
    },
    StructureDef {
        name: "Dynamo Wheel",
        base_costs: &[
            (ResourceId::Cogs, 4.0),
            (ResourceId::Moonsteel, 8.0),
            (ResourceId::Schemes, 1.0),
        ],
        ratio: 1.25,
        converter: Some(Converter {
            inputs: &[(ResourceId::Dew, 0.05)],
            outputs: &[], // a pure fuel draw: the output is the global layer
        }),
        global_ratio: DYNAMO_GLOBAL_RATIO,
        energy_prod: 5.0, // × fuel fraction
        mess: 5.0,        // a starved wheel spins down proportionally
        gate: StructureGate::Idea(IdeaId::Dynamos),
        ..BLD
    },
    StructureDef {
        name: "Sunning Deck",
        base_costs: &[(ResourceId::Moonstone, 60.0)],
        energy_prod: 2.0, // winter ×0.75, summer ×1.25
        gate: StructureGate::Idea(IdeaId::SolarBasking),
        ..BLD
    },
    StructureDef {
        name: "Stream Wheel",
        base_costs: &[(ResourceId::Moonstone, 120.0), (ResourceId::Pavers, 25.0)],
        energy_prod: 5.0,
        gate: StructureGate::Idea(IdeaId::Millworks),
        ..BLD
    },
    StructureDef {
        name: "Star Basin",
        base_costs: &[
            (ResourceId::Moonstone, 150.0),
            (ResourceId::Badges, 1200.0),
            (ResourceId::Pavers, 12.0),
            (ResourceId::Schemes, 6.0),
        ],
        global_ratio: STAR_BASIN_GLOBAL_RATIO,
        productions: &[(ResourceId::Stardust, 0.0003)],
        cap_bonuses: &[(ResourceId::Stardust, 60.0)],
        energy_prod: 10.0,
        gate: StructureGate::Idea(IdeaId::Starcatching),
        ..BLD
    },
    StructureDef {
        name: "Prism Spinner",
        base_costs: &[
            (ResourceId::Moonstone, 250.0),
            (ResourceId::Stardust, 6.0),
            (ResourceId::Pavers, 30.0),
        ],
        converter: Some(Converter {
            inputs: &[(ResourceId::Moonstone, 0.004)],
            outputs: &[(ResourceId::Stardust, 0.0006)],
        }),
        energy_cons: 2.0,
        gate: StructureGate::Idea(IdeaId::PrismCraft),
        ..BLD
    },
    StructureDef {
        name: "Box Labyrinth",
        base_costs: &[(ResourceId::Brass, 30.0), (ResourceId::Tiles, 90.0)],
        cap_bonuses: &[(ResourceId::Curiosity, 1500.0)],
        energy_cons: 2.0,
        gate: StructureGate::Idea(IdeaId::LabyrinthTheory),
        ..BLD
    },
    StructureDef {
        name: "Moonlight Stage",
        base_costs: &[(ResourceId::Buttons, 250.0), (ResourceId::Moonstone, 20.0)],
        crowding_relief: 0.075,
        productions: &[(ResourceId::Charm, 0.25)],
        surplus_scaled: true, // an unpowered stage is dark by design
        gate: StructureGate::Idea(IdeaId::MoonlightArts),
        ..BLD
    },
    StructureDef {
        name: "Memory Chest",
        base_costs: &[
            (ResourceId::Hushfelt, 700.0),
            (ResourceId::Curiosity, 60000.0),
            (ResourceId::Starshards, 1.0),
            (ResourceId::Schemes, 25.0),
        ],
        ratio: 1.25,
        // Move carryover and déjà-vu chance land with the prestige/dreamtime
        // leaves.
        energy_cons: 20.0,
        gate: StructureGate::Idea(IdeaId::MemoryWeaving),
        ..BLD
    },
    StructureDef {
        name: "Yarn Computer",
        base_costs: &[
            (ResourceId::Stardust, 30.0),
            (ResourceId::Curiosity, 60000.0),
        ],
        productions: &[(ResourceId::Cleverness, 0.02)],
        energy_cons: 0.0, // special-cased: 2n + n²/2 in total at n copies
        gate: StructureGate::Idea(IdeaId::YarnComputing),
        ..BLD
    },
    StructureDef {
        name: "Unicorn Meadow",
        base_costs: &[(ResourceId::Unicorns, 2.0)],
        ratio: 1.75,
        productions: &[(ResourceId::Unicorns, 0.001)], // mystic leaf multiplies later
        gate: StructureGate::Idea(IdeaId::UnicornLore),
        ..BLD
    },
    // 38: Rainbow Arch — cross-leaf (mystic effects). @spec SIM-ECO-028, SIM-UNI-005
    StructureDef {
        name: "Rainbow Arch",
        base_costs: &[(ResourceId::Rainbows, 5000.0), (ResourceId::Cairns, 25.0)],
        ratio: 1.15,
        gate: StructureGate::Idea(IdeaId::ParadeRoutes),
        ..BLD
    },
    // 39: Sunstone — cross-leaf (ceremonies effects). @spec SIM-ECO-028, SIM-SUN-004
    StructureDef {
        name: "Sunstone",
        base_costs: &[
            (ResourceId::Moonstone, 8.0),
            (ResourceId::Cairns, 50.0),
            (ResourceId::Echoes, 1.0),
        ],
        ratio: 1.15,
        energy_cons: 1.0,
        gate: StructureGate::Idea(IdeaId::LongSunMasonry),
        ..BLD
    },
    // ---- Wave 6: cosmic outposts (ids 40..=52), each Site-gated ----
    // @spec SIM-ECO-028, SIM-SKY-002, SIM-SKY-005, SIM-SKY-008
    // 40: Lookout Post — skymap trickle + storage.
    StructureDef {
        name: "Lookout Post",
        base_costs: &[(ResourceId::Skymaps, 20.0)],
        productions: &[(ResourceId::Skymaps, 0.02)],
        cap_bonuses: &[(ResourceId::Skymaps, 500.0)],
        gate: StructureGate::Site(SiteId::FenceTop),
        ..BLD
    },
    // 41: Treehouse — houses 5; outdoor catnip.
    StructureDef {
        name: "Treehouse",
        base_costs: &[(ResourceId::Skymaps, 50.0), (ResourceId::Twigs, 500.0)],
        productions: &[(ResourceId::Catnip, 0.2)],
        housing: 5,
        gate: StructureGate::Site(SiteId::OldOak),
        outdoor: true,
        ..BLD
    },
    // 42: Weather Vane — skymap trickle + storage; energy −1.
    StructureDef {
        name: "Weather Vane",
        base_costs: &[(ResourceId::Skymaps, 100.0), (ResourceId::Moonmilk, 10.0)],
        productions: &[(ResourceId::Skymaps, 0.05)],
        cap_bonuses: &[(ResourceId::Skymaps, 2000.0)],
        energy_cons: 1.0,
        gate: StructureGate::Site(SiteId::Rooftops),
        outdoor: true,
        ..BLD
    },
    // 43: Solar Array — seasonal energy.
    StructureDef {
        name: "Solar Array",
        base_costs: &[(ResourceId::Skymaps, 200.0), (ResourceId::Moonstone, 100.0)],
        energy_prod: 12.0, // winter ×0.75, summer ×1.25 (seasonal, like the Sunning Deck)
        gate: StructureGate::Site(SiteId::SunnyHill),
        outdoor: true,
        ..BLD
    },
    // 44: Ranger Station — outdoor production +8%/bld; energy −2.
    StructureDef {
        name: "Ranger Station",
        base_costs: &[(ResourceId::Cometsilk, 20.0), (ResourceId::Moonmilk, 50.0)],
        energy_cons: 2.0,
        gate: StructureGate::Site(SiteId::MeadowsEdge),
        outdoor: true,
        outdoor_ratio: 0.08,
        ..BLD
    },
    // 45: Hushfelt Grotto — hushfelt trickle + storage; energy +4.
    StructureDef {
        name: "Hushfelt Grotto",
        base_costs: &[(ResourceId::Cometsilk, 40.0), (ResourceId::Moonmilk, 100.0)],
        productions: &[(ResourceId::Hushfelt, 0.05)],
        cap_bonuses: &[(ResourceId::Hushfelt, 50.0)],
        energy_prod: 4.0,
        gate: StructureGate::Site(SiteId::TheBrook),
        outdoor: true,
        ..BLD
    },
    // 46: Whisper Post — hushfelt + echo trickle; storage; energy −2.
    StructureDef {
        name: "Whisper Post",
        base_costs: &[(ResourceId::Cometsilk, 30.0), (ResourceId::Echoes, 2.0)],
        productions: &[(ResourceId::Hushfelt, 0.03), (ResourceId::Echoes, 0.0001)],
        cap_bonuses: &[(ResourceId::Hushfelt, 50.0)],
        energy_cons: 2.0,
        gate: StructureGate::Site(SiteId::WhisperingWoods),
        outdoor: true,
        ..BLD
    },
    // 47: Cairn Circle — outdoor production +12%/bld.
    StructureDef {
        name: "Cairn Circle",
        base_costs: &[(ResourceId::Cometsilk, 60.0), (ResourceId::Echoes, 5.0)],
        gate: StructureGate::Site(SiteId::TheFoothills),
        outdoor: true,
        outdoor_ratio: 0.12,
        ..BLD
    },
    // 48: Sky Dock — skymap trickle + storage; enables the Star Whale; energy −5.
    StructureDef {
        name: "Sky Dock",
        base_costs: &[(ResourceId::Cometsilk, 100.0), (ResourceId::Echoes, 20.0)],
        productions: &[(ResourceId::Skymaps, 0.2)],
        cap_bonuses: &[(ResourceId::Skymaps, 20000.0)],
        energy_cons: 5.0,
        gate: StructureGate::Site(SiteId::CloudNine),
        outdoor: true,
        ..BLD
    },
    // 49: Aurora Lantern — outdoor +25%/bld; +2 Long Sun cap; +50 Jar cap; energy −10.
    StructureDef {
        name: "Aurora Lantern",
        base_costs: &[(ResourceId::Echoes, 50.0), (ResourceId::JarsOfPurrs, 1.0)],
        cap_bonuses: &[(ResourceId::JarsOfPurrs, 50.0)],
        energy_cons: 10.0,
        gate: StructureGate::Site(SiteId::TheAurora),
        outdoor: true,
        outdoor_ratio: 0.25,
        long_sun_cap: 2.0,
        ..BLD
    },
    // 50: Star Nursery — unicorn + prism trickle; +100 Jar cap; energy −10.
    StructureDef {
        name: "Star Nursery",
        base_costs: &[(ResourceId::Echoes, 100.0), (ResourceId::JarsOfPurrs, 3.0)],
        productions: &[(ResourceId::Unicorns, 0.001), (ResourceId::Prisms, 0.0001)],
        cap_bonuses: &[(ResourceId::JarsOfPurrs, 100.0)],
        energy_cons: 10.0,
        gate: StructureGate::Site(SiteId::TheMilkyWay),
        outdoor: true,
        ..BLD
    },
    // 51: Far Beacon — outdoor +50%/bld; +500 Jar cap; energy −20.
    StructureDef {
        name: "Far Beacon",
        base_costs: &[(ResourceId::Echoes, 500.0), (ResourceId::JarsOfPurrs, 10.0)],
        cap_bonuses: &[(ResourceId::JarsOfPurrs, 500.0)],
        energy_cons: 20.0,
        gate: StructureGate::Site(SiteId::TheFarShore),
        outdoor: true,
        outdoor_ratio: 0.50,
        ..BLD
    },
    // 52: Wishing Star — the capstone: outdoor +200%; a large global bonus; energy −40.
    StructureDef {
        name: "Wishing Star",
        base_costs: &[
            (ResourceId::Echoes, 2000.0),
            (ResourceId::JarsOfPurrs, 50.0),
        ],
        global_ratio: 0.5,
        energy_cons: 40.0,
        gate: StructureGate::Site(SiteId::TheFurthestStar),
        outdoor: true,
        outdoor_ratio: 2.0,
        ..BLD
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
    /// Passive per-effective-worker yield; None for the Tinkerer, whose
    /// background crafting is owed by the kittens leaf's society wave.
    pub yield_per_tick: Option<(ResourceId, f64)>,
    pub gate: JobGate,
}

// @spec SIM-ECO-015, SIM-ECO-016, SIM-ECO-017, SIM-ECO-018
// @spec SIM-ECO-040, SIM-ECO-041, SIM-ECO-042, SIM-ECO-043
pub fn job_def(j: JobId) -> &'static JobDef {
    &JOBS[j as usize]
}

static JOBS: [JobDef; 8] = [
    JobDef {
        name: "Gardener",
        yield_per_tick: Some((ResourceId::Catnip, 0.15)),
        gate: JobGate::Always,
    },
    JobDef {
        name: "Twig Collector",
        yield_per_tick: Some((ResourceId::Twigs, 0.03)),
        gate: JobGate::Always,
    },
    JobDef {
        name: "Groomer",
        yield_per_tick: Some((ResourceId::Fluff, 0.02)),
        gate: JobGate::Idea(IdeaId::GroomedToPurrfection),
    },
    JobDef {
        name: "Observer",
        yield_per_tick: Some((ResourceId::Curiosity, 0.025)),
        gate: JobGate::RequiresBox,
    },
    JobDef {
        name: "Rummager",
        yield_per_tick: Some((ResourceId::Pebbles, 0.02)),
        gate: JobGate::Idea(IdeaId::CouchSpelunking),
    },
    JobDef {
        name: "Pouncer",
        yield_per_tick: Some((ResourceId::Pep, 0.06)),
        gate: JobGate::Idea(IdeaId::PouncingPractice),
    },
    JobDef {
        name: "Sunwarden",
        yield_per_tick: Some((ResourceId::Warmth, 0.0015)),
        gate: JobGate::Idea(IdeaId::SolarDevotion),
    },
    JobDef {
        name: "Tinkerer",
        yield_per_tick: None,
        gate: JobGate::Idea(IdeaId::Toymaking),
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

// ---- Blurbs ----

/// A run of a blurb: plain words, or words naming a resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlurbPart<'a> {
    Text(&'a str),
    /// The resource the words name, and the words as the blurb spells them.
    Resource(ResourceId, &'a str),
}

fn resource_by_identifier(id: &str) -> Option<ResourceId> {
    ResourceId::ALL.into_iter().find(|r| format!("{r:?}") == id)
}

/// A blurb read as plain words and resource references. A mark runs from a
/// `{` to the next `}`: `{text|Id}` names the resource whose identifier is
/// exactly `Id`, read as `text`, and `{Id}` reads as the identifier itself. A
/// mark naming no resource reads as its text, and a brace that opens or closes
/// no mark reads as itself, so a slip in the tables never hides words.
/// @spec SIM-CUR-014
pub fn blurb_parts(blurb: &str) -> Vec<BlurbPart<'_>> {
    let mut out = Vec::new();
    let mut rest = blurb;
    while let Some(open) = rest.find('{') {
        let Some(close) = rest[open + 1..].find('}') else {
            break;
        };
        let inside = &rest[open + 1..open + 1 + close];
        let (text, id) = inside.split_once('|').unwrap_or((inside, inside));
        if open > 0 {
            out.push(BlurbPart::Text(&rest[..open]));
        }
        out.push(match resource_by_identifier(id) {
            Some(r) => BlurbPart::Resource(r, text),
            None => BlurbPart::Text(text),
        });
        rest = &rest[open + 1 + close + 1..];
    }
    if !rest.is_empty() {
        out.push(BlurbPart::Text(rest));
    }
    out
}

/// A blurb's plain words: every mark read as its text. @spec SIM-CUR-014
pub fn blurb_text(blurb: &str) -> String {
    blurb_parts(blurb)
        .into_iter()
        .map(|part| match part {
            BlurbPart::Text(t) | BlurbPart::Resource(_, t) => t,
        })
        .collect()
}

// ---- Ideas ----

pub struct IdeaDef {
    pub name: &'static str,
    /// The joke and the plain effect, each resource it names marked
    /// (`blurb_parts`). @spec SIM-CUR-013, SIM-CUR-015
    pub blurb: &'static str,
    /// Curiosity price.
    pub cost: f64,
    /// Gate-craft components spent together with the curiosity, all-or-
    /// nothing. @spec SIM-CUR-007
    pub gate: &'static [(ResourceId, f64)],
    /// ALL prerequisites must be researched. @spec SIM-CUR-008
    pub prereqs: &'static [IdeaId],
}

const IDEA: IdeaDef = IdeaDef {
    name: "",
    blurb: "",
    cost: 0.0,
    gate: &[],
    prereqs: &[],
};

// @spec SIM-CUR-005
pub fn idea_def(i: IdeaId) -> &'static IdeaDef {
    &IDEAS[i as usize]
}

// Every entry ends in `..IDEA` — even the fully specified ones — so adding a
// field to IdeaDef never forces 57 edits.
#[allow(clippy::needless_update)]
static IDEAS: [IdeaDef; IDEA_COUNT] = [
    // ---- Band 0: curiosity only (the founding nine) ----
    IdeaDef {
        name: "Pantry Organization",
        blurb: "Stack the {catnip|Catnip}. Label nothing. Unlocks the Pantry.",
        cost: 15.0,
        ..IDEA
    },
    IdeaDef {
        name: "The Scratching Arts",
        blurb: "An ancient discipline. Unlocks the Scratching Post.",
        cost: 25.0,
        ..IDEA
    },
    IdeaDef {
        name: "Groomed to Purrfection",
        blurb: "Mutual grooming, industrialized. Unlocks the Groomer job.",
        cost: 40.0,
        prereqs: &[IdeaId::PantryOrganization],
        ..IDEA
    },
    IdeaDef {
        name: "Twig Architecture",
        blurb: "Load-bearing {twigs|Twigs}. Unlocks the Cat Tree and Twig Pile.",
        cost: 60.0,
        prereqs: &[IdeaId::ScratchingArts],
        ..IDEA
    },
    IdeaDef {
        name: "Selective Hearing",
        blurb: "You call, they consider. Gathering yields double.",
        cost: 80.0,
        prereqs: &[IdeaId::PantryOrganization],
        ..IDEA
    },
    IdeaDef {
        name: "Biscuit Making",
        blurb: "Rhythmic paw-kneading raises everyone's spirits. +10% happiness.",
        cost: 120.0,
        prereqs: &[IdeaId::GroomedToPurrfection],
        ..IDEA
    },
    IdeaDef {
        name: "Seasonal Awareness",
        blurb: "Thick coats and forward planning. Softer winters, visible forecast.",
        cost: 150.0,
        prereqs: &[IdeaId::TwigArchitecture],
        ..IDEA
    },
    IdeaDef {
        name: "Zoomie Channeling",
        blurb: "Harness the 3 a.m. energy. Zoomies run stronger and longer.",
        cost: 200.0,
        prereqs: &[IdeaId::BiscuitMaking],
        ..IDEA
    },
    IdeaDef {
        name: "Neighborhood Watch",
        blurb: "Word gets around about this place. Kittens arrive more often.",
        cost: 250.0,
        prereqs: &[IdeaId::SeasonalAwareness],
        ..IDEA
    },
    // ---- Band 0, continued ----
    IdeaDef {
        name: "Couch Spelunking",
        blurb: "The cushions hide worlds. Unlocks the Couch Mine, Rummaging, and the Rummager job.",
        cost: 120.0,
        prereqs: &[IdeaId::TwigArchitecture],
        ..IDEA
    },
    IdeaDef {
        name: "Horticulture",
        blurb: "Rows, apparently, help. Unlocks the Herb Garden.",
        cost: 180.0,
        prereqs: &[IdeaId::CouchSpelunking],
        ..IDEA
    },
    IdeaDef {
        name: "Emberkeeping",
        blurb: "A warm glow, carefully tended. Unlocks the Hearth.",
        cost: 300.0,
        prereqs: &[IdeaId::CouchSpelunking],
        ..IDEA
    },
    IdeaDef {
        name: "Notetaking",
        blurb: "Scratches that mean things. Unlocks the Study Nook.",
        cost: 350.0,
        prereqs: &[IdeaId::Horticulture],
        ..IDEA
    },
    IdeaDef {
        name: "Cozy Carpentry",
        blurb: "A sill, but structural. Unlocks the Window Perch.",
        cost: 400.0,
        prereqs: &[IdeaId::Emberkeeping],
        ..IDEA
    },
    IdeaDef {
        name: "Tinkering",
        blurb: "Small paws, small parts. Unlocks the Craft Corner and the {wand|Wands}, {tile|Tiles}, {lattice|Lattices}, and {cairn|Cairns} crafts.",
        cost: 500.0,
        prereqs: &[IdeaId::Notetaking],
        ..IDEA
    },
    IdeaDef {
        name: "Pack Rattery",
        blurb: "Keep everything, beautifully. Unlocks the Shed and the {badge|Badges} craft.",
        cost: 650.0,
        prereqs: &[IdeaId::Tinkering],
        ..IDEA
    },
    IdeaDef {
        name: "Unicorn Lore",
        blurb: "The old {stories|Stories} were load-bearing. Unlocks the Unicorn Meadow.",
        cost: 750.0,
        prereqs: &[IdeaId::Notetaking],
        ..IDEA
    },
    IdeaDef {
        name: "Oral Tradition",
        blurb: "Tales told tail-to-tail. Unlocks the Story Circle, the {quill|Quills} craft, and {charm|Charm}.",
        cost: 900.0,
        prereqs: &[IdeaId::Tinkering],
        ..IDEA
    },
    // ---- Band 1: curiosity + stories ----
    IdeaDef {
        name: "Gift Economy",
        blurb: "Generosity, with a stall. Unlocks the Gift Stand and the trade surface.",
        cost: 1100.0,
        prereqs: &[IdeaId::OralTradition],
        ..IDEA
    },
    IdeaDef {
        name: "Storycraft",
        blurb: "Editing, for cats. Unlocks the {story|Stories} craft itself.",
        cost: 1200.0,
        prereqs: &[IdeaId::OralTradition],
        ..IDEA
    },
    IdeaDef {
        name: "Sun Salutations",
        blurb: "Greet the beam; the beam greets back. Unlocks the Sun Room and {warmth|Warmth}.",
        cost: 1500.0,
        gate: &[(ResourceId::Stories, 5.0)],
        prereqs: &[IdeaId::Storycraft],
        ..IDEA
    },
    IdeaDef {
        name: "Excavation",
        blurb: "Dig with intent. Unlocks the Gravel Pit and the {brass|Brass} craft.",
        cost: 1800.0,
        gate: &[(ResourceId::Stories, 8.0)],
        prereqs: &[IdeaId::Storycraft],
        ..IDEA
    },
    IdeaDef {
        name: "Cogwheels",
        blurb: "Round things that turn other round things. Unlocks the Cogworks and the {cog|Cogs} craft.",
        cost: 2600.0,
        gate: &[(ResourceId::Stories, 12.0)],
        prereqs: &[IdeaId::Excavation],
        ..IDEA
    },
    IdeaDef {
        name: "Treat Alchemy",
        blurb: "Transmute {shinies|Shinies} into snacks. Unlocks the Treat Bakery.",
        cost: 3200.0,
        gate: &[(ResourceId::Stories, 15.0)],
        prereqs: &[IdeaId::Excavation],
        ..IDEA
    },
    IdeaDef {
        name: "Tea Ceremonies",
        blurb: "Steeped in decorum. Unlocks the Tea House and jamborees.",
        cost: 3800.0,
        gate: &[(ResourceId::Stories, 20.0)],
        prereqs: &[IdeaId::SunSalutations],
        ..IDEA
    },
    IdeaDef {
        name: "Solar Devotion",
        blurb: "Beyond basking lies belief. Unlocks the Sunbeam Altar, the Sunwarden job, and ceremonies.",
        cost: 4500.0,
        gate: &[(ResourceId::Stories, 25.0)],
        prereqs: &[IdeaId::SunSalutations],
        ..IDEA
    },
    IdeaDef {
        name: "Sky Charts",
        blurb: "The dots connect. Unlocks the Stargazing Ledge, sky events, and {skymaps|Skymaps}.",
        cost: 5200.0,
        gate: &[(ResourceId::Stories, 30.0)],
        prereqs: &[IdeaId::Cogwheels],
        ..IDEA
    },
    IdeaDef {
        name: "Porch Logistics",
        blurb: "Everything in its stack. Unlocks the Porch and the {kite|Kites} craft.",
        cost: 6000.0,
        gate: &[(ResourceId::Stories, 35.0)],
        prereqs: &[IdeaId::SkyCharts],
        ..IDEA
    },
    // ---- Band 2: curiosity + anthologies ----
    IdeaDef {
        name: "Anthology Binding",
        blurb: "{Stories}, stitched into a spine. Unlocks the {anthology|Anthologies} craft.",
        cost: 7000.0,
        gate: &[(ResourceId::Stories, 40.0)],
        prereqs: &[IdeaId::SkyCharts],
        ..IDEA
    },
    IdeaDef {
        name: "Dew Harvest",
        blurb: "Morning, bottled. Unlocks the Dew Collector.",
        cost: 8000.0,
        gate: &[(ResourceId::Anthologies, 10.0)],
        prereqs: &[IdeaId::AnthologyBinding],
        ..IDEA
    },
    IdeaDef {
        name: "Toymaking",
        blurb: "Serious fun, seriously made. Unlocks the Toy Workshop and the Tinkerer job.",
        cost: 10000.0,
        gate: &[(ResourceId::Anthologies, 15.0)],
        prereqs: &[IdeaId::AnthologyBinding],
        ..IDEA
    },
    IdeaDef {
        name: "Ceramic Arts",
        blurb: "Mud, remembered by fire. Unlocks the Kiln.",
        cost: 12000.0,
        gate: &[(ResourceId::Anthologies, 20.0)],
        prereqs: &[IdeaId::DewHarvest],
        ..IDEA
    },
    IdeaDef {
        name: "Deep Curiosity",
        blurb: "Ask the question under the question. Unlocks the Whisker Lab.",
        cost: 14000.0,
        gate: &[(ResourceId::Anthologies, 25.0)],
        prereqs: &[IdeaId::Toymaking],
        ..IDEA
    },
    IdeaDef {
        name: "Moonlight Arts",
        blurb: "The night has an audience. Unlocks the Moonlight Stage.",
        cost: 16000.0,
        gate: &[(ResourceId::Anthologies, 30.0)],
        prereqs: &[IdeaId::Toymaking],
        ..IDEA
    },
    IdeaDef {
        name: "Labyrinth Theory",
        blurb: "Boxes within boxes, rigorously. Unlocks the Box Labyrinth.",
        cost: 18000.0,
        gate: &[(ResourceId::Anthologies, 35.0)],
        prereqs: &[IdeaId::DeepCuriosity],
        ..IDEA
    },
    IdeaDef {
        name: "Grand Schemes",
        blurb: "Plans with appendices. Unlocks the {scheme|Schemes} craft.",
        cost: 20000.0,
        gate: &[(ResourceId::Anthologies, 40.0)],
        prereqs: &[IdeaId::DeepCuriosity],
        ..IDEA
    },
    // ---- Band 3: curiosity + schemes ----
    IdeaDef {
        name: "Dynamos",
        blurb: "Spin {dew|Dew} into hum. Unlocks the Dynamo Wheel.",
        cost: 25000.0,
        gate: &[(ResourceId::Schemes, 8.0)],
        prereqs: &[IdeaId::GrandSchemes],
        ..IDEA
    },
    IdeaDef {
        name: "Solar Basking",
        blurb: "Lying down, but productive. Unlocks the Sunning Deck.",
        cost: 28000.0,
        gate: &[(ResourceId::Schemes, 10.0)],
        prereqs: &[IdeaId::GrandSchemes],
        ..IDEA
    },
    IdeaDef {
        name: "Millworks",
        blurb: "The stream was going that way anyway. Unlocks the Stream Wheel.",
        cost: 32000.0,
        gate: &[(ResourceId::Schemes, 12.0)],
        prereqs: &[IdeaId::SolarBasking],
        ..IDEA
    },
    IdeaDef {
        name: "Moonmetallurgy",
        blurb: "Alloys by lamplight. Unlocks the {moonsteel|Moonsteel} and {paver|Pavers} crafts.",
        cost: 36000.0,
        gate: &[(ResourceId::Schemes, 15.0)],
        prereqs: &[IdeaId::Dynamos],
        ..IDEA
    },
    IdeaDef {
        name: "Starcatching",
        blurb: "Hold a bowl very still. Unlocks the Star Basin and the {cometsilk|Cometsilk} craft.",
        cost: 42000.0,
        gate: &[(ResourceId::Schemes, 20.0)],
        prereqs: &[IdeaId::Moonmetallurgy],
        ..IDEA
    },
    IdeaDef {
        name: "Prism Craft",
        blurb: "Bend light until it works. Unlocks the Prism Spinner.",
        cost: 48000.0,
        gate: &[(ResourceId::Schemes, 25.0)],
        prereqs: &[IdeaId::Starcatching],
        ..IDEA
    },
    IdeaDef {
        name: "Balloon Craft",
        blurb: "Logistics, but up. Unlocks the {balloon|Balloons} craft.",
        cost: 55000.0,
        gate: &[(ResourceId::Schemes, 30.0)],
        prereqs: &[IdeaId::Moonmetallurgy],
        ..IDEA
    },
    IdeaDef {
        name: "Yarn Computing",
        blurb: "The string remembers. Unlocks the Yarn Computer.",
        cost: 65000.0,
        gate: &[(ResourceId::Schemes, 40.0)],
        prereqs: &[IdeaId::Starcatching],
        ..IDEA
    },
    // ---- Band 4: curiosity + mystic currencies (content lands waves 4–6) ----
    IdeaDef {
        name: "Parade Routes",
        blurb: "Glitter needs an itinerary. Unlocks Unicorn Parades.",
        cost: 30000.0,
        gate: &[(ResourceId::Schemes, 15.0)],
        prereqs: &[IdeaId::UnicornLore, IdeaId::GrandSchemes],
        ..IDEA
    },
    IdeaDef {
        name: "Pen Pal Networks",
        blurb: "Letters between meadows. Unlocks Pen Pals.",
        cost: 26000.0,
        gate: &[(ResourceId::Schemes, 10.0)],
        prereqs: &[IdeaId::GiftEconomy, IdeaId::GrandSchemes],
        ..IDEA
    },
    IdeaDef {
        name: "Scout Picnicking",
        blurb: "Diplomacy by sandwich. Unlocks Scout Picnics.",
        cost: 20000.0,
        gate: &[(ResourceId::Schemes, 5.0)],
        prereqs: &[IdeaId::GiftEconomy, IdeaId::GrandSchemes],
        ..IDEA
    },
    IdeaDef {
        name: "Dream Tending",
        blurb: "Someone should water them. Unlocks the dreaming surface.",
        cost: 90000.0,
        gate: &[(ResourceId::Starshards, 3.0)],
        prereqs: &[IdeaId::ParadeRoutes],
        ..IDEA
    },
    IdeaDef {
        name: "Night Court Etiquette",
        blurb: "Bow, but sleepily. Unlocks Promises.",
        cost: 150000.0,
        gate: &[(ResourceId::Echoes, 5.0)],
        prereqs: &[IdeaId::DreamTending],
        ..IDEA
    },
    IdeaDef {
        name: "Hushfelt Seeking",
        blurb: "Listen for the softest quiet. Unlocks {hushfelt|Hushfelt} sources.",
        cost: 80000.0,
        gate: &[(ResourceId::Echoes, 1.0)],
        prereqs: &[IdeaId::YarnComputing],
        ..IDEA
    },
    IdeaDef {
        name: "Memory Weaving",
        blurb: "Keep the good parts on purpose. Unlocks the Memory Chest.",
        cost: 100000.0,
        gate: &[(ResourceId::Starshards, 3.0)],
        prereqs: &[IdeaId::HushfeltSeeking],
        ..IDEA
    },
    IdeaDef {
        name: "Skyward Longing",
        blurb: "The meadow has an upstairs. Unlocks the cosmic surface and the {moonmilk|Moonmilk} craft.",
        cost: 90000.0,
        gate: &[(ResourceId::Echoes, 2.0)],
        prereqs: &[IdeaId::YarnComputing, IdeaId::PorchLogistics],
        ..IDEA
    },
    IdeaDef {
        name: "Dreamweaving",
        blurb: "Fabric with a thread count of one nap. Unlocks the {dreamweave|Dreamweave} craft.",
        cost: 120000.0,
        gate: &[(ResourceId::Echoes, 5.0)],
        prereqs: &[IdeaId::MemoryWeaving],
        ..IDEA
    },
    IdeaDef {
        name: "Adventurous Spirit",
        blurb: "What if, but bravely. Unlocks the adventures surface.",
        cost: 40000.0,
        gate: &[(ResourceId::Schemes, 20.0)],
        prereqs: &[IdeaId::GrandSchemes],
        ..IDEA
    },
    IdeaDef {
        name: "Dream Loomcraft",
        blurb: "Warp of wonder, weft of rest. Unlocks the Dream Loom.",
        cost: 200000.0,
        gate: &[(ResourceId::Starshards, 10.0)],
        prereqs: &[IdeaId::MemoryWeaving],
        ..IDEA
    },
    IdeaDef {
        name: "Deep Nap Theory",
        blurb: "Peer-reviewed drowsing. Unlocks the Deep Nap.",
        cost: 300000.0,
        gate: &[(ResourceId::Echoes, 10.0)],
        prereqs: &[IdeaId::DreamLoomcraft],
        ..IDEA
    },
    // ---- Band 0, appended late (ids never reorder) ----
    IdeaDef {
        name: "Pouncing Practice",
        blurb: "Wiggle, then commit. Unlocks expeditions and the Pouncer job.",
        cost: 200.0,
        prereqs: &[IdeaId::PantryOrganization],
        ..IDEA
    },
    // ---- Band 4, appended: the wonder ideas (ids never reorder) ----
    IdeaDef {
        name: "Long Sun Masonry",
        blurb: "Lay a stone that remembers the light. Unlocks the Sunstone.",
        cost: 120000.0,
        gate: &[(ResourceId::Echoes, 3.0)],
        prereqs: &[IdeaId::SolarDevotion, IdeaId::SkywardLonging],
        ..IDEA
    },
    IdeaDef {
        name: "Skyward Song",
        blurb: "A tune that carries to the cosmos. Summons Star Whale visits.",
        cost: 180000.0,
        gate: &[(ResourceId::Echoes, 8.0)],
        prereqs: &[IdeaId::SkywardLonging, IdeaId::NightCourtEtiquette],
        ..IDEA
    },
];

// ---- Crafts ----

/// One batch: spend `inputs`, gain 1 × (1 + craft-ratio effects) of the
/// output. Locked until the granting idea. @spec SIM-ECO-033, SIM-ECO-034
pub struct CraftDef {
    pub name: &'static str,
    pub inputs: &'static [(ResourceId, f64)],
    pub gate: Option<IdeaId>,
    /// Tinkerer work tier (SIM-KIT-026's rank comparison). @spec SIM-ECO-043
    pub tier: u8,
}

pub fn craft_def(c: CraftId) -> &'static CraftDef {
    &CRAFTS[c as usize]
}

/// The crafted resource each recipe outputs (1:1 with the RES table).
pub fn craft_output(c: CraftId) -> ResourceId {
    match c {
        CraftId::Wands => ResourceId::Wands,
        CraftId::Tiles => ResourceId::Tiles,
        CraftId::Badges => ResourceId::Badges,
        CraftId::Brass => ResourceId::Brass,
        CraftId::Cogs => ResourceId::Cogs,
        CraftId::Moonsteel => ResourceId::Moonsteel,
        CraftId::Lattices => ResourceId::Lattices,
        CraftId::Pavers => ResourceId::Pavers,
        CraftId::Quills => ResourceId::Quills,
        CraftId::Stories => ResourceId::Stories,
        CraftId::Anthologies => ResourceId::Anthologies,
        CraftId::Schemes => ResourceId::Schemes,
        CraftId::Cairns => ResourceId::Cairns,
        CraftId::Kites => ResourceId::Kites,
        CraftId::Balloons => ResourceId::Balloons,
        CraftId::Moonmilk => ResourceId::Moonmilk,
        CraftId::Dreamweave => ResourceId::Dreamweave,
        CraftId::Cometsilk => ResourceId::Cometsilk,
    }
}

static CRAFTS: [CraftDef; 18] = [
    CraftDef {
        name: "Wands",
        inputs: &[(ResourceId::Twigs, 15.0)],
        gate: Some(IdeaId::Tinkering),
        tier: 1,
    },
    CraftDef {
        name: "Tiles",
        inputs: &[(ResourceId::Pebbles, 25.0)],
        gate: Some(IdeaId::Tinkering),
        tier: 1,
    },
    CraftDef {
        name: "Badges",
        inputs: &[(ResourceId::Buttons, 12.0)],
        gate: Some(IdeaId::PackRattery),
        tier: 1,
    },
    CraftDef {
        name: "Brass",
        inputs: &[(ResourceId::Buttons, 10.0), (ResourceId::Embers, 10.0)],
        gate: Some(IdeaId::Excavation),
        tier: 2,
    },
    CraftDef {
        name: "Cogs",
        inputs: &[(ResourceId::Brass, 8.0)],
        gate: Some(IdeaId::Cogwheels),
        tier: 3,
    },
    CraftDef {
        name: "Moonsteel",
        inputs: &[(ResourceId::Moonstone, 1.0), (ResourceId::Brass, 8.0)],
        gate: Some(IdeaId::Moonmetallurgy),
        tier: 4,
    },
    CraftDef {
        name: "Lattices",
        inputs: &[(ResourceId::Wands, 5.0)],
        gate: Some(IdeaId::Tinkering),
        tier: 2,
    },
    CraftDef {
        name: "Pavers",
        inputs: &[(ResourceId::Tiles, 250.0), (ResourceId::Brass, 3.0)],
        gate: Some(IdeaId::Moonmetallurgy),
        tier: 3,
    },
    CraftDef {
        name: "Quills",
        inputs: &[(ResourceId::Feathers, 18.0)],
        gate: Some(IdeaId::OralTradition),
        tier: 1,
    },
    CraftDef {
        name: "Stories",
        inputs: &[(ResourceId::Charm, 40.0), (ResourceId::Quills, 3.0)],
        gate: Some(IdeaId::Storycraft),
        tier: 2,
    },
    CraftDef {
        name: "Anthologies",
        inputs: &[(ResourceId::Curiosity, 1000.0), (ResourceId::Stories, 5.0)],
        gate: Some(IdeaId::AnthologyBinding),
        tier: 3,
    },
    CraftDef {
        name: "Schemes",
        inputs: &[
            (ResourceId::Curiosity, 2500.0),
            (ResourceId::Anthologies, 3.0),
        ],
        gate: Some(IdeaId::GrandSchemes),
        tier: 4,
    },
    CraftDef {
        name: "Cairns",
        inputs: &[
            (ResourceId::Wands, 3.0),
            (ResourceId::Tiles, 5.0),
            (ResourceId::Badges, 1.0),
        ],
        gate: Some(IdeaId::Tinkering),
        tier: 1,
    },
    CraftDef {
        name: "Kites",
        inputs: &[
            (ResourceId::Skymaps, 3.0),
            (ResourceId::Badges, 15.0),
            (ResourceId::Lattices, 10.0),
        ],
        gate: Some(IdeaId::PorchLogistics),
        tier: 3,
    },
    CraftDef {
        name: "Balloons",
        inputs: &[
            (ResourceId::Moonsteel, 125.0),
            (ResourceId::Kites, 20.0),
            (ResourceId::Schemes, 1.0),
        ],
        gate: Some(IdeaId::BalloonCraft),
        tier: 5,
    },
    CraftDef {
        name: "Moonmilk",
        inputs: &[(ResourceId::Dew, 750.0)],
        gate: Some(IdeaId::SkywardLonging),
        tier: 4,
    },
    CraftDef {
        name: "Dreamweave",
        inputs: &[
            (ResourceId::Hushfelt, 100.0),
            (ResourceId::Moonsteel, 250.0),
        ],
        gate: Some(IdeaId::Dreamweaving),
        tier: 5,
    },
    CraftDef {
        name: "Cometsilk",
        inputs: &[(ResourceId::Stardust, 25.0)],
        gate: Some(IdeaId::Starcatching),
        tier: 5,
    },
];

// ---- Refinements ----

/// One-time improvements priced in curiosity + materials, gated by ideas,
/// each contributing effect-key values permanently. @spec SIM-CUR-010
pub struct RefinementDef {
    pub name: &'static str,
    /// Marked as an idea's blurb is. @spec SIM-CUR-013, SIM-CUR-015
    pub blurb: &'static str,
    /// Curiosity price, spent together with the materials, all-or-nothing.
    pub cost: f64,
    pub materials: &'static [(ResourceId, f64)],
    /// Locked until this idea is researched.
    pub gate: IdeaId,
    pub effects: &'static [(Effect, f64)],
    /// When set, the effect values scale by this structure's owned count
    /// (Catnip Cellars per Pantry, Thicker Blankets per Hearth).
    pub per_structure: Option<StructureId>,
}

pub fn refinement_def(r: RefinementId) -> &'static RefinementDef {
    &REFINEMENTS[r as usize]
}

const REF: RefinementDef = RefinementDef {
    name: "",
    blurb: "",
    cost: 0.0,
    materials: &[],
    gate: IdeaId::PantryOrganization,
    effects: &[],
    per_structure: None,
};

#[allow(clippy::needless_update)]
static REFINEMENTS: [RefinementDef; REFINEMENT_COUNT] = [
    RefinementDef {
        name: "Pebble Combs",
        blurb: "Grooming tools for the garden. Gardeners +50%.",
        cost: 100.0,
        materials: &[(ResourceId::Pebbles, 275.0)],
        gate: IdeaId::CouchSpelunking,
        effects: &[(Effect::JobRatio(JobId::Gardener), 0.50)],
        ..REF
    },
    RefinementDef {
        name: "Button Combs",
        blurb: "Smoother teeth, finer rows. Gardeners +30%.",
        cost: 200.0,
        materials: &[(ResourceId::Buttons, 25.0)],
        gate: IdeaId::Emberkeeping,
        effects: &[(Effect::JobRatio(JobId::Gardener), 0.30)],
        ..REF
    },
    RefinementDef {
        name: "Brass Combs",
        blurb: "They gleam and they rake. Gardeners +50%.",
        cost: 5000.0,
        materials: &[(ResourceId::Brass, 75.0)],
        gate: IdeaId::Cogwheels,
        effects: &[(Effect::JobRatio(JobId::Gardener), 0.50)],
        ..REF
    },
    RefinementDef {
        name: "Moonsteel Combs",
        blurb: "Forged by lamplight for the leaf. Gardeners +50%.",
        cost: 38000.0,
        materials: &[(ResourceId::Moonsteel, 10.0)],
        gate: IdeaId::Moonmetallurgy,
        effects: &[(Effect::JobRatio(JobId::Gardener), 0.50)],
        ..REF
    },
    RefinementDef {
        name: "Pebble Snips",
        blurb: "Crude but decisive. Twig Collectors +70%.",
        cost: 100.0,
        materials: &[(ResourceId::Pebbles, 500.0)],
        gate: IdeaId::CouchSpelunking,
        effects: &[(Effect::JobRatio(JobId::TwigCollector), 0.70)],
        ..REF
    },
    RefinementDef {
        name: "Button Shears",
        blurb: "Snip with dignity. Twig Collectors +50%.",
        cost: 200.0,
        materials: &[(ResourceId::Buttons, 50.0)],
        gate: IdeaId::Emberkeeping,
        effects: &[(Effect::JobRatio(JobId::TwigCollector), 0.50)],
        ..REF
    },
    RefinementDef {
        name: "Brass Clippers",
        blurb: "A satisfying click. Twig Collectors +50%.",
        cost: 20000.0,
        materials: &[(ResourceId::Brass, 75.0)],
        gate: IdeaId::Cogwheels,
        effects: &[(Effect::JobRatio(JobId::TwigCollector), 0.50)],
        ..REF
    },
    RefinementDef {
        name: "Moonsteel Pruners",
        blurb: "The {twig|Twigs} barely notices. Twig Collectors +50%.",
        cost: 38000.0,
        materials: &[(ResourceId::Moonsteel, 10.0)],
        gate: IdeaId::Moonmetallurgy,
        effects: &[(Effect::JobRatio(JobId::TwigCollector), 0.50)],
        ..REF
    },
    RefinementDef {
        name: "Deep Pockets",
        blurb: "Where do they keep it all? Rummagers +50%.",
        cost: 1000.0,
        materials: &[(ResourceId::Buttons, 50.0)],
        gate: IdeaId::PackRattery,
        effects: &[(Effect::JobRatio(JobId::Rummager), 0.50)],
        ..REF
    },
    RefinementDef {
        name: "Sorting Claws",
        blurb: "{Fluff} from treasure, instantly. Rummagers +50%.",
        cost: 15000.0,
        materials: &[(ResourceId::Brass, 25.0)],
        gate: IdeaId::Excavation,
        effects: &[(Effect::JobRatio(JobId::Rummager), 0.50)],
        ..REF
    },
    RefinementDef {
        name: "Bolas of Yarn",
        blurb: "Tangles at a distance. Pounce ratio +1.",
        cost: 1000.0,
        materials: &[(ResourceId::Twigs, 50.0), (ResourceId::Pebbles, 250.0)],
        gate: IdeaId::PouncingPractice,
        effects: &[(Effect::PounceRatio, 1.0)],
        ..REF
    },
    RefinementDef {
        name: "Pounce Pads",
        blurb: "Silent landings, louder hauls. Pounce ratio +2.",
        cost: 2000.0,
        materials: &[(ResourceId::Buttons, 750.0)],
        gate: IdeaId::Emberkeeping,
        effects: &[(Effect::PounceRatio, 2.0)],
        ..REF
    },
    RefinementDef {
        name: "Whisker Guards",
        blurb: "Protect the instruments. Pounce ratio +0.5.",
        cost: 10000.0,
        materials: &[(ResourceId::Brass, 50.0)],
        gate: IdeaId::Cogwheels,
        effects: &[(Effect::PounceRatio, 0.5)],
        ..REF
    },
    RefinementDef {
        name: "Moonsteel Mittens",
        blurb: "Soft outside, serious inside. Pounce ratio +0.5.",
        cost: 50000.0,
        materials: &[(ResourceId::Moonsteel, 25.0)],
        gate: IdeaId::Moonmetallurgy,
        effects: &[(Effect::PounceRatio, 0.5)],
        ..REF
    },
    RefinementDef {
        name: "Deep Shelves",
        blurb: "Higher than a cat can reach, almost. +100 {twig|Twigs} cap, +150 {pebble|Pebbles} cap.",
        cost: 500.0,
        materials: &[
            (ResourceId::Twigs, 1000.0),
            (ResourceId::Pebbles, 750.0),
            (ResourceId::Buttons, 50.0),
        ],
        gate: IdeaId::PackRattery,
        effects: &[
            (Effect::MaxCap(ResourceId::Twigs), 100.0),
            (Effect::MaxCap(ResourceId::Pebbles), 150.0),
        ],
        ..REF
    },
    RefinementDef {
        name: "Stacked Crates",
        blurb: "A skyline of storage. +100 {twig|Twigs}, +150 {pebble|Pebbles}, +25 {button|Buttons} cap.",
        cost: 800.0,
        materials: &[(ResourceId::Wands, 25.0), (ResourceId::Tiles, 10.0)],
        gate: IdeaId::PackRattery,
        effects: &[
            (Effect::MaxCap(ResourceId::Twigs), 100.0),
            (Effect::MaxCap(ResourceId::Pebbles), 150.0),
            (Effect::MaxCap(ResourceId::Buttons), 25.0),
        ],
        ..REF
    },
    RefinementDef {
        name: "Labeled Jars",
        blurb:
            "Labels nobody reads, order everybody feels. +50 {button|Buttons}, +30 {ember|Embers}, +10 {moonstone|Moonstone} cap.",
        cost: 60000.0,
        materials: &[(ResourceId::Moonstone, 25.0), (ResourceId::Brass, 200.0)],
        gate: IdeaId::Moonmetallurgy,
        effects: &[
            (Effect::MaxCap(ResourceId::Buttons), 50.0),
            (Effect::MaxCap(ResourceId::Embers), 30.0),
            (Effect::MaxCap(ResourceId::Moonstone), 10.0),
        ],
        ..REF
    },
    RefinementDef {
        name: "Catnip Cellars",
        blurb: "Cool, dark, fragrant. Pantries also hold +125 {catnip|Catnip} each.",
        cost: 50000.0,
        materials: &[(ResourceId::Brass, 125.0), (ResourceId::Schemes, 5.0)],
        gate: IdeaId::Dynamos,
        effects: &[(Effect::MaxCap(ResourceId::Catnip), 125.0)],
        per_structure: Some(StructureId::Pantry),
        ..REF
    },
    RefinementDef {
        name: "Bigger Baskets",
        blurb: "Weave wider. +200 {pebble|Pebbles} cap, +50 {button|Buttons} cap.",
        cost: 15000.0,
        materials: &[(ResourceId::Badges, 50.0), (ResourceId::Brass, 25.0)],
        gate: IdeaId::Cogwheels,
        effects: &[
            (Effect::MaxCap(ResourceId::Pebbles), 200.0),
            (Effect::MaxCap(ResourceId::Buttons), 50.0),
        ],
        ..REF
    },
    RefinementDef {
        name: "Bellows",
        blurb: "More air, more glow. Hearth {embers|Embers} +0.002/t each.",
        cost: 5000.0,
        materials: &[(ResourceId::Wands, 35.0), (ResourceId::Pebbles, 2000.0)],
        gate: IdeaId::Emberkeeping,
        effects: &[(
            Effect::StructureOutputAdd(StructureId::Hearth, ResourceId::Embers),
            0.002,
        )],
        ..REF
    },
    RefinementDef {
        name: "Ember Beds",
        blurb: "Ceramic cradles keep the coals company. Hearth {embers|Embers} +0.003/t each.",
        cost: 35000.0,
        materials: &[(ResourceId::Anthologies, 5.0)],
        gate: IdeaId::CeramicArts,
        effects: &[(
            Effect::StructureOutputAdd(StructureId::Hearth, ResourceId::Embers),
            0.003,
        )],
        ..REF
    },
    RefinementDef {
        name: "Story Press",
        blurb: "The gears tell tales. Cogworks +0.005 {stories|Stories}/t each.",
        cost: 7500.0,
        materials: &[(ResourceId::Cogs, 45.0)],
        gate: IdeaId::Cogwheels,
        effects: &[(
            Effect::StructureOutputAdd(StructureId::Cogworks, ResourceId::Stories),
            0.005,
        )],
        ..REF
    },
    RefinementDef {
        name: "Better Bindings",
        blurb: "Spines that survive enthusiasm. {Stories}, {anthologies|Anthologies}, and {schemes|Schemes} craft +25%.",
        cost: 25000.0,
        materials: &[(ResourceId::Anthologies, 5.0)],
        gate: IdeaId::AnthologyBinding,
        effects: &[
            (Effect::CraftRatioFor(CraftId::Stories), 0.25),
            (Effect::CraftRatioFor(CraftId::Anthologies), 0.25),
            (Effect::CraftRatioFor(CraftId::Schemes), 0.25),
        ],
        ..REF
    },
    RefinementDef {
        name: "Field Notes",
        blurb: "Margins full of paw prints. Observers +30%.",
        cost: 250.0,
        materials: &[(ResourceId::Pebbles, 100.0)],
        gate: IdeaId::Notetaking,
        effects: &[(Effect::JobRatio(JobId::Observer), 0.30)],
        ..REF
    },
    RefinementDef {
        name: "Star Diaries",
        blurb: "The sky repeats itself, if you write it down. Sky-event chance ×2.",
        cost: 20000.0,
        materials: &[(ResourceId::Kites, 5.0)],
        gate: IdeaId::SkyCharts,
        effects: &[(Effect::SkyEventChanceRatio, 1.0)],
        ..REF
    },
    RefinementDef {
        name: "Patient Eyes",
        blurb: "Somebody is always looking up. Auto-observe chance ×2.",
        cost: 25000.0,
        materials: &[(ResourceId::Kites, 10.0)],
        gate: IdeaId::SkyCharts,
        effects: &[(Effect::AutoObserveChanceRatio, 1.0)],
        ..REF
    },
    RefinementDef {
        name: "Tidy-Up Rota",
        blurb: "Whose turn is it? A chart knows. Toy Workshop mess 2 → −2.",
        cost: 10000.0,
        materials: &[(ResourceId::Stories, 15.0)],
        gate: IdeaId::Toymaking,
        effects: &[(Effect::StructureMessDelta(StructureId::ToyWorkshop), -4.0)],
        ..REF
    },
    RefinementDef {
        name: "Group Lessons",
        blurb: "Everyone keeps everyone sharp. Skill decay removed.",
        cost: 45000.0,
        materials: &[(ResourceId::Anthologies, 10.0)],
        gate: IdeaId::DeepCuriosity,
        effects: &[],
        ..REF
    },
    RefinementDef {
        name: "Encouraging Mews",
        blurb: "Praise, frequently and specifically. Skill XP +50%.",
        cost: 20000.0,
        materials: &[(ResourceId::Anthologies, 5.0)],
        gate: IdeaId::Toymaking,
        effects: &[(Effect::SkillXpRatio, 0.50)],
        ..REF
    },
    RefinementDef {
        name: "Nimble Paws",
        blurb: "Practice, but faster. Tinkerer speed +100%.",
        cost: 30000.0,
        materials: &[(ResourceId::Cogs, 10.0), (ResourceId::Anthologies, 5.0)],
        gate: IdeaId::Toymaking,
        effects: &[(Effect::TinkererSpeed, 1.0)],
        ..REF
    },
    RefinementDef {
        name: "Patient Paws",
        blurb: "Slow is smooth; smooth is fast. Tinkerer speed +100%.",
        cost: 75000.0,
        materials: &[(ResourceId::Schemes, 10.0)],
        gate: IdeaId::Dynamos,
        effects: &[(Effect::TinkererSpeed, 1.0)],
        ..REF
    },
    RefinementDef {
        name: "Clockwork Paws",
        blurb: "Tick, tock, toy. Tinkerer speed +200%.",
        cost: 150000.0,
        materials: &[(ResourceId::Schemes, 25.0)],
        gate: IdeaId::YarnComputing,
        effects: &[(Effect::TinkererSpeed, 2.0)],
        ..REF
    },
    RefinementDef {
        name: "Warm Welcome Mat",
        blurb: "It says 'mew'. Arrival chance +5% (inside the cap).",
        cost: 4000.0,
        materials: &[(ResourceId::Stories, 20.0)],
        gate: IdeaId::TeaCeremonies,
        effects: &[(Effect::ArrivalChanceInCap, 0.05)],
        ..REF
    },
    RefinementDef {
        name: "Thicker Blankets",
        blurb: "{Warmth} you can knead. +1 happiness point per Hearth.",
        cost: 8000.0,
        materials: &[(ResourceId::Quills, 50.0)],
        gate: IdeaId::TeaCeremonies,
        effects: &[(Effect::HappinessPoints, 1.0)],
        per_structure: Some(StructureId::Hearth),
        ..REF
    },
    RefinementDef {
        name: "Dew Funnels",
        blurb: "The morning pours itself. Dew Collectors +50%.",
        cost: 30000.0,
        materials: &[(ResourceId::Lattices, 40.0)],
        gate: IdeaId::DewHarvest,
        effects: &[(Effect::StructureRatio(StructureId::DewCollector), 0.50)],
        ..REF
    },
];

// ---- House Rules ----

/// Mutually exclusive pairs, charm-priced; adopting one permanently blocks
/// its sibling for the current meadow. @spec SIM-CUR-011
pub struct RuleDef {
    pub name: &'static str,
    /// Marked as an idea's blurb is. @spec SIM-CUR-013, SIM-CUR-015
    pub blurb: &'static str,
    pub cost_charm: f64,
    /// The sibling this rule permanently blocks (and is blocked by).
    pub blocks: RuleId,
    pub effects: &'static [(Effect, f64)],
}

pub fn rule_def(r: RuleId) -> &'static RuleDef {
    &RULES[r as usize]
}

static RULES: [RuleDef; 8] = [
    RuleDef {
        name: "Early Birds",
        blurb: "Dawn patrol tends the patch. {Catnip} production +10%.",
        cost_charm: 150.0,
        blocks: RuleId::NightOwls,
        effects: &[(Effect::ResourceRatio(ResourceId::Catnip), 0.10)],
    },
    RuleDef {
        name: "Night Owls",
        blurb: "The best questions come after dark. {Curiosity} production +10%.",
        cost_charm: 150.0,
        blocks: RuleId::EarlyBirds,
        effects: &[(Effect::ResourceRatio(ResourceId::Curiosity), 0.10)],
    },
    RuleDef {
        name: "Open Meadow",
        blurb: "The gate stands ajar. Arrival chance +10% (inside the cap).",
        cost_charm: 1500.0,
        blocks: RuleId::CozyBurrows,
        effects: &[(Effect::ArrivalChanceInCap, 0.10)],
    },
    RuleDef {
        name: "Cozy Burrows",
        blurb: "Everyone gets a nook. +5 happiness points.",
        cost_charm: 1500.0,
        blocks: RuleId::OpenMeadow,
        effects: &[(Effect::HappinessPoints, 5.0)],
    },
    RuleDef {
        name: "Sharing Circle",
        blurb: "One {feather|Feathers}, many admirers. Luxury drain −25%.",
        cost_charm: 4000.0,
        blocks: RuleId::PersonalStashes,
        effects: &[(Effect::LuxuryDemandRatio, -0.25)],
    },
    RuleDef {
        name: "Personal Stashes",
        blurb: "Mine, labeled and defended. All caps +10%.",
        cost_charm: 4000.0,
        blocks: RuleId::SharingCircle,
        effects: &[(Effect::CapRatio, 0.10)],
    },
    RuleDef {
        name: "Loud Purring",
        blurb: "Audible contentment carries. {Charm} production +20%.",
        cost_charm: 15000.0,
        blocks: RuleId::QuietContemplation,
        effects: &[(Effect::ResourceRatio(ResourceId::Charm), 0.20)],
    },
    RuleDef {
        name: "Quiet Contemplation",
        blurb: "Stillness gathers its own heat. {Warmth} production +20%.",
        cost_charm: 15000.0,
        blocks: RuleId::LoudPurring,
        effects: &[(Effect::ResourceRatio(ResourceId::Warmth), 0.20)],
    },
];

// ---- Names ----

/// The names kittens arrive with: chosen by hand in one cozy register, sized
/// to a grown clowder, short enough to stand whole in the roster, no two
/// alike at a glance, and none the name of anything else in the game.
/// Saved names live on their kittens, so this table can grow or change
/// without renaming anyone. @spec SIM-KIT-028, SIM-KIT-029
pub static NAME_POOL: [&str; 401] = [
    "Biscuit",
    "Mochi",
    "Pixel",
    "Waffles",
    "Miso",
    "Clementine",
    "Tofu",
    "Pepper",
    "Marmalade",
    "Noodle",
    "Pretzel",
    "Juniper",
    "Bean",
    "Dumpling",
    "Ziggy",
    "Olive",
    "Paprika",
    "Maple",
    "Sprocket",
    "Turnip",
    "Gnocchi",
    "Butterscotch",
    "Fig",
    "Pancake",
    "Nimbus",
    "Tangerine",
    "Pickles",
    "Comet",
    "Bramble",
    "Cricket",
    "Fennel",
    "Sesame",
    "Plum",
    "Wasabi",
    "Crumpet",
    "Nutmeg",
    "Socks",
    "Clover",
    "Captain Whiskers",
    "Muffin",
    "Scone",
    "Brioche",
    "Bagel",
    "Pudding",
    "Custard",
    "Toffee",
    "Fudge",
    "Caramel",
    "Truffle",
    "Cupcake",
    "Cookie",
    "Macaron",
    "Meringue",
    "Strudel",
    "Cobbler",
    "Crouton",
    "Nugget",
    "Ravioli",
    "Tortellini",
    "Pierogi",
    "Gyoza",
    "Wonton",
    "Bao",
    "Onigiri",
    "Tempura",
    "Udon",
    "Ramen",
    "Soba",
    "Kimchi",
    "Tamale",
    "Churro",
    "Nacho",
    "Taco",
    "Burrito",
    "Empanada",
    "Falafel",
    "Hummus",
    "Pita",
    "Couscous",
    "Lentil",
    "Chickpea",
    "Edamame",
    "Jellybean",
    "Gumdrop",
    "Lollipop",
    "Marshmallow",
    "Sprinkles",
    "Licorice",
    "Nougat",
    "Praline",
    "Bonbon",
    "Brownie",
    "Snickerdoodle",
    "Gingersnap",
    "Shortbread",
    "Biscotti",
    "Cannoli",
    "Tiramisu",
    "Baklava",
    "Latte",
    "Chai",
    "Cocoa",
    "Espresso",
    "Cappuccino",
    "Matcha",
    "Oolong",
    "Earl Grey",
    "Rooibos",
    "Cider",
    "Lemonade",
    "Sarsaparilla",
    "Apricot",
    "Peaches",
    "Cherry",
    "Kiwi",
    "Mango",
    "Papaya",
    "Guava",
    "Lychee",
    "Persimmon",
    "Quince",
    "Kumquat",
    "Damson",
    "Pomelo",
    "Satsuma",
    "Nectarine",
    "Huckleberry",
    "Gooseberry",
    "Blueberry",
    "Raspberry",
    "Cranberry",
    "Mulberry",
    "Currant",
    "Raisin",
    "Coconut",
    "Pineapple",
    "Banana",
    "Melon",
    "Pumpkin",
    "Squash",
    "Zucchini",
    "Radish",
    "Parsnip",
    "Rutabaga",
    "Artichoke",
    "Potato",
    "Sprout",
    "Leek",
    "Shallot",
    "Scallion",
    "Okra",
    "Yam",
    "Beet",
    "Rhubarb",
    "Celery",
    "Kohlrabi",
    "Cabbage",
    "Broccoli",
    "Peapod",
    "Spud",
    "Tater Tot",
    "Cinnamon",
    "Cardamom",
    "Saffron",
    "Ginger",
    "Anise",
    "Sage",
    "Thyme",
    "Basil",
    "Rosemary",
    "Oregano",
    "Parsley",
    "Chive",
    "Dill",
    "Tarragon",
    "Cumin",
    "Turmeric",
    "Sumac",
    "Allspice",
    "Vanilla",
    "Chili",
    "Jalapeno",
    "Chipotle",
    "Tahini",
    "Pesto",
    "Relish",
    "Chutney",
    "Salsa",
    "Walnut",
    "Pecan",
    "Cashew",
    "Pistachio",
    "Almond",
    "Acorn",
    "Chestnut",
    "Peanut",
    "Macadamia",
    "Oatmeal",
    "Barley",
    "Granola",
    "Muesli",
    "Popcorn",
    "Brie",
    "Gouda",
    "Feta",
    "Ricotta",
    "Halloumi",
    "Paneer",
    "Cheddar",
    "Camembert",
    "Gruyere",
    "Mozzarella",
    "Parmesan",
    "Sourdough",
    "Focaccia",
    "Ciabatta",
    "Baguette",
    "Croissant",
    "Pumpernickel",
    "Challah",
    "Naan",
    "Tortilla",
    "Daisy",
    "Poppy",
    "Tulip",
    "Violet",
    "Lilac",
    "Lavender",
    "Marigold",
    "Petunia",
    "Peony",
    "Dahlia",
    "Zinnia",
    "Begonia",
    "Azalea",
    "Camellia",
    "Magnolia",
    "Wisteria",
    "Hyacinth",
    "Primrose",
    "Snapdragon",
    "Foxglove",
    "Buttercup",
    "Dandelion",
    "Thistle",
    "Fern",
    "Moss",
    "Ivy",
    "Willow",
    "Birch",
    "Aspen",
    "Cedar",
    "Rowan",
    "Linden",
    "Acacia",
    "Cypress",
    "Bamboo",
    "Cactus",
    "Aloe",
    "Sorrel",
    "Yarrow",
    "Bracken",
    "Heather",
    "Lupin",
    "Larkspur",
    "Holly",
    "Mistletoe",
    "Hawthorn",
    "Sycamore",
    "Chamomile",
    "Hibiscus",
    "Jasmine",
    "Lotus",
    "Iris",
    "Freesia",
    "Gardenia",
    "Sunflower",
    "Snowdrop",
    "Crocus",
    "Daffodil",
    "Cornflower",
    "Sweet Pea",
    "Drizzle",
    "Breeze",
    "Thunder",
    "Misty",
    "Puddle",
    "Cumulus",
    "Cirrus",
    "Zephyr",
    "Flurry",
    "Blizzard",
    "Snowflake",
    "Frost",
    "Sunny",
    "Aurora",
    "Twilight",
    "Dusk",
    "Galaxy",
    "Nebula",
    "Orbit",
    "Meteor",
    "Eclipse",
    "Solstice",
    "Equinox",
    "Halo",
    "Cosmo",
    "Pluto",
    "Jupiter",
    "Luna",
    "Stella",
    "Teacup",
    "Thimble",
    "Mitten",
    "Slipper",
    "Teapot",
    "Kettle",
    "Ladle",
    "Whisk",
    "Spatula",
    "Doily",
    "Tassel",
    "Pompom",
    "Spool",
    "Bobbin",
    "Tinsel",
    "Confetti",
    "Velvet",
    "Corduroy",
    "Gingham",
    "Tweed",
    "Flannel",
    "Calico",
    "Tabby",
    "Tuxedo",
    "Patches",
    "Smudge",
    "Speckles",
    "Freckles",
    "Boots",
    "Scamper",
    "Pippin",
    "Tumble",
    "Wiggles",
    "Waddles",
    "Squiggle",
    "Jitterbug",
    "Fidget",
    "Gizmo",
    "Kerfuffle",
    "Hullabaloo",
    "Brouhaha",
    "Lollygag",
    "Skedaddle",
    "Razzle",
    "Jinx",
    "Mischief",
    "Rascal",
    "Scallywag",
    "Rapscallion",
    "Ragamuffin",
    "Nibbles",
    "Snuggles",
    "Cuddles",
    "Twinkle",
    "Glimmer",
    "Shimmer",
    "Flicker",
    "Wobble",
    "Tuppence",
    "Sixpence",
    "Rusty",
    "Felix",
    "Otis",
    "Rufus",
    "Archie",
    "Winnie",
    "Mabel",
    "Hazel",
    "Ruby",
    "Opal",
    "Pearl",
    "Agnes",
    "Gus",
    "Percy",
    "Rosie",
    "Lulu",
    "Gigi",
    "Momo",
    "Bubbles",
    "Smokey",
    "Shadow",
    "Midnight",
    "Sir Reginald",
    "Professor Pip",
    "Lord Fuzzington",
    "Uncle Barnaby",
    "Madame Bijou",
    "Lady Fairweather",
    "Countess Crumb",
    "Baron Snoozly",
];

// ---- Profiles ----

/// A fur pattern: its words, marking with `{k}` where slot k's colour name
/// goes, and its slots, each listing the colours it may take — only colours a
/// real cat could wear there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FurPatternDef {
    pub words: &'static str,
    pub slots: &'static [&'static [FurColour]],
}

/// Each pattern, in `FurPattern` order. @spec SIM-KIT-035
static FUR_PATTERNS: [FurPatternDef; 9] = {
    use FurColour::*;
    [
        // Solid
        FurPatternDef {
            words: "solid {0}",
            slots: &[&[
                Black, White, Grey, Cream, Ginger, Brown, Chocolate, Lilac, Cinnamon, Fawn,
            ]],
        },
        // Tabby: a base and its stripes.
        FurPatternDef {
            words: "{0} tabby",
            slots: &[
                &[Brown, Silver, Ginger, Cream, Grey],
                &[Black, Chocolate, Cinnamon, Ginger, Grey],
            ],
        },
        // Tuxedo: a dark coat and a white muzzle.
        FurPatternDef {
            words: "{0} tuxedo",
            slots: &[&[Black, Grey, Chocolate], &[White]],
        },
        // Bicolour: a colour and a white blaze and chin.
        FurPatternDef {
            words: "{0} and white",
            slots: &[&[Black, Grey, Ginger, Cream, Chocolate, Lilac], &[White]],
        },
        // Tortoiseshell: a dark colour patched with a warm one.
        FurPatternDef {
            words: "tortoiseshell",
            slots: &[&[Black, Chocolate, Grey, Lilac], &[Ginger, Cream]],
        },
        // Calico: white, patched warm and dark.
        FurPatternDef {
            words: "calico",
            slots: &[&[White], &[Ginger, Cream], &[Black, Grey]],
        },
        // Colourpoint: a pale body with dark points.
        FurPatternDef {
            words: "{1} point",
            slots: &[
                &[Cream, White, Fawn],
                &[Seal, Grey, Chocolate, Lilac, Ginger],
            ],
        },
        // Van: white, with colour on the head.
        FurPatternDef {
            words: "{1} and white van",
            slots: &[&[White], &[Ginger, Black, Cream, Grey]],
        },
        // Smoke: dark tips over a pale undercoat.
        FurPatternDef {
            words: "{0} smoke",
            slots: &[&[Black, Grey, Chocolate], &[Silver, White]],
        },
    ]
};

/// A pattern's definition. @spec SIM-KIT-035
pub fn fur_pattern_def(p: FurPattern) -> &'static FurPatternDef {
    &FUR_PATTERNS[p as usize]
}

/// A fur colour's name, as a fur's words say it. @spec SIM-KIT-039
pub fn fur_colour_name(c: FurColour) -> &'static str {
    match c {
        FurColour::Black => "black",
        FurColour::White => "white",
        FurColour::Grey => "grey",
        FurColour::Silver => "silver",
        FurColour::Brown => "brown",
        FurColour::Chocolate => "chocolate",
        FurColour::Ginger => "ginger",
        FurColour::Cream => "cream",
        FurColour::Cinnamon => "cinnamon",
        FurColour::Fawn => "fawn",
        FurColour::Seal => "seal",
        FurColour::Lilac => "lilac",
        FurColour::Lavender => "lavender",
        FurColour::Periwinkle => "periwinkle",
        FurColour::Seafoam => "seafoam",
        FurColour::Rose => "rose",
        FurColour::Marigold => "marigold",
        FurColour::Plum => "plum",
        FurColour::Teal => "teal",
        FurColour::Cornflower => "cornflower",
    }
}

/// The colours a surprising kitten wears, none of them a colour any slot
/// lists. @spec SIM-KIT-042
pub static SURPRISING_COLOURS: [FurColour; 8] = [
    FurColour::Lavender,
    FurColour::Periwinkle,
    FurColour::Seafoam,
    FurColour::Rose,
    FurColour::Marigold,
    FurColour::Plum,
    FurColour::Teal,
    FurColour::Cornflower,
];

/// One kitten in this many is a surprise. @spec SIM-KIT-032
pub const SURPRISE_ODDS: usize = 40;

/// An eye look: each eye's colour, and the words for the look.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EyeLookDef {
    pub left: EyeColour,
    pub right: EyeColour,
    pub words: &'static str,
}

/// An eye look whose two eyes are one colour.
const fn both(c: EyeColour, words: &'static str) -> EyeLookDef {
    EyeLookDef {
        left: c,
        right: c,
        words,
    }
}

/// Each look, in `EyeLook` order. @spec SIM-KIT-036
static EYE_LOOKS: [EyeLookDef; 8] = {
    use EyeColour::*;
    [
        both(Green, "green eyes"),
        both(Gold, "golden eyes"),
        both(Blue, "blue eyes"),
        both(Amber, "amber eyes"),
        both(Copper, "copper eyes"),
        both(Hazel, "hazel eyes"),
        EyeLookDef {
            left: Blue,
            right: Copper,
            words: "odd eyes, blue and copper",
        },
        EyeLookDef {
            left: Blue,
            right: Green,
            words: "odd eyes, blue and green",
        },
    ]
};

/// A look's definition. @spec SIM-KIT-036
pub fn eye_look_def(l: EyeLook) -> &'static EyeLookDef {
    &EYE_LOOKS[l as usize]
}

/// A favourite kind: the label that introduces it on a profile, and its
/// entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FavouriteKindDef {
    pub label: &'static str,
    pub entries: &'static [&'static str],
}

static FOODS: [&str; 32] = [
    "tuna, but only on Tuesdays",
    "the crust off a slice of toast",
    "a single cold green pea",
    "whatever you are eating right now",
    "warm milk foam from the top of a cup",
    "sardines, eaten with great ceremony",
    "the last bite of a pancake",
    "chicken, shredded very small",
    "a lick of butter, stolen",
    "salmon skin, crispy",
    "plain yogurt off a spoon",
    "melon, for reasons unknown",
    "cheese rind left on a plate",
    "scrambled egg, a little cold",
    "the gravy and not the meat",
    "olives, sniffed but never eaten",
    "shrimp tails from the bin",
    "a pinch of roast turkey",
    "cantaloupe, inexplicably",
    "dry kibble, one at a time",
    "the water from the tuna tin",
    "a crumb of blueberry muffin",
    "corn kernels batted off the table",
    "pumpkin mash",
    "mackerel by the window",
    "a strand of spaghetti, slurped",
    "cream cheese on a fingertip",
    "ham, offered by a soft touch",
    "the bacon nobody was watching",
    "rotisserie chicken skin",
    "a nibble of cucumber, bravely",
    "broth, warmed, in a saucer",
];

static TOYS: [&str; 29] = [
    "a crinkly paper bag",
    "the cap from a milk jug",
    "a hair tie, stolen nightly",
    "a sock with no partner",
    "a cork from a long-ago party",
    "a crumpled receipt",
    "a ping-pong ball",
    "a toy mouse with one ear",
    "the drawstring of a hoodie",
    "a shoelace, still in its shoe",
    "an ice cube skating on the floor",
    "a rubber band",
    "a bottle cap under the fridge",
    "the end of a measuring tape",
    "the ribbon off a birthday present",
    "a pom-pom the size of a grape",
    "a crinkle ball",
    "a wine cork, much loved",
    "a pencil, pushed off the desk",
    "the tassel on a curtain",
    "a paper straw wrapper",
    "a sprig of dried lavender",
    "a balled-up piece of foil",
    "the corner of a rug",
    "a houseplant's lowest leaf",
    "a dangling earring",
    "a stick from the garden",
    "a bit of string, any bit",
    "the pull cord of a lamp",
];

static NAP_SPOTS: [&str; 28] = [
    "the top of the fridge",
    "a laundry basket, fresh from the dryer",
    "the exact middle of the bed",
    "a patch of light on the rug",
    "on the keyboard, mid-sentence",
    "a shoebox one size too small",
    "the bathroom sink",
    "behind the curtains",
    "on a pile of fresh towels",
    "the bottom stair",
    "a radiator shelf",
    "on top of the warm router",
    "in an open suitcase",
    "on whoever is reading",
    "the bookshelf, between two novels",
    "a flowerpot with no flower",
    "the back of the sofa",
    "under the duvet, toes out",
    "the windowsill above the sink",
    "in a paper bag by the door",
    "the passenger seat",
    "a drawer left open an inch",
    "on a freshly folded sweater",
    "the doormat, for the draughts",
    "the warm spot where you just sat",
    "on top of the piano",
    "inside the bathtub, dry",
    "a box by the heater",
];

static SHOWS: [&str; 28] = [
    "rain racing down the window",
    "the washing machine's spin",
    "pigeons on the wire",
    "a fly that cannot be caught",
    "the fish tank, very closely",
    "snow falling past the lamp",
    "the dripping kitchen tap",
    "squirrels holding a meeting",
    "you, while you sleep",
    "the ceiling fan going round",
    "soap bubbles drifting by",
    "the neighbour's dog, through a gap",
    "leaves skittering down the path",
    "a spider building its web",
    "the toaster, waiting for it",
    "cooking shows, from the counter",
    "moths around the lamp",
    "the screensaver",
    "a candle flame, from far away",
    "the post arriving",
    "nature films about birds",
    "the bathwater running",
    "the robot vacuum, with suspicion",
    "clouds sliding over the roof",
    "the shadow of a passing bird",
    "the cursor on your screen",
    "fireflies at dusk",
    "the goldfish next door",
];

static PASTIMES: [&str; 28] = [
    "sitting in boxes",
    "knocking things off shelves",
    "chirping at birds",
    "kneading the blankets",
    "chasing its own tail",
    "hiding under the bed",
    "sprinting about at three in the morning",
    "loafing",
    "sitting on important papers",
    "grooming a friend",
    "carrying socks around the house",
    "climbing the curtains",
    "batting at dangling things",
    "following you to the bathroom",
    "drinking from the tap",
    "staring into empty corners",
    "making biscuits on laps",
    "rolling in the dust",
    "stealing your seat",
    "pretending not to care",
    "playing fetch, rarely",
    "talking back",
    "headbutting chins",
    "supervising dinner",
    "sitting in the sink",
    "squeezing into small places",
    "tucking its paws under",
    "lying in the sun",
];

static QUIRKS: [&str; 28] = [
    "sneezes when happy",
    "sleeps with its tongue out",
    "only drinks from a glass",
    "trills instead of meowing",
    "sits like a little loaf",
    "has one crooked whisker",
    "blinks slowly at strangers",
    "carries its food to another room",
    "tucks its nose under its tail",
    "chatters at the window",
    "sleeps on its back, belly up",
    "is wary of the fridge humming",
    "taps you twice for attention",
    "walks across the piano at night",
    "has one white toe",
    "purrs in its sleep",
    "headbutts doorframes",
    "comes running at the sound of a zip",
    "kicks its water bowl first",
    "always lands on its feet, smugly",
    "sits in any square on the floor",
    "stares at walls, knowingly",
    "naps in a perfect circle",
    "yawns very loudly",
    "has a meow like a squeaky hinge",
    "pats the water before drinking",
    "drools when petted",
    "sleeps with one eye open",
];

static SECRET_TALENTS: [&str; 18] = [
    "opening the treat cupboard",
    "turning off the alarm clock",
    "fitting through any gap",
    "knowing when you'll be home",
    "herding the other kittens",
    "stealing socks undetected",
    "purring loud enough to rattle glasses",
    "sleeping through thunder",
    "finding the one warm spot",
    "opening doors by leaping on handles",
    "hearing a can open three rooms away",
    "walking along very thin railings",
    "sitting perfectly still for an hour",
    "untying shoelaces",
    "climbing down, eventually",
    "telling the time by dinner",
    "fetching hair ties, on command",
    "looking innocent",
];

static FEARS: [&str; 18] = [
    "the vacuum cleaner",
    "cucumbers left on the floor",
    "its own reflection, some days",
    "the doorbell",
    "plastic bags that move by themselves",
    "the hairdryer",
    "anything that pops",
    "the sound of tinfoil",
    "sneezes, especially its own",
    "the bath, obviously",
    "hats",
    "the blender",
    "umbrellas opening indoors",
    "the dark under the stairs",
    "thunder, a little",
    "the neighbour's very large dog",
    "being picked up the wrong way",
    "absolutely nothing, it claims",
];

static DAYDREAMS: [&str; 17] = [
    "an endless roll of toilet paper",
    "a bird that just sits there",
    "a box big enough for everyone",
    "a sunny windowsill, forever",
    "a fish the size of a sofa",
    "being taller than the fridge",
    "a lap that never stands up",
    "flying, like the pigeons",
    "a room full of crinkly bags",
    "a bowl that refills itself",
    "catching the moon in the pond",
    "a hammock in the sun",
    "the last slice of ham, all to itself",
    "a castle made of cushions",
    "chasing a leaf that never lands",
    "tummy rubs with no trap",
    "a nap that lasts all week",
];

static SAYINGS: [&str; 16] = [
    "mrrp?",
    "a small squeak, like a hinge",
    "nothing, but very loudly",
    "mew, then thinks better of it",
    "mrow, with feeling",
    "brrrt!",
    "a long complaint about dinner",
    "meow, only when you're on the phone",
    "eek",
    "chirrup?",
    "a polite little mip",
    "one meow per stair",
    "wow, apparently",
    "the same thing as you, but later",
    "hello, every single morning",
    "mmm-rrrah",
];

static COLLECTIONS: [&str; 17] = [
    "bottle caps",
    "hair ties from all over the house",
    "leaves, brought in one at a time",
    "single socks",
    "pen lids",
    "crumpled receipts",
    "bits of string",
    "things that roll under the sofa",
    "shoelaces, detached",
    "ribbons from presents",
    "one earring of every pair",
    "crayons",
    "twist ties from the bread bags",
    "acorns from the garden",
    "corks",
    "paper clips",
    "anything that glints",
];

static PET_PEEVES: [&str; 16] = [
    "a closed door, any door",
    "an empty patch in the food bowl",
    "being looked at while eating",
    "wet paws",
    "the alarm clock",
    "when you move during a nap",
    "a bath, or the idea of one",
    "cold floors in the morning",
    "being ignored for a phone",
    "a new brand of litter",
    "hiccups",
    "the lid on the fish tank",
    "someone else in its chair",
    "the bottom of the bowl showing",
    "rain, personally",
    "sudden music",
];

static SMELLS: [&str; 17] = [
    "fresh laundry",
    "your shoes, unfortunately",
    "rain on hot stones",
    "toast",
    "cut grass",
    "the inside of a paper bag",
    "lavender in the garden",
    "the cardboard of a new delivery",
    "warm radiators",
    "pine needles",
    "old books",
    "a freshly opened tin",
    "your pillow",
    "the dog next door, from a distance",
    "wet earth after a storm",
    "coffee, from across the room",
    "oranges, which it hates loving",
];

static SIGNATURE_MOVES: [&str; 17] = [
    "the slow blink",
    "the loaf",
    "the sudden sprint",
    "the flop and roll",
    "the headbutt",
    "the paw on your cheek",
    "the tail flick of disapproval",
    "the full-body stretch",
    "the shoulder perch",
    "the silent stare",
    "the bread-making knead",
    "the doorway sit",
    "the one-paw wave",
    "the sideways hop",
    "the tummy trap",
    "the yawn and squeak",
    "the knock-it-off-the-table",
];

/// Every favourite kind, in `FavouriteKind` order: its label and its
/// entries. @spec SIM-KIT-037, SIM-KIT-038
static FAVOURITE_KINDS: [FavouriteKindDef; 14] = [
    FavouriteKindDef {
        label: "Favourite food",
        entries: &FOODS,
    },
    FavouriteKindDef {
        label: "Favourite toy",
        entries: &TOYS,
    },
    FavouriteKindDef {
        label: "Best nap spot",
        entries: &NAP_SPOTS,
    },
    FavouriteKindDef {
        label: "Loves watching",
        entries: &SHOWS,
    },
    FavouriteKindDef {
        label: "Pastime",
        entries: &PASTIMES,
    },
    FavouriteKindDef {
        label: "Quirk",
        entries: &QUIRKS,
    },
    FavouriteKindDef {
        label: "Secret talent",
        entries: &SECRET_TALENTS,
    },
    FavouriteKindDef {
        label: "Afraid of",
        entries: &FEARS,
    },
    FavouriteKindDef {
        label: "Daydreams about",
        entries: &DAYDREAMS,
    },
    FavouriteKindDef {
        label: "Often says",
        entries: &SAYINGS,
    },
    FavouriteKindDef {
        label: "Collects",
        entries: &COLLECTIONS,
    },
    FavouriteKindDef {
        label: "Pet peeve",
        entries: &PET_PEEVES,
    },
    FavouriteKindDef {
        label: "Favourite smell",
        entries: &SMELLS,
    },
    FavouriteKindDef {
        label: "Signature move",
        entries: &SIGNATURE_MOVES,
    },
];

/// A kind's definition. @spec SIM-KIT-037, SIM-KIT-038
pub fn favourite_kind_def(k: FavouriteKind) -> &'static FavouriteKindDef {
    &FAVOURITE_KINDS[k as usize]
}

/// A trait as the player reads it: its name and its blurb.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraitDef {
    pub name: &'static str,
    pub blurb: &'static str,
}

/// Each trait's name and blurb, in `KittenTrait` order. The figures match
/// the Elder trait effects exactly (a content test holds them). @spec SIM-KIT-040
static TRAITS: [TraitDef; 8] = [
    TraitDef {
        name: "Easygoing",
        blurb: "As Elder: nothing changes. Some cats simply are.",
    },
    TraitDef {
        name: "Crafty",
        blurb: "As Elder: every craft +5%.",
    },
    TraitDef {
        name: "Brassy",
        blurb: "As Elder: {badges|Badges}, {brass|Brass}, {cogs|Cogs}, and {moonsteel|Moonsteel} craft +10%.",
    },
    TraitDef {
        name: "Alchemical",
        blurb: "As Elder: {pavers|Pavers}, {moonmilk|Moonmilk}, {dreamweave|Dreamweave}, and {cometsilk|Cometsilk} craft +7.5%.",
    },
    TraitDef {
        name: "Charming",
        blurb: "As Elder: trade ratio +3%.",
    },
    TraitDef {
        name: "Bold",
        blurb: "As Elder: pounce ratio +0.5.",
    },
    TraitDef {
        name: "Curious",
        blurb: "As Elder: ideas cost 5% less {curiosity|Curiosity}.",
    },
    TraitDef {
        name: "Serene",
        blurb: "As Elder: rites cost 9% less.",
    },
];

/// A trait's definition. @spec SIM-KIT-040
pub fn trait_def(t: KittenTrait) -> &'static TraitDef {
    &TRAITS[t as usize]
}

// ---- Achievements ----

/// An achievement as the player reads it: what it is called, the nudge shown
/// while it is locked, what happened once it is earned, and whether it stays
/// hidden until then. Words mark the resources they name as blurbs do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AchievementDef {
    pub name: &'static str,
    pub hint: &'static str,
    pub words: &'static str,
    pub secret: bool,
}

/// Every achievement, in `AchievementId` order. @spec SIM-ACH-001
static ACHIEVEMENTS: [AchievementDef; 61] = [
    AchievementDef {
        name: "A Face at the Door",
        hint: "Leave the light on. Someone's bound to wander by.",
        words: "The first kitten came in out of the weather and decided to stay.",
        secret: false,
    },
    AchievementDef {
        name: "A Proper Clowder",
        hint: "One kitten is a pet. A few more is a household.",
        words: "Ten kittens! That's officially a clowder.",
        secret: false,
    },
    AchievementDef {
        name: "Wall-to-Wall Whiskers",
        hint: "Room for a few more? There's always room for a few more.",
        words: "Fifty kittens. Every sunbeam has a tenant.",
        secret: false,
    },
    AchievementDef {
        name: "Too Many to Name (Almost)",
        hint: "Keep going. The names will hold out. Probably.",
        words: "A hundred kittens, and every one of them still answers to their name. Sometimes.",
        secret: false,
    },
    AchievementDef {
        name: "Full House",
        hint: "Every bed, every basket, every lap.",
        words: "Every bed in the house is taken, and two of them are taken twice.",
        secret: false,
    },
    AchievementDef {
        name: "Once in a Blue Moon",
        hint: "Some kittens aren't like the others.",
        words:
            "A kitten arrived wearing a colour no ordinary cat comes in. Nobody's asking questions.",
        secret: true,
    },
    AchievementDef {
        name: "Odd One Out",
        hint: "Look them in the eyes. Both of them.",
        words: "A kitten with odd eyes: one of each, and twice the charm.",
        secret: false,
    },
    AchievementDef {
        name: "Strike a Pose",
        hint: "Every kitten has a favourite way to sit. Find them all.",
        words: "Every pose there is, somewhere in the clowder.",
        secret: false,
    },
    AchievementDef {
        name: "A Coat of Many Colours",
        hint: "Stripes, patches, socks, and more. Pat them all.",
        words: "A kitten in every pattern a coat can come in.",
        secret: false,
    },
    AchievementDef {
        name: "Snack Buddies",
        hint: "Two kittens, one plate.",
        words: "Two kittens love the very same food. They've agreed to share. Mostly.",
        secret: true,
    },
    AchievementDef {
        name: "Matching Outfits",
        hint: "Did you two plan this?",
        words:
            "Two kittens turned up in exactly the same coat. It's a little awkward for everyone.",
        secret: true,
    },
    AchievementDef {
        name: "Seconds, Please",
        hint: "When every name is taken, borrow one.",
        words: "The names ran out, so a kitten got a numeral after theirs. Very regal.",
        secret: true,
    },
    AchievementDef {
        name: "Wise Whiskers",
        hint: "Every clowder needs someone to look up to.",
        words: "An Elder keeps watch over the clowder now, mostly with their eyes closed.",
        secret: false,
    },
    AchievementDef {
        name: "Moving Up",
        hint: "Hard work deserves a little recognition.",
        words: "A kitten's first promotion. They've been insufferable about it ever since.",
        secret: false,
    },
    AchievementDef {
        name: "Senior Purrsonnel",
        hint: "Climb a little higher. Then higher still.",
        words: "A kitten has risen to the fifth rank, and been given a very important cushion.",
        secret: false,
    },
    AchievementDef {
        name: "Paws of Experience",
        hint: "Practice makes purrfect. Lots and lots of practice.",
        words: "A kitten knows their work so well they can do it asleep, and sometimes do.",
        secret: false,
    },
    AchievementDef {
        name: "Everyone's on Break",
        hint: "Sometimes the best work is no work at all.",
        words: "Ten kittens at home and not one of them working. Bliss.",
        secret: true,
    },
    AchievementDef {
        name: "Visiting the Neighbours",
        hint: "The grass is always greener on the other side of the fence.",
        words: "A kitten slipped off to see what the neighbours were having for dinner.",
        secret: false,
    },
    AchievementDef {
        name: "Home Again",
        hint: "Everyone comes home eventually.",
        words: "A wanderer came home, smelling faintly of someone else's dinner.",
        secret: false,
    },
    AchievementDef {
        name: "Pure Bliss",
        hint: "Happy kittens, happier kittens, happiest kittens.",
        words: "The whole clowder is blissful. You can hear the purring from the road.",
        secret: false,
    },
    AchievementDef {
        name: "Hangry",
        hint: "Not every day is a good day.",
        words: "The clowder is grumpy. Someone knocked a cup off the table, on purpose.",
        secret: true,
    },
    AchievementDef {
        name: "The Zoomies!",
        hint: "Three in the morning is a perfectly good time to run laps.",
        words: "The zoomies! Everybody's running and nobody knows why.",
        secret: false,
    },
    AchievementDef {
        name: "Party Animals",
        hint: "What this place needs is a party.",
        words: "A jamboree! Streamers, snacks, and a conga line of tails.",
        secret: false,
    },
    AchievementDef {
        name: "Home Improvement",
        hint: "A meadow is nice. A meadow with things in it is nicer.",
        words: "The first thing built in the meadow. It's lopsided, and it's perfect.",
        secret: false,
    },
    AchievementDef {
        name: "Sprawl",
        hint: "Build. Then build some more.",
        words: "A hundred buildings dot the meadow. It's practically a village.",
        secret: false,
    },
    AchievementDef {
        name: "Kitten Metropolis",
        hint: "Go big. Bigger than that.",
        words: "Five hundred buildings. There's talk of electing a mayor.",
        secret: false,
    },
    AchievementDef {
        name: "Handmade",
        hint: "Some things are better made by paw.",
        words: "Something lovingly crafted by kitten paws. Only slightly chewed.",
        secret: false,
    },
    AchievementDef {
        name: "Busy Paws",
        hint: "Idle paws find projects of their own.",
        words: "A kitten has taken up a craft of their own, and will not be told how to do it.",
        secret: false,
    },
    AchievementDef {
        name: "What a Mess",
        hint: "A lived-in meadow is a happy meadow. Up to a point.",
        words: "The meadow is officially a mess. Nobody admits to anything.",
        secret: false,
    },
    AchievementDef {
        name: "Lights Flicker",
        hint: "Everything wants plugging in at once.",
        words:
            "The meadow is using more energy than it makes. Someone left the yarn ball spinning.",
        secret: true,
    },
    AchievementDef {
        name: "Humming Along",
        hint: "Keep the lights on and the wheels turning.",
        words: "Plenty of energy, and some to spare. The whole meadow hums.",
        secret: false,
    },
    AchievementDef {
        name: "A Bright Idea",
        hint: "Curiosity never hurt a cat.",
        words: "The clowder's first idea. They're very proud of it.",
        secret: false,
    },
    AchievementDef {
        name: "Well Read",
        hint: "One idea leads to another, and another.",
        words: "Twenty ideas! The library is mostly cushions, but still.",
        secret: false,
    },
    AchievementDef {
        name: "Know-It-All",
        hint: "Learn everything. Then nap about it.",
        words: "Every idea there is. The clowder is now unbearable at parties.",
        secret: false,
    },
    AchievementDef {
        name: "Refined Tastes",
        hint: "Little improvements add up.",
        words: "Ten refinements. Everything's just that bit nicer around here.",
        secret: false,
    },
    AchievementDef {
        name: "House Rules",
        hint: "Every household has its rules.",
        words: "A house rule, adopted. It will be followed whenever convenient.",
        secret: false,
    },
    AchievementDef {
        name: "Pounce!",
        hint: "Wiggle, wiggle, wiggle... go!",
        words: "A pounce came home with treasures in its mouth.",
        secret: false,
    },
    AchievementDef {
        name: "The Great Hunt (for String)",
        hint: "Strength in numbers, out in the long grass.",
        words: "Fifty squads out at once. Not a piece of string in the meadow is safe.",
        secret: true,
    },
    AchievementDef {
        name: "Followed Home",
        hint: "Something magical might follow you home.",
        words: "A {unicorn|Unicorns} followed a kitten home. Can we keep it?",
        secret: false,
    },
    AchievementDef {
        name: "Open for Business",
        hint: "The neighbours have things. You have things.",
        words: "The first trade came home. Everyone's a merchant now.",
        secret: false,
    },
    AchievementDef {
        name: "Shh, They're Sleeping",
        hint: "Knock quietly. Some neighbours sleep in.",
        words: "A caravan arrived to find the neighbours napping, and tiptoed home again.",
        secret: true,
    },
    AchievementDef {
        name: "Dear Neighbour",
        hint: "Write to someone. They might write back.",
        words: "A neighbour has a pen pal now. The letters are mostly paw prints.",
        secret: false,
    },
    AchievementDef {
        name: "Best Friends Forever",
        hint: "Keep writing. Friendship takes time.",
        words: "Ten letters to the same neighbour. You're practically family.",
        secret: false,
    },
    AchievementDef {
        name: "Know the Neighbourhood",
        hint: "Somebody lives past every hedge.",
        words: "Every neighbour found. The clowder knows everyone on the block.",
        secret: false,
    },
    AchievementDef {
        name: "Sunbather",
        hint: "Find a warm spot and stay there.",
        words: "The first sunbeam rite. Warm, golden, and very, very sleepy.",
        secret: false,
    },
    AchievementDef {
        name: "Enlightened",
        hint: "Bask long enough and you might see something.",
        words: "The clowder reached enlightenment. It looked a lot like a nap.",
        secret: false,
    },
    AchievementDef {
        name: "Glitter Parade",
        hint: "Some days call for glitter.",
        words: "A parade! Rainbows everywhere, and glitter in places glitter shouldn't be.",
        secret: false,
    },
    AchievementDef {
        name: "Sweet Dreams",
        hint: "Dream a little dream.",
        words: "A {dreamcorn|Dreamcorns} wandered out of a dream and into the meadow.",
        secret: false,
    },
    AchievementDef {
        name: "Bottled Contentment",
        hint: "Some feelings are worth keeping.",
        words: "Contentment, bottled. A {jar of purrs|JarsOfPurrs} hums on the shelf.",
        secret: false,
    },
    AchievementDef {
        name: "Over the Fence",
        hint: "There's a whole world past the garden wall.",
        words: "The clowder reached somewhere new, over the fence and beyond.",
        secret: false,
    },
    AchievementDef {
        name: "Wish Upon a Star",
        hint: "Go as far as far goes.",
        words: "The furthest place there is. Everyone made a wish while they were there.",
        secret: false,
    },
    AchievementDef {
        name: "Whale Watching",
        hint: "Keep an eye on the sky.",
        words: "A Star Whale is visiting. Everyone's trying very hard not to stare.",
        secret: false,
    },
    AchievementDef {
        name: "A Very Long Nap",
        hint: "Sometimes a short nap isn't enough.",
        words: "The clowder woke from a nap that lasted years. Nobody's in a hurry to get up.",
        secret: false,
    },
    AchievementDef {
        name: "Good Morning",
        hint: "Every long night ends.",
        words: "The Distant Dawn has risen. Good morning, everyone.",
        secret: true,
    },
    AchievementDef {
        name: "Moving Day",
        hint: "Every meadow is a beginning.",
        words: "The clowder moved to a new meadow. The boxes were the best part.",
        secret: false,
    },
    AchievementDef {
        name: "Family Tree",
        hint: "Generations of whiskers.",
        words: "Five generations of the clowder. The family tree is mostly cats sitting in it.",
        secret: false,
    },
    AchievementDef {
        name: "Remembered",
        hint: "Nobody's forgotten.",
        words: "A hundred kittens remembered in the Family Album.",
        secret: false,
    },
    AchievementDef {
        name: "Off on an Adventure",
        hint: "Pack a snack. Go somewhere new.",
        words: "An adventure is under way! Somebody packed far too many snacks.",
        secret: false,
    },
    AchievementDef {
        name: "The Old Ways",
        hint: "Some things are worth passing down.",
        words: "A tradition, kept. The kittens of tomorrow will thank you with a slow blink.",
        secret: false,
    },
    AchievementDef {
        name: "A Whole Year",
        hint: "Spring, summer, autumn, winter, and round again.",
        words: "A whole year in the meadow. Happy anniversary!",
        secret: false,
    },
    AchievementDef {
        name: "Old Meadow",
        hint: "Stick around. A long, long time.",
        words: "A hundred years in the meadow. The oldest tree has a kitten-shaped dent.",
        secret: true,
    },
];

/// An achievement's definition. @spec SIM-ACH-001
pub fn achievement_def(a: AchievementId) -> &'static AchievementDef {
    &ACHIEVEMENTS[a as usize]
}

// ---- Flavor templates ----

pub struct FlavorTemplate {
    pub text: &'static str,
    pub needs_kitten: bool,
}

pub static FLAVOR_TEMPLATES: [FlavorTemplate; 10] = [
    FlavorTemplate {
        text: "{name} knocked a pencil off the table. It was necessary.",
        needs_kitten: true,
    },
    FlavorTemplate {
        text: "{name} is sitting in the fresh laundry. The laundry is warm now.",
        needs_kitten: true,
    },
    FlavorTemplate {
        text: "{name} stared at the wall for a while. It knows something.",
        needs_kitten: true,
    },
    FlavorTemplate {
        text: "{name} found the exact center of the sunniest spot and claimed it.",
        needs_kitten: true,
    },
    FlavorTemplate {
        text: "{name} chirped at a bird through the window. Negotiations continue.",
        needs_kitten: true,
    },
    FlavorTemplate {
        text: "{name} performed the ceremonial pre-nap stretch. Flawless form.",
        needs_kitten: true,
    },
    FlavorTemplate {
        text: "A sunbeam moved; the whole clowder relocated accordingly.",
        needs_kitten: false,
    },
    FlavorTemplate {
        text: "Somewhere, a cardboard box settled with a dignified whumpf.",
        needs_kitten: false,
    },
    FlavorTemplate {
        text: "A moth got in. The meadow has never been so alert.",
        needs_kitten: false,
    },
    FlavorTemplate {
        text: "It rained gently. The porch smelled like spring for an hour.",
        needs_kitten: false,
    },
];

// ---- Sunbeam Rites ----

use crate::state::{ArchId, ColonyId, RiteId};

/// A Sunbeam Rite: warmth+shinies (+charm) priced, unlocked once devotion's
/// high-water reached its gate. @spec SIM-SUN-005
pub struct RiteDef {
    pub name: &'static str,
    /// A short joke and the plain effect, each resource it names marked
    /// (`blurb_parts`).
    pub blurb: &'static str,
    pub cost: &'static [(ResourceId, f64)],
    pub devotion_gate: f64,
}

pub fn rite_def(r: RiteId) -> &'static RiteDef {
    &RITES[r as usize]
}

static RITES: [RiteDef; 11] = [
    RiteDef {
        name: "Morning Stretch",
        blurb: "A long, luxurious stretch toward the light. {warmth|Warmth} production +10%.",
        cost: &[(ResourceId::Warmth, 100.0)],
        devotion_gate: 150.0,
    },
    RiteDef {
        name: "Shared Sunpuddle",
        blurb: "Room in the puddle for everyone. {curiosity|Curiosity} cap +2% per Sunbeam Altar.",
        cost: &[(ResourceId::Warmth, 250.0), (ResourceId::Shinies, 150.0)],
        devotion_gate: 300.0,
    },
    RiteDef {
        name: "Golden Hour",
        blurb: "The light goes the colour of honey. {warmth|Warmth} cap ×1.5, and ×0.1 more for each Golden Hour after the first.",
        cost: &[(ResourceId::Warmth, 350.0), (ResourceId::Shinies, 250.0)],
        devotion_gate: 500.0,
    },
    RiteDef {
        name: "Warm Welcome",
        blurb: "Every cat gets a seat in the sun. +0.4 happiness per Sunbeam Altar.",
        cost: &[(ResourceId::Warmth, 500.0), (ResourceId::Shinies, 250.0)],
        devotion_gate: 750.0,
    },
    RiteDef {
        name: "Stained Sunlight",
        blurb: "Colour pools on the floor. Sunbeam Altars +0.05 {charm|Charm}/t each.",
        cost: &[(ResourceId::Warmth, 500.0), (ResourceId::Shinies, 250.0)],
        devotion_gate: 750.0,
    },
    RiteDef {
        name: "The Long Sun",
        blurb: "The afternoon that never quite ends. Turns on the Long Sun, a production bonus that grows with devotion.",
        cost: &[(ResourceId::Warmth, 750.0), (ResourceId::Shinies, 500.0)],
        devotion_gate: 1000.0,
    },
    RiteDef {
        name: "Grand Basking",
        blurb: "Everyone at once, in the warmest spot. Sunbeam Altars +60 {charm|Charm} cap and +0.1 {charm|Charm}/t each.",
        cost: &[(ResourceId::Warmth, 1250.0), (ResourceId::Shinies, 750.0)],
        devotion_gate: 10000.0,
    },
    RiteDef {
        name: "Sun Wardens' Oath",
        blurb: "The wardens promise to keep watch. +25 {pep|Pep} cap per Sunbeam Altar.",
        cost: &[(ResourceId::Warmth, 3500.0), (ResourceId::Shinies, 1500.0)],
        devotion_gate: 75000.0,
    },
    RiteDef {
        name: "The Reverie",
        blurb: "Eyes half closed, thoughts drifting. Opens Sunrise Reverie, which turns devotion into serenity.",
        cost: &[(ResourceId::Warmth, 5000.0), (ResourceId::Shinies, 1800.0)],
        devotion_gate: 100000.0,
    },
    RiteDef {
        name: "Enlightenment",
        blurb: "Everything is warm and nothing hurts. Opens the Enlightenment tiers, and lets the earlier rites be taken again, each time at a higher price.",
        cost: &[(ResourceId::Warmth, 7500.0), (ResourceId::Shinies, 2000.0)],
        devotion_gate: 125000.0,
    },
    RiteDef {
        name: "Everwarm Frescoes",
        blurb: "Murals that remember every summer. Every Sun Room gains {warmth|Warmth} cap from Warm Memories, more with each fresco.",
        cost: &[
            (ResourceId::Warmth, 4000.0),
            (ResourceId::Shinies, 1800.0),
            (ResourceId::Charm, 8000.0),
        ],
        devotion_gate: 200000.0,
    },
];

/// Rites 0–7 (except the single-purchase Long Sun at index 5) restack once the
/// Enlightenment rite is bought. @spec SIM-SUN-006
pub fn rite_restackable(r: RiteId) -> bool {
    matches!(
        r,
        RiteId::MorningStretch
            | RiteId::SharedSunpuddle
            | RiteId::GoldenHour
            | RiteId::WarmWelcome
            | RiteId::StainedSunlight
            | RiteId::GrandBasking
            | RiteId::SunWardensOath
    )
}
pub const RITE_RESTACK_RATIO: f64 = 2.5; // price ×2.5 per additional copy

// ---- Rainbow Arch upgrades ----

/// A Rainbow Arch upgrade: rainbow/prism/other priced, bought once each.
/// @spec SIM-UNI-005
pub struct ArchDef {
    pub name: &'static str,
    /// A short joke and the plain effect, each resource it names marked
    /// (`blurb_parts`).
    pub blurb: &'static str,
    pub cost: &'static [(ResourceId, f64)],
}

pub fn arch_def(a: ArchId) -> &'static ArchDef {
    &ARCHES[a as usize]
}

static ARCHES: [ArchDef; 8] = [
    ArchDef {
        name: "Ribbon Arch",
        blurb: "Streamers in every colour. {unicorns|Unicorns} production +5%.",
        cost: &[(ResourceId::Rainbows, 500.0)],
    },
    ArchDef {
        name: "Crystal Arch",
        blurb: "It hums when the light hits it. {unicorns|Unicorns} production +10%; {prism|Prisms} descent chance +0.05% a day.",
        cost: &[(ResourceId::Rainbows, 25000.0), (ResourceId::Prisms, 25.0)],
    },
    ArchDef {
        name: "Prism Arch",
        blurb: "It throws colour on every wall. {unicorns|Unicorns} production +25%; chance of a shower of {rainbows|Rainbows} +0.05% a day.",
        cost: &[(ResourceId::Rainbows, 50000.0), (ResourceId::Prisms, 50.0)],
    },
    ArchDef {
        name: "Sky Arch",
        blurb: "Tall enough to tickle the clouds. {unicorns|Unicorns} production +50%; {prisms|Prisms} drift down, +0.00002/t.",
        cost: &[
            (ResourceId::Rainbows, 125000.0),
            (ResourceId::Prisms, 500.0),
        ],
    },
    ArchDef {
        name: "Dreaming Arch",
        blurb: "{Unicorns} nap beneath it. {unicorns|Unicorns} production +250%; {starshard|Starshards} refining +5%.",
        cost: &[
            (ResourceId::Shinies, 500.0),
            (ResourceId::Rainbows, 1_000_000.0),
            (ResourceId::Prisms, 5000.0),
        ],
    },
    ArchDef {
        name: "Starlit Arch",
        blurb: "Lit from within, all night long. {unicorns|Unicorns} production +500%; {starshard|Starshards} refining +10%.",
        cost: &[
            (ResourceId::Shinies, 1250.0),
            (ResourceId::Rainbows, 750_000.0),
            (ResourceId::Prisms, 25000.0),
        ],
    },
    ArchDef {
        name: "Memory Arch",
        blurb: "Every parade it has seen, remembered. Rainbow Arches begin to dream {dreamcorns|Dreamcorns}, faster the more are built.",
        cost: &[
            (ResourceId::Stardust, 2500.0),
            (ResourceId::Prisms, 50000.0),
        ],
    },
    ArchDef {
        name: "Everdream Arch",
        blurb: "A doorway into the softest dream. {jars of purrs|JarsOfPurrs} cap +2; opens condensing jars.",
        cost: &[
            (ResourceId::Dreamcorns, 100.0),
            (ResourceId::Cairns, 1000.0),
        ],
    },
];

// ---- Colonies ----

/// One item in a colony's sell basket: `amount` of `resource` at probability
/// `chance` per successful caravan, plus `season_bonus` when in `season`, only
/// once the Pen Pal level reaches `min_penpal`. @spec SIM-TRADE-003
#[derive(Debug, Clone, Copy)]
pub struct ColonySell {
    pub resource: ResourceId,
    pub amount: f64,
    pub chance: f64,
    pub season: Option<Season>,
    pub season_bonus: f64,
    pub min_penpal: u32,
}

const SELL: ColonySell = ColonySell {
    resource: ResourceId::Catnip,
    amount: 0.0,
    chance: 1.0,
    season: None,
    season_bonus: 0.0,
    min_penpal: 0,
};

/// A tradeable colony: start standing, the buy-resource each caravan spends,
/// and the sell basket. The Star Whale reserves a visitor slot only.
/// @spec SIM-TRADE-003
pub struct ColonyDef {
    pub name: &'static str,
    pub start_standing: f64,
    pub buy: (ResourceId, f64),
    pub sells: &'static [ColonySell],
}

pub fn colony_def(c: ColonyId) -> &'static ColonyDef {
    &COLONIES[c as usize]
}

/// Every good a colony offers, with the Pen Pal level each wants: its sell
/// basket in the table's order, then — for Foxes — the kite-scaled moonstone
/// their trade adds outside the basket, which wants no level.
/// @spec SIM-TRADE-012, SIM-TRADE-004
pub fn colony_goods(c: ColonyId) -> Vec<(ResourceId, u32)> {
    let mut out: Vec<(ResourceId, u32)> = colony_def(c)
        .sells
        .iter()
        .map(|g| (g.resource, g.min_penpal))
        .collect();
    if c == ColonyId::Foxes {
        out.push((ResourceId::Moonstone, 0));
    }
    out
}

#[allow(clippy::needless_update)]
static COLONIES: [ColonyDef; 8] = [
    ColonyDef {
        name: "Otters",
        start_standing: 0.25,
        buy: (ResourceId::Pebbles, 1000.0),
        sells: &[
            ColonySell {
                resource: ResourceId::Twigs,
                amount: 500.0,
                season: Some(Season::Summer),
                season_bonus: 0.35,
                ..SELL
            },
            ColonySell {
                resource: ResourceId::Wands,
                amount: 10.0,
                chance: 0.25,
                ..SELL
            },
        ],
    },
    ColonyDef {
        name: "Raccoons",
        start_standing: 0.0,
        buy: (ResourceId::Buttons, 100.0),
        sells: &[
            ColonySell {
                resource: ResourceId::Catnip,
                amount: 35000.0,
                season: Some(Season::Winter),
                season_bonus: 0.45,
                ..SELL
            },
            ColonySell {
                resource: ResourceId::Quills,
                amount: 5.0,
                chance: 0.25,
                ..SELL
            },
            ColonySell {
                resource: ResourceId::Stories,
                amount: 3.0,
                chance: 0.15,
                ..SELL
            },
        ],
    },
    ColonyDef {
        name: "Foxes",
        start_standing: -0.15,
        buy: (ResourceId::Twigs, 500.0),
        // Moonstone is added programmatically, kite-scaled. @spec SIM-TRADE-004
        sells: &[
            ColonySell {
                resource: ResourceId::Buttons,
                amount: 250.0,
                season: Some(Season::Autumn),
                season_bonus: 0.35,
                ..SELL
            },
            ColonySell {
                resource: ResourceId::Brass,
                amount: 25.0,
                chance: 0.25,
                ..SELL
            },
            ColonySell {
                resource: ResourceId::Cogs,
                amount: 5.0,
                chance: 0.10,
                ..SELL
            },
        ],
    },
    ColonyDef {
        name: "Hedgehogs",
        start_standing: 0.0,
        buy: (ResourceId::Feathers, 500.0),
        sells: &[
            ColonySell {
                resource: ResourceId::Pebbles,
                amount: 1000.0,
                season: Some(Season::Spring),
                season_bonus: 0.25,
                ..SELL
            },
            ColonySell {
                resource: ResourceId::Tiles,
                amount: 5.0,
                chance: 0.75,
                ..SELL
            },
            ColonySell {
                resource: ResourceId::Pavers,
                amount: 1.0,
                chance: 0.10,
                ..SELL
            },
        ],
    },
    ColonyDef {
        name: "Rabbits",
        start_standing: 0.15,
        buy: (ResourceId::Tiles, 50.0),
        sells: &[
            ColonySell {
                resource: ResourceId::Embers,
                amount: 350.0,
                ..SELL
            },
            ColonySell {
                resource: ResourceId::Dew,
                amount: 100.0,
                chance: 0.25,
                ..SELL
            },
            // A garden colony's garden herb: the trade-only luxury the Tea
            // House and the Owls' price both name.
            ColonySell {
                resource: ResourceId::Mint,
                amount: 20.0,
                chance: 0.5,
                season: Some(Season::Spring),
                season_bonus: 0.25,
                ..SELL
            },
        ],
    },
    ColonyDef {
        name: "Deer",
        start_standing: 0.0,
        buy: (ResourceId::Shinies, 250.0),
        sells: &[
            ColonySell {
                resource: ResourceId::Moonstone,
                amount: 1.0,
                chance: 0.95,
                ..SELL
            },
            ColonySell {
                resource: ResourceId::Stardust,
                amount: 0.1,
                chance: 0.50,
                min_penpal: DEER_STARDUST_MIN_PENPAL,
                ..SELL
            },
        ],
    },
    ColonyDef {
        name: "Owls",
        start_standing: -0.10,
        buy: (ResourceId::Mint, 50.0),
        sells: &[ColonySell {
            resource: ResourceId::Curiosity,
            amount: 2500.0,
            ..SELL
        }],
    },
    ColonyDef {
        name: "Star Whales",
        start_standing: 0.0,
        buy: (ResourceId::Hushfelt, 0.0),
        sells: &[], // [D] the visitor slot; basket deferred to the cosmic wave
    },
];

// ---- Wave 5: prestige / the Great Meadow Move (SIM-MOVE) ----

/// The paragon threshold: a Move needs a real clowder. @spec SIM-MOVE-001
pub const MEADOW_MOVE_MIN_KITTENS: usize = 71;
/// Warm Welcome Home's new-meadow catnip gift. @spec SIM-MOVE-012
pub const WARM_WELCOME_CATNIP: f64 = 50.0;
/// Tidy Meadow's mess-accrual multiplier while active. @spec SIM-ADV-002
pub const TIDY_MESS_MULT: f64 = 2.0;
/// Frugal Meadow's energy-consumption twist (+10%) while active, added into the
/// EnergyConsumptionRatio key the reward also flows through. @spec SIM-ADV-002
pub const FRUGAL_MEADOW_TWIST: f64 = 0.10;

/// One Tradition's shop entry — its Elder-Wisdom price. Effects flow through
/// the effect-key system (see `effects::tradition_effect`). @spec SIM-MOVE-010
pub struct TraditionDef {
    pub cost_wisdom: f64,
    /// A short joke and the plain effect, each resource it names marked
    /// (`blurb_parts`).
    pub blurb: &'static str,
}

pub fn tradition_def(t: TraditionId) -> &'static TraditionDef {
    &TRADITIONS[t.ix()]
}

/// The prestige shop table, in TraditionId order. @spec SIM-MOVE-010
static TRADITIONS: [TraditionDef; 30] = [
    TraditionDef { cost_wisdom: 5.0, blurb: "Nothing breaks on the way. Every building's price −1%." },   // CarefulPacking
    TraditionDef { cost_wisdom: 5.0, blurb: "Room for one more. A Move carries 1 more kitten along." },   // TravelingBasketsI
    TraditionDef { cost_wisdom: 35.0, blurb: "Roomier baskets. A Move carries 2 more kittens along." },  // TravelingBasketsII
    TraditionDef { cost_wisdom: 300.0, blurb: "Baskets with an upstairs. A Move carries 4 more kittens along." }, // TravelingBasketsIII
    TraditionDef { cost_wisdom: 10.0, blurb: "A few tins tucked in the luggage. A Move carries 1.5% more of every store." },  // MemoryChestI
    TraditionDef { cost_wisdom: 100.0, blurb: "The good tins, packed carefully. A Move carries 3% more of every store." }, // MemoryChestII
    TraditionDef { cost_wisdom: 5.0, blurb: "Someone left the light on. A new meadow starts with 50 {catnip|Catnip} and a Cozy Cushion." },   // WarmWelcomeHome
    TraditionDef { cost_wisdom: 25.0, blurb: "The old cats remember how. Craft ratio +5%; {quills|Quills}, {stories|Stories}, and {anthologies|Anthologies} craft +25%." },  // ElderStories
    TraditionDef { cost_wisdom: 50.0, blurb: "Pleasing proportions. Every building's price −1.6%." },  // GoldenRatio
    TraditionDef { cost_wisdom: 100.0, blurb: "Rooms that feel right. Every building's price −1.8%." }, // GrandProportion
    TraditionDef { cost_wisdom: 250.0, blurb: "A cat, perfectly in proportion. Every building's price −2%." }, // VitruvianCat
    TraditionDef { cost_wisdom: 750.0, blurb: "Everything, beautifully, again. Every building's price −2.25%." }, // Renaissance
    TraditionDef { cost_wisdom: 500.0, blurb: "A second life, and the wisdom of the first. Elder Wisdom's production bonus, and its cap, +5%." }, // NineLivesI
    TraditionDef { cost_wisdom: 750.0, blurb: "A third life, a little wiser. Elder Wisdom's production bonus, and its cap, +5%." }, // NineLivesII
    TraditionDef {
        cost_wisdom: 1250.0,
        blurb: "Four lives in, and the naps keep getting better. Elder Wisdom's production bonus, and its cap, +5%.",
    }, // NineLivesIII
    TraditionDef {
        cost_wisdom: 1750.0,
        blurb: "Five lives: halfway to legend. Elder Wisdom's production bonus, and its cap, +5%.",
    }, // NineLivesIV
    TraditionDef {
        cost_wisdom: 2500.0,
        blurb: "Six lives, and the stories are long now. Elder Wisdom's production bonus, and its cap, +5%.",
    }, // NineLivesV
    TraditionDef {
        cost_wisdom: 5000.0,
        blurb: "Seven lives, and nothing surprises them. Elder Wisdom's production bonus, and its cap, +5%.",
    }, // NineLivesVI
    TraditionDef {
        cost_wisdom: 7500.0,
        blurb: "Eight lives, and they still chase string. Elder Wisdom's production bonus, and its cap, +5%.",
    }, // NineLivesVII
    TraditionDef {
        cost_wisdom: 15000.0,
        blurb: "Nine lives? Who's counting. Elder Wisdom's production bonus, and its cap, +5%.",
    }, // NineLivesVIII
    TraditionDef {
        cost_wisdom: 30000.0,
        blurb: "More lives than any cat should have. Elder Wisdom's production bonus, and its cap, +5%.",
    }, // NineLivesIX
    TraditionDef {
        cost_wisdom: 60000.0,
        blurb: "Lives beyond counting, every one of them cozy. Elder Wisdom's production bonus, and its cap, +5%.",
    }, // NineLivesX
    TraditionDef { cost_wisdom: 5.0, blurb: "A wave over every fence. Standing with every neighbour +10%." },   // GoodNeighbours
    TraditionDef { cost_wisdom: 25.0, blurb: "A good night's sleep, all round. Kittens arrive 75% more often." },  // WellRested
    TraditionDef { cost_wisdom: 400.0, blurb: "Word gets around. Kittens arrive 150% more often." }, // BrightEyed
    TraditionDef { cost_wisdom: 25.0, blurb: "Why stop? A jamboree hosted while one runs adds its 100 days to the party." },  // FestivalSpirit
    TraditionDef { cost_wisdom: 50.0, blurb: "Some cats bring their work home. On a Move, one carried kitten keeps its job." },  // SecondWind
    TraditionDef { cost_wisdom: 25.0, blurb: "Whiskers that twitch before the weather turns. The coming season is always shown, no idea needed." },  // Forecaster
    TraditionDef { cost_wisdom: 125.0, blurb: "Eyes for sparkle. Prism descents and showers of {rainbows|Rainbows} are 10% likelier." }, // PrismSight
    TraditionDef { cost_wisdom: 40.0, blurb: "What's past the fence? Opens Adventures." },  // AdventurousSpirit
];

// ---- Wave 5: adventures (SIM-ADV) ----

/// One Adventure's reward: a per-completion amount, a LimitedDR cap on the
/// accumulated pool, and the effect key it flows through. Twists and goals are
/// engine behavior (see `season`, `produce`, and `adventure_goal_met`).
/// @spec SIM-ADV-003, SIM-ADV-005
pub struct AdventureDef {
    pub reward_per: f64,
    pub reward_cap: f64,
    pub reward_key: Effect,
}

pub fn adventure_def(a: AdventureId) -> &'static AdventureDef {
    &ADVENTURES[a.ix()]
}

/// The Adventures catalogue, in AdventureId order. @spec SIM-ADV-003
static ADVENTURES: [AdventureDef; 7] = [
    // QuietRetreat: reach 50 kittens → +5% spring catnip (cap ×2).
    AdventureDef {
        reward_per: 0.05,
        reward_cap: 0.10,
        reward_key: Effect::SeasonResourceRatio(Season::Spring, ResourceId::Catnip),
    },
    // LongWinter: reach a Sunbeam Altar → +5% winter catnip and +5% Sunning-Deck
    // summer energy (cap ×2). Only the winter-catnip half is expressible today
    // (SeasonResourceRatio); the Sunning-Deck summer-energy half has no clean
    // effect key yet and is DEFERRED. @spec SIM-ADV-005
    AdventureDef {
        reward_per: 0.05,
        reward_cap: 0.10,
        reward_key: Effect::SeasonResourceRatio(Season::Winter, ResourceId::Catnip),
    },
    // NapTime: build a Yarn Computer → +0.2 master-skill multiplier (cap ×4).
    AdventureDef {
        reward_per: 0.2,
        reward_cap: 0.8,
        reward_key: Effect::MasterSkillMultiplier,
    },
    // FrugalMeadow: build the full energy set → −2% energy consumption (cap ×1).
    // The reward is a NEGATIVE ratio: it lowers consumption through the
    // EnergyConsumptionRatio key (the twist raises it). The cap magnitude is 0.02.
    AdventureDef {
        reward_per: -0.02,
        reward_cap: 0.02,
        reward_key: Effect::EnergyConsumptionRatio,
    },
    // SilentSun: Move with a full Memory Chest → +10% Long Sun effectiveness
    // and +cap (cap ×4). The reward raises the Long Sun cap through the real
    // LongSunCapBonus key (the "+cap" half; the effectiveness half rides the
    // same raised cap). @spec SIM-ADV-005, SIM-SKY-005
    AdventureDef {
        reward_per: 0.10,
        reward_cap: 0.4,
        reward_key: Effect::LongSunCapBonus,
    },
    // GentlePaws: research Pen Pal Networks → +10% pounce ratio (cap ×2).
    AdventureDef {
        reward_per: 0.10,
        reward_cap: 0.20,
        reward_key: Effect::PounceRatio,
    },
    // TidyMeadow: a full tidy year → an exclusive tradition unlock (one-shot,
    // no stacking effect value).
    AdventureDef {
        reward_per: 0.0,
        reward_cap: 1.0,
        reward_key: Effect::MasterSkillMultiplier,
    },
];

// ---- Wave 6: cosmic (SIM-SKY) / dreamtime (SIM-DREAM) ----

/// The reduced transfer fraction the outdoor channel takes of the global,
/// Elder Wisdom, and Long Sun layers. @spec SIM-SKY-004
pub const OUTDOOR_TRANSFER: f64 = 0.25;

/// A Star Whale summon's curiosity + echo cost. @spec SIM-SKY-006
pub const WHALE_SUMMON_CURIOSITY: f64 = 50000.0;
pub const WHALE_SUMMON_ECHOES: f64 = 100.0;
/// A base visit lasts five game-years plus one per contentment point.
pub const WHALE_BASE_VISIT_YEARS: u32 = 5;
/// One lullaby (a dreamcorn offering) raises contentment and lengthens the visit.
/// @spec SIM-SKY-007
pub const WHALE_LULLABY_DREAMCORNS: f64 = 1.0;
/// The visit-gated exchange: hushfelt per caravan → cosmic goods, scaled by
/// contentment. @spec SIM-SKY-007
pub const WHALE_HUSHFELT_PER_CARAVAN: f64 = 1000.0;
pub const WHALE_SKYMAPS_PER_CARAVAN: f64 = 200.0;
pub const WHALE_STARSHARDS_PER_CARAVAN: f64 = 2.0;
pub const WHALE_ECHOES_PER_CARAVAN: f64 = 5.0;

/// Deep Nap bounds and starshard cost curve. @spec SIM-DREAM-001, SIM-DREAM-002
pub const NAP_BASE_MAX_YEARS: u32 = 100;
pub const LONG_SLUMBER_MAX_YEARS: u32 = 400;
pub const LONG_SLUMBER_COST_DISCOUNT: f64 = 0.10;
/// The Deep Cushion's cut to a nap's starshard cost. @spec SIM-DREAM-011
pub const DEEP_CUSHION_COST_DISCOUNT: f64 = 0.05;
/// The Lucid Nap's reverie per year napped. @spec SIM-DREAM-011
pub const LUCID_NAP_REVERIE_PER_YEAR: f64 = 0.1;
pub const STARSHARD_COST_BASE: f64 = 5.0;
pub const STARSHARD_COST_RATIO: f64 = 1.08;
pub const STARSHARD_HEAT_COUPLING: f64 = 0.01; // × stirred_dreams, raising nap cost
/// One nap-year's stirred-dreams heat (before heat-efficiency). @spec SIM-DREAM-002
pub const HEAT_PER_YEAR: f64 = 1.0;
/// Stirred dreams dissipate this much per tick (× 1 + dissipation effects),
/// flooring at zero. @spec SIM-DREAM-003
pub const HEAT_DISSIPATION_PER_TICK: f64 = 0.5;
/// Meadow-year the Distant Dawn era lights (with the loom). @spec SIM-DREAM-007
pub const DISTANT_DAWN_YEAR: u64 = 40000;

/// Per-Memory-Chest déjà-vu chance per season boundary. @spec SIM-DREAM-004
pub const MEMORY_CHEST_DEJA_VU: f64 = 0.01;
/// Reverie gathered on a déjà-vu day (a deterministic RNG draw in this band),
/// raised by reverie-yield effects. @spec SIM-DREAM-004
pub const DEJA_VU_REVERIE_BASE: f64 = 0.5;
pub const DEJA_VU_REVERIE_SPREAD: f64 = 1.5;

/// One cosmic site: its one-time mission fuel and the one outpost it hosts.
/// @spec SIM-SKY-001, SIM-SKY-002
pub struct SiteDef {
    pub name: &'static str,
    pub mission_fuel: &'static [(ResourceId, f64)],
    pub outpost: StructureId,
}

pub fn site_def(s: SiteId) -> &'static SiteDef {
    &SITES[s.ix()]
}

static SITES: [SiteDef; 13] = [
    SiteDef {
        name: "Fence Top",
        mission_fuel: &[(ResourceId::Curiosity, 5000.0), (ResourceId::Skymaps, 50.0)],
        outpost: StructureId::LookoutPost,
    },
    SiteDef {
        name: "Old Oak",
        mission_fuel: &[
            (ResourceId::Curiosity, 25000.0),
            (ResourceId::Skymaps, 200.0),
        ],
        outpost: StructureId::Treehouse,
    },
    SiteDef {
        name: "Rooftops",
        mission_fuel: &[
            (ResourceId::Moonmilk, 50.0),
            (ResourceId::Skymaps, 500.0),
            (ResourceId::Balloons, 2.0),
        ],
        outpost: StructureId::WeatherVane,
    },
    SiteDef {
        name: "Sunny Hill",
        mission_fuel: &[
            (ResourceId::Moonmilk, 150.0),
            (ResourceId::Skymaps, 2000.0),
            (ResourceId::Balloons, 5.0),
        ],
        outpost: StructureId::SolarArray,
    },
    SiteDef {
        name: "Meadow's Edge",
        mission_fuel: &[(ResourceId::Moonmilk, 400.0), (ResourceId::Cometsilk, 25.0)],
        outpost: StructureId::RangerStation,
    },
    SiteDef {
        name: "The Brook",
        mission_fuel: &[
            (ResourceId::Moonmilk, 1000.0),
            (ResourceId::Cometsilk, 75.0),
        ],
        outpost: StructureId::HushfeltGrotto,
    },
    SiteDef {
        name: "Whispering Woods",
        mission_fuel: &[(ResourceId::Cometsilk, 60.0), (ResourceId::Echoes, 5.0)],
        outpost: StructureId::WhisperPost,
    },
    SiteDef {
        name: "The Foothills",
        mission_fuel: &[(ResourceId::Cometsilk, 200.0), (ResourceId::Echoes, 25.0)],
        outpost: StructureId::CairnCircle,
    },
    SiteDef {
        name: "Cloud Nine",
        mission_fuel: &[
            (ResourceId::Cometsilk, 500.0),
            (ResourceId::Echoes, 100.0),
            (ResourceId::Balloons, 20.0),
        ],
        outpost: StructureId::SkyDock,
    },
    SiteDef {
        name: "The Aurora",
        mission_fuel: &[
            (ResourceId::JarsOfPurrs, 5.0),
            (ResourceId::Echoes, 250.0),
            (ResourceId::Dreamweave, 5.0),
        ],
        outpost: StructureId::AuroraLantern,
    },
    SiteDef {
        name: "The Milky Way",
        mission_fuel: &[
            (ResourceId::JarsOfPurrs, 25.0),
            (ResourceId::Echoes, 1000.0),
            (ResourceId::Dreamweave, 15.0),
        ],
        outpost: StructureId::StarNursery,
    },
    SiteDef {
        name: "The Far Shore",
        mission_fuel: &[
            (ResourceId::JarsOfPurrs, 100.0),
            (ResourceId::Echoes, 4000.0),
            (ResourceId::Dreamweave, 40.0),
        ],
        outpost: StructureId::FarBeacon,
    },
    SiteDef {
        name: "The Furthest Star",
        mission_fuel: &[
            (ResourceId::JarsOfPurrs, 500.0),
            (ResourceId::Echoes, 16000.0),
            (ResourceId::Dreamweave, 100.0),
        ],
        outpost: StructureId::WishingStar,
    },
];

/// One Dream Loom upgrade: its reverie (and Jar) cost. Effects flow through the
/// effect-key system (see `effects::loom_effect`). @spec SIM-DREAM-006
pub struct DreamLoomDef {
    pub name: &'static str,
    /// A short joke and the plain effect, each resource it names marked
    /// (`blurb_parts`).
    pub blurb: &'static str,
    pub cost: &'static [(ResourceId, f64)],
}

pub fn dream_loom_def(l: DreamLoomId) -> &'static DreamLoomDef {
    &DREAM_LOOMS[l.ix()]
}

static DREAM_LOOMS: [DreamLoomDef; 9] = [
    DreamLoomDef {
        name: "Soft Pillow",
        blurb: "Sink right in. Naps stir dreams 10% less.",
        cost: &[(ResourceId::Reverie, 5.0)],
    },
    DreamLoomDef {
        name: "Warm Blanket",
        blurb: "Tucked in tight. Stirred dreams settle 20% faster.",
        cost: &[(ResourceId::Reverie, 15.0)],
    },
    DreamLoomDef {
        name: "Dream Catcher",
        blurb: "Catches the good ones. Déjà-vu days come 25% more often.",
        cost: &[(ResourceId::Reverie, 40.0)],
    },
    DreamLoomDef {
        name: "Deep Cushion",
        blurb: "The softest seat in the house. Naps cost 5% fewer {starshards|Starshards}.",
        cost: &[(ResourceId::Reverie, 100.0)],
    },
    DreamLoomDef {
        name: "Lucid Nap",
        blurb: "Awake inside the dream. Each nap brings 0.1 {reverie|Reverie} per year napped.",
        cost: &[(ResourceId::Reverie, 250.0)],
    },
    DreamLoomDef {
        name: "Cozy Cave",
        blurb: "Dark, quiet, warm. Naps stir dreams 15% less.",
        cost: &[(ResourceId::Reverie, 500.0)],
    },
    DreamLoomDef {
        name: "Long Slumber",
        blurb: "The longest nap there is. Naps may run 400 years longer and cost 10% fewer {starshards|Starshards}.",
        cost: &[(ResourceId::Reverie, 1000.0)],
    },
    DreamLoomDef {
        name: "Reverie Net",
        blurb: "Gathers every drifting thought. Déjà-vu days bring 50% more {reverie|Reverie}.",
        cost: &[(ResourceId::Reverie, 2500.0)],
    },
    DreamLoomDef {
        name: "The Distant Dawn",
        blurb: "A warm sunrise after the longest night. At year 40000 the Distant Dawn rises: Passed-Down Wisdom's caps ×4, for good.",
        cost: &[
            (ResourceId::Reverie, 10000.0),
            (ResourceId::JarsOfPurrs, 100.0),
        ],
    },
];
