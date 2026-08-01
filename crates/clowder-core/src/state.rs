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
    RainbowArch,
    Sunstone,
    // ---- Wave 6: cosmic outposts (ids 40..=52), each Site-gated ----
    LookoutPost,
    Treehouse,
    WeatherVane,
    SolarArray,
    RangerStation,
    HushfeltGrotto,
    WhisperPost,
    CairnCircle,
    SkyDock,
    AuroraLantern,
    StarNursery,
    FarBeacon,
    WishingStar,
}

pub const STRUCTURE_COUNT: usize = 53;

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
        StructureId::RainbowArch,
        StructureId::Sunstone,
        StructureId::LookoutPost,
        StructureId::Treehouse,
        StructureId::WeatherVane,
        StructureId::SolarArray,
        StructureId::RangerStation,
        StructureId::HushfeltGrotto,
        StructureId::WhisperPost,
        StructureId::CairnCircle,
        StructureId::SkyDock,
        StructureId::AuroraLantern,
        StructureId::StarNursery,
        StructureId::FarBeacon,
        StructureId::WishingStar,
    ];
    pub fn ix(self) -> usize {
        self as usize
    }
}

/// A structure's build gate: always available, unlocked by an idea, or gated on
/// a cosmic site being reached (the outposts 40..=52). @spec SIM-ECO-028
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructureGate {
    Always,
    Idea(IdeaId),
    Site(SiteId),
}

/// The thirteen cosmic expedition sites, in ladder order (a site is reachable
/// only once its predecessor is reached). Copy + Ord so a `BTreeSet<SiteId>`
/// records reached sites in canonical order. @spec SIM-SKY-002
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SiteId {
    FenceTop,
    OldOak,
    Rooftops,
    SunnyHill,
    MeadowsEdge,
    TheBrook,
    WhisperingWoods,
    TheFoothills,
    CloudNine,
    TheAurora,
    TheMilkyWay,
    TheFarShore,
    TheFurthestStar,
}

impl SiteId {
    pub const ALL: [SiteId; 13] = [
        SiteId::FenceTop,
        SiteId::OldOak,
        SiteId::Rooftops,
        SiteId::SunnyHill,
        SiteId::MeadowsEdge,
        SiteId::TheBrook,
        SiteId::WhisperingWoods,
        SiteId::TheFoothills,
        SiteId::CloudNine,
        SiteId::TheAurora,
        SiteId::TheMilkyWay,
        SiteId::TheFarShore,
        SiteId::TheFurthestStar,
    ];
    pub fn ix(self) -> usize {
        self as usize
    }
}

/// The nine Dream Loom upgrades, in ladder order. Copy + Ord so a
/// `BTreeSet<DreamLoomId>` records woven looms in canonical order.
/// @spec SIM-DREAM-006
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DreamLoomId {
    SoftPillow,
    WarmBlanket,
    DreamCatcher,
    DeepCushion,
    LucidNap,
    CozyCave,
    LongSlumber,
    ReverieNet,
    DistantDawn,
}

impl DreamLoomId {
    pub const ALL: [DreamLoomId; 9] = [
        DreamLoomId::SoftPillow,
        DreamLoomId::WarmBlanket,
        DreamLoomId::DreamCatcher,
        DreamLoomId::DeepCushion,
        DreamLoomId::LucidNap,
        DreamLoomId::CozyCave,
        DreamLoomId::LongSlumber,
        DreamLoomId::ReverieNet,
        DreamLoomId::DistantDawn,
    ];
    pub fn ix(self) -> usize {
        self as usize
    }
}

// @spec SIM-ECO-040, SIM-ECO-041, SIM-ECO-042, SIM-ECO-043
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

/// Birth traits, rolled uniformly at arrival (one draw after the name draw).
/// A trait's clowder-wide effect applies only while its kitten is the Elder.
/// Some cats simply are: None is a full member of the eight.
// @spec SIM-KIT-023, SIM-KIT-024
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum KittenTrait {
    #[default]
    None,
    Crafty,
    Brassy,
    Alchemical,
    Charming,
    Bold,
    Curious,
    Serene,
}

impl KittenTrait {
    pub const ALL: [KittenTrait; 8] = [
        KittenTrait::None,
        KittenTrait::Crafty,
        KittenTrait::Brassy,
        KittenTrait::Alchemical,
        KittenTrait::Charming,
        KittenTrait::Bold,
        KittenTrait::Curious,
        KittenTrait::Serene,
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
    LongSunMasonry,
    SkywardSong,
}

pub const IDEA_COUNT: usize = 59;

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
        IdeaId::LongSunMasonry,
        IdeaId::SkywardSong,
    ];
}

/// The Refinements table, ids append-only (35 in the society wave).
// @spec SIM-CUR-010
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RefinementId {
    PebbleCombs,
    ButtonCombs,
    BrassCombs,
    MoonsteelCombs,
    PebbleSnips,
    ButtonShears,
    BrassClippers,
    MoonsteelPruners,
    DeepPockets,
    SortingClaws,
    BolasOfYarn,
    PouncePads,
    WhiskerGuards,
    MoonsteelMittens,
    DeepShelves,
    StackedCrates,
    LabeledJars,
    CatnipCellars,
    BiggerBaskets,
    Bellows,
    EmberBeds,
    StoryPress,
    BetterBindings,
    FieldNotes,
    StarDiaries,
    PatientEyes,
    TidyUpRota,
    GroupLessons,
    EncouragingMews,
    NimblePaws,
    PatientPaws,
    ClockworkPaws,
    WarmWelcomeMat,
    ThickerBlankets,
    DewFunnels,
}

pub const REFINEMENT_COUNT: usize = 35;

impl RefinementId {
    pub const ALL: [RefinementId; REFINEMENT_COUNT] = [
        RefinementId::PebbleCombs,
        RefinementId::ButtonCombs,
        RefinementId::BrassCombs,
        RefinementId::MoonsteelCombs,
        RefinementId::PebbleSnips,
        RefinementId::ButtonShears,
        RefinementId::BrassClippers,
        RefinementId::MoonsteelPruners,
        RefinementId::DeepPockets,
        RefinementId::SortingClaws,
        RefinementId::BolasOfYarn,
        RefinementId::PouncePads,
        RefinementId::WhiskerGuards,
        RefinementId::MoonsteelMittens,
        RefinementId::DeepShelves,
        RefinementId::StackedCrates,
        RefinementId::LabeledJars,
        RefinementId::CatnipCellars,
        RefinementId::BiggerBaskets,
        RefinementId::Bellows,
        RefinementId::EmberBeds,
        RefinementId::StoryPress,
        RefinementId::BetterBindings,
        RefinementId::FieldNotes,
        RefinementId::StarDiaries,
        RefinementId::PatientEyes,
        RefinementId::TidyUpRota,
        RefinementId::GroupLessons,
        RefinementId::EncouragingMews,
        RefinementId::NimblePaws,
        RefinementId::PatientPaws,
        RefinementId::ClockworkPaws,
        RefinementId::WarmWelcomeMat,
        RefinementId::ThickerBlankets,
        RefinementId::DewFunnels,
    ];
}

/// House Rules: four mutually exclusive, charm-priced pairs.
// @spec SIM-CUR-011
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RuleId {
    EarlyBirds,
    NightOwls,
    OpenMeadow,
    CozyBurrows,
    SharingCircle,
    PersonalStashes,
    LoudPurring,
    QuietContemplation,
}

impl RuleId {
    pub const ALL: [RuleId; 8] = [
        RuleId::EarlyBirds,
        RuleId::NightOwls,
        RuleId::OpenMeadow,
        RuleId::CozyBurrows,
        RuleId::SharingCircle,
        RuleId::PersonalStashes,
        RuleId::LoudPurring,
        RuleId::QuietContemplation,
    ];
}

/// The eight neighbouring colonies the clowder trades with. Copy + Ord so a
/// `BTreeSet<ColonyId>` records discoveries in canonical order; `as usize`
/// indexes the per-colony pen-pal array. @spec SIM-TRADE-003
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ColonyId {
    Otters,
    Raccoons,
    Foxes,
    Hedgehogs,
    Rabbits,
    Deer,
    Owls,
    StarWhales,
}

impl ColonyId {
    pub const ALL: [ColonyId; 8] = [
        ColonyId::Otters,
        ColonyId::Raccoons,
        ColonyId::Foxes,
        ColonyId::Hedgehogs,
        ColonyId::Rabbits,
        ColonyId::Deer,
        ColonyId::Owls,
        ColonyId::StarWhales,
    ];
    pub fn ix(self) -> usize {
        self as usize
    }
}

/// The eleven Sunbeam Rites, in ladder order. @spec SIM-SUN-005
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RiteId {
    MorningStretch,
    SharedSunpuddle,
    GoldenHour,
    WarmWelcome,
    StainedSunlight,
    TheLongSun,
    GrandBasking,
    SunWardensOath,
    TheReverie,
    Enlightenment,
    EverwarmFrescoes,
}

impl RiteId {
    pub const ALL: [RiteId; 11] = [
        RiteId::MorningStretch,
        RiteId::SharedSunpuddle,
        RiteId::GoldenHour,
        RiteId::WarmWelcome,
        RiteId::StainedSunlight,
        RiteId::TheLongSun,
        RiteId::GrandBasking,
        RiteId::SunWardensOath,
        RiteId::TheReverie,
        RiteId::Enlightenment,
        RiteId::EverwarmFrescoes,
    ];
    pub fn ix(self) -> usize {
        self as usize
    }
}

/// The eight Rainbow Arch upgrades, in ladder order. @spec SIM-UNI-005
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ArchId {
    RibbonArch,
    CrystalArch,
    PrismArch,
    SkyArch,
    DreamingArch,
    StarlitArch,
    MemoryArch,
    EverdreamArch,
}

impl ArchId {
    pub const ALL: [ArchId; 8] = [
        ArchId::RibbonArch,
        ArchId::CrystalArch,
        ArchId::PrismArch,
        ArchId::SkyArch,
        ArchId::DreamingArch,
        ArchId::StarlitArch,
        ArchId::MemoryArch,
        ArchId::EverdreamArch,
    ];
    pub fn ix(self) -> usize {
        self as usize
    }
}

/// The prestige Traditions shop, in shop-table order (indices are load-bearing:
/// the Nine Lives ladder 12..=21 and the derived `nine_lives_ratio` count owned
/// traditions by index). Copy + Ord so a `BTreeSet<TraditionId>` records
/// purchases in canonical order. @spec SIM-MOVE-010
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TraditionId {
    CarefulPacking,
    TravelingBasketsI,
    TravelingBasketsII,
    TravelingBasketsIII,
    MemoryChestI,
    MemoryChestII,
    WarmWelcomeHome,
    ElderStories,
    GoldenRatio,
    GrandProportion,
    VitruvianCat,
    Renaissance,
    NineLivesI,
    NineLivesII,
    NineLivesIII,
    NineLivesIV,
    NineLivesV,
    NineLivesVI,
    NineLivesVII,
    NineLivesVIII,
    NineLivesIX,
    NineLivesX,
    GoodNeighbours,
    WellRested,
    BrightEyed,
    FestivalSpirit,
    SecondWind,
    Forecaster,
    PrismSight,
    AdventurousSpirit,
}

impl TraditionId {
    pub const ALL: [TraditionId; 30] = [
        TraditionId::CarefulPacking,
        TraditionId::TravelingBasketsI,
        TraditionId::TravelingBasketsII,
        TraditionId::TravelingBasketsIII,
        TraditionId::MemoryChestI,
        TraditionId::MemoryChestII,
        TraditionId::WarmWelcomeHome,
        TraditionId::ElderStories,
        TraditionId::GoldenRatio,
        TraditionId::GrandProportion,
        TraditionId::VitruvianCat,
        TraditionId::Renaissance,
        TraditionId::NineLivesI,
        TraditionId::NineLivesII,
        TraditionId::NineLivesIII,
        TraditionId::NineLivesIV,
        TraditionId::NineLivesV,
        TraditionId::NineLivesVI,
        TraditionId::NineLivesVII,
        TraditionId::NineLivesVIII,
        TraditionId::NineLivesIX,
        TraditionId::NineLivesX,
        TraditionId::GoodNeighbours,
        TraditionId::WellRested,
        TraditionId::BrightEyed,
        TraditionId::FestivalSpirit,
        TraditionId::SecondWind,
        TraditionId::Forecaster,
        TraditionId::PrismSight,
        TraditionId::AdventurousSpirit,
    ];
    pub fn ix(self) -> usize {
        self as usize
    }
}

/// The Adventures catalogue, in catalogue order (`as usize` indexes the
/// per-adventure completion and reward arrays). @spec SIM-ADV-003
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AdventureId {
    QuietRetreat,
    LongWinter,
    NapTime,
    FrugalMeadow,
    SilentSun,
    GentlePaws,
    TidyMeadow,
}

impl AdventureId {
    pub const ALL: [AdventureId; 7] = [
        AdventureId::QuietRetreat,
        AdventureId::LongWinter,
        AdventureId::NapTime,
        AdventureId::FrugalMeadow,
        AdventureId::SilentSun,
        AdventureId::GentlePaws,
        AdventureId::TidyMeadow,
    ];
    pub fn ix(self) -> usize {
        self as usize
    }
}

/// A slim Family Album memorial — never a full Kitten (no skills/job). The
/// Album is append-only across every Move, the game's memory of every kitten.
/// @spec SIM-MOVE-001
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArchivedKitten {
    pub name: String,
    pub trait_id: KittenTrait,
    #[serde(default, deserialize_with = "saturating_u8")]
    pub rank: u8,
    #[serde(default)]
    pub generation: u32,
}

/// A Promise to the Night Court: fills a slot, draws gentle dreamcorn upkeep,
/// and grants a small global effect while kept. A Promise whose upkeep goes
/// unpaid goes `quiet` (effect suspended, slot held) and wakes on its own.
/// @spec SIM-UNI-009, SIM-UNI-010
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Promise {
    #[serde(default)]
    pub quiet: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
///
/// Two kinds are *reveal-tracked*: they climb the cost-fraction ladder of
/// SIM-ENG-023 and their reveals persist in `revealed`. The rest carry no
/// ladder and answer with the two states they have. New variants are appended
/// rather than inserted: `revealed` is a `BTreeSet<ContentId>` whose iteration
/// order reaches the save bytes, and derived `Ord` follows declaration order,
/// so inserting a variant ahead of `Structure` or `Idea` would reorder every
/// existing save. @spec SIM-ENG-032
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ContentId {
    Structure(StructureId),
    Idea(IdeaId),
    Craft(CraftId),
    Job(JobId),
    Rite(RiteId),
    Arch(ArchId),
    Refinement(RefinementId),
    Rule(RuleId),
    Tradition(TraditionId),
    DreamLoom(DreamLoomId),
}

impl ContentId {
    /// Whether this kind climbs the reveal ladder and persists in `revealed`.
    /// The two-state kinds are absent from that set by construction, so an
    /// availability query must not read it for them — it would answer Hidden
    /// forever. @spec SIM-ENG-032
    pub fn is_reveal_tracked(self) -> bool {
        matches!(self, ContentId::Structure(_) | ContentId::Idea(_))
    }
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
    /// Per-job skill XP, JobId-indexed. Older saves zero-fill.
    /// @spec SIM-KIT-019, SIM-KIT-027
    #[serde(default)]
    pub skills: [f64; 8],
    /// Lifetime XP pool, spent on promotions. @spec SIM-KIT-021
    #[serde(default)]
    pub exp: f64,
    /// Promoted with exp + shinies. @spec SIM-KIT-021
    #[serde(default, deserialize_with = "saturating_u8")]
    pub rank: u8,
    /// Birth trait; active clowder-wide only while this kitten is the Elder.
    /// @spec SIM-KIT-023
    #[serde(default)]
    pub trait_id: KittenTrait,
    /// The recipe this Tinkerer is bound to, if any. @spec SIM-ECO-043
    #[serde(default)]
    pub tinkering: Option<CraftId>,
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
pub(crate) fn serialize_array<S: serde::Serializer, T: Serialize, const N: usize>(
    v: &[T; N],
    s: S,
) -> Result<S::Ok, S::Error> {
    v[..].serialize(s)
}

/// Deserialize an enum-indexed array, zero-filling the tail: older saves
/// carry shorter arrays because ids only ever append. Longer-than-known
/// arrays are corrupt (a newer save is refused by the version check first).
// @spec SAVE-013
pub(crate) fn zero_filled_array<'de, D: Deserializer<'de>, T, const N: usize>(
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
    /// The largest the roster has ever been — a ratchet, like the kite
    /// high-water. Prestige is revealed by *having met* the size it asks for,
    /// not by currently having it: a gate that un-reveals would take an armed
    /// row out from under the cursor the moment a kitten wandered off, and
    /// TUI-NAV-043 wants a commit that is no longer valid to fail to the
    /// footer instead. Reveal is for what a meadow has never met.
    /// @spec SIM-ENG-046
    #[serde(default)]
    pub roster_high_water: usize,
    /// Permanently revealed content ids. @spec SIM-ENG-023
    #[serde(default)]
    pub revealed: BTreeSet<ContentId>,
    /// What the player has lived through — canonical state, because it cannot
    /// be recomputed from anything else. A save written before the journal
    /// existed defaults to empty, which is the honest representation of
    /// history it never recorded. @spec SIM-ENG-027, SAVE-011, SAVE-014
    #[serde(default)]
    pub journal: crate::journal::Journal,
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
    /// Roster index of the Elder, if one is appointed. @spec SIM-KIT-022
    #[serde(default)]
    pub elder: Option<usize>,
    /// Tick until which a jamboree runs (active while ticks < jamboree_until);
    /// 0 when none. @spec SIM-KIT-025
    #[serde(default)]
    pub jamboree_until: u64,
    /// Purchased refinements — permanent. @spec SIM-CUR-010
    #[serde(default)]
    pub refinements: BTreeSet<RefinementId>,
    /// Adopted house rules — permanent for this meadow. @spec SIM-CUR-011
    #[serde(default)]
    pub rules: BTreeSet<RuleId>,
    // ---- Wave 4: ceremonies (SIM-SUN) ----
    /// Devotion pool, offered up by Basking. @spec SIM-SUN-001
    #[serde(default)]
    pub devotion: f64,
    /// Legacy serenity — only ever grows, persists the Move. @spec SIM-SUN-002
    #[serde(default)]
    pub serenity: f64,
    /// The high-water mark of devotion ever reached — the Rite worship-gate.
    /// @spec SIM-SUN-005
    #[serde(default)]
    pub peak_devotion: f64,
    /// The Enlightenment tier, serenity-bought and monotone. @spec SIM-SUN-008
    #[serde(default)]
    pub enlightenment_tier: u32,
    /// Bought Sunbeam Rites (membership). @spec SIM-SUN-005
    #[serde(default)]
    pub rites: BTreeSet<RiteId>,
    /// Per-rite copy count — restacked rites (SIM-SUN-006) exceed one; effects
    /// and prices scale by it. Old saves zero-fill. @spec SIM-SUN-006
    #[serde(default)]
    pub rite_counts: [u32; 11],
    // ---- Wave 4: mystic (SIM-UNI) ----
    /// Bought Rainbow Arch upgrades. @spec SIM-UNI-005
    #[serde(default)]
    pub arches_bought: BTreeSet<ArchId>,
    /// Dream-drift accumulator; at ≥ 1 a prism naps into a dreamcorn.
    /// @spec SIM-UNI-007
    #[serde(default)]
    pub dream_drift: f64,
    /// The Promises to the Night Court, in slot order. @spec SIM-UNI-009
    #[serde(default)]
    pub promises: Vec<Promise>,
    // ---- Wave 4: colonies (SIM-TRADE) ----
    /// Per-colony Pen Pal level, ColonyId-indexed. @spec SIM-TRADE-006
    #[serde(default)]
    pub penpal: [u32; 8],
    /// Discovered colonies — permanent. @spec SIM-TRADE-008
    #[serde(default)]
    pub discovered: BTreeSet<ColonyId>,
    // ---- Wave 5: prestige / the Great Meadow Move (SIM-MOVE) ----
    /// Capped-linear production/storage legacy; earned on a Move, spent on
    /// Traditions and Pass It Down. @spec SIM-MOVE-006, SIM-MOVE-007
    #[serde(default)]
    pub elder_wisdom: f64,
    /// Sqrt-diminishing happiness legacy; earned on a Move, only ever grows.
    /// @spec SIM-MOVE-008
    #[serde(default)]
    pub warm_memories: f64,
    /// The burn-tier second production pool (Pass It Down). @spec SIM-MOVE-009
    #[serde(default)]
    pub passed_down_wisdom: f64,
    /// The meadow generation, incremented by each Move. @spec SIM-MOVE-004
    #[serde(default)]
    pub generation: u32,
    /// Purchased Traditions — permanent across every Move. @spec SIM-MOVE-010
    #[serde(default)]
    pub traditions: BTreeSet<TraditionId>,
    /// The append-only Family Album — every kitten who ever lived, archived by
    /// generation, never deleted. @spec SIM-MOVE-001
    #[serde(default)]
    pub family_album: Vec<ArchivedKitten>,
    // ---- Wave 5: adventures (SIM-ADV) ----
    /// The Adventure armed for the next Move. @spec SIM-ADV-001
    #[serde(default)]
    pub armed_adventure: Option<AdventureId>,
    /// The Adventure active in this meadow (its twist applies). @spec SIM-ADV-002
    #[serde(default)]
    pub active_adventure: Option<AdventureId>,
    /// Per-adventure completion counts, AdventureId-indexed. @spec SIM-ADV-004
    #[serde(default)]
    pub adventure_completions: [u32; 7],
    /// Per-adventure accumulated raw reward pool (LimitedDR-capped in `effect`);
    /// permanent, surviving every Move. @spec SIM-ADV-005
    #[serde(default)]
    pub adventure_rewards: [f64; 7],
    /// Whether the active adventure's goal has been met this meadow — the twist
    /// stays until the following Move, then lifts. @spec SIM-ADV-004
    #[serde(default)]
    pub(crate) adventure_done: bool,
    /// Consecutive clean-mess days toward the Tidy Meadow year goal.
    /// @spec SIM-ADV-004
    #[serde(default)]
    pub(crate) tidy_progress: u32,
    // ---- Wave 6: cosmic (SIM-SKY) / dreamtime (SIM-DREAM) ----
    /// Reached cosmic sites — permanent, preserved across every Move.
    /// @spec SIM-SKY-003, SIM-MOVE-004
    #[serde(default)]
    pub sites_reached: BTreeSet<SiteId>,
    /// Woven Dream Loom upgrades — permanent, preserved across every Move.
    /// @spec SIM-DREAM-006, SIM-MOVE-004
    #[serde(default)]
    pub dream_looms: BTreeSet<DreamLoomId>,
    /// Stirred-dreams heat: raised by naps, dissipating passively; a nap-cadence
    /// brake that never reduces any stock. @spec SIM-DREAM-002, SIM-DREAM-003
    #[serde(default)]
    pub stirred_dreams: f64,
    /// The Distant Dawn era, once lit never cleared, preserved across Moves;
    /// raises the Passed-Down Wisdom production cap. @spec SIM-DREAM-007
    #[serde(default)]
    pub distant_dawn: bool,
    /// Remaining Star Whale visit in game-years (0 = not visiting).
    /// @spec SIM-SKY-006
    #[serde(default)]
    pub whale_visit_years: u32,
    /// The visiting whale's contentment, raised by feeding. @spec SIM-SKY-007
    #[serde(default)]
    pub whale_contentment: u32,
    /// Tick until which a déjà-vu day pauses production and consumption.
    /// @spec SIM-DREAM-004
    #[serde(default)]
    pub deja_vu_until: u64,
    /// Derived accelerator for the per-tick reveal scan: the complement of
    /// `revealed`, in canonical order. Never serialized — rebuilt on load —
    /// so it can't drift from `revealed`, which stays the sole truth.
    /// @spec SIM-ENG-023
    #[serde(skip)]
    pub(crate) unrevealed_structs: Vec<StructureId>,
    #[serde(skip)]
    pub(crate) unrevealed_ideas: Vec<IdeaId>,
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
            journal: crate::journal::Journal::default(),
            roster_high_water: 0,
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
            elder: None,
            jamboree_until: 0,
            refinements: BTreeSet::new(),
            rules: BTreeSet::new(),
            devotion: 0.0,
            serenity: 0.0,
            peak_devotion: 0.0,
            enlightenment_tier: 0,
            rites: BTreeSet::new(),
            rite_counts: [0; 11],
            arches_bought: BTreeSet::new(),
            dream_drift: 0.0,
            promises: Vec::new(),
            penpal: [0; 8],
            discovered: BTreeSet::new(),
            elder_wisdom: 0.0,
            warm_memories: 0.0,
            passed_down_wisdom: 0.0,
            generation: 0,
            traditions: BTreeSet::new(),
            family_album: Vec::new(),
            armed_adventure: None,
            active_adventure: None,
            adventure_completions: [0; 7],
            adventure_rewards: [0.0; 7],
            adventure_done: false,
            tidy_progress: 0,
            sites_reached: BTreeSet::new(),
            dream_looms: BTreeSet::new(),
            stirred_dreams: 0.0,
            distant_dawn: false,
            whale_visit_years: 0,
            whale_contentment: 0,
            deja_vu_until: 0,
            unrevealed_structs: StructureId::ALL.to_vec(),
            unrevealed_ideas: IdeaId::ALL.to_vec(),
        }
    }

    /// Rebuild the unrevealed worklists as the exact complement of `revealed`,
    /// in canonical order. Called after deserialization (the worklists are not
    /// serialized) so live play and a reloaded game share one reveal state.
    // @spec SIM-ENG-023
    pub(crate) fn rebuild_unrevealed(&mut self) {
        self.unrevealed_structs = StructureId::ALL
            .iter()
            .copied()
            .filter(|s| !self.revealed.contains(&ContentId::Structure(*s)))
            .collect();
        self.unrevealed_ideas = IdeaId::ALL
            .iter()
            .copied()
            .filter(|i| !self.revealed.contains(&ContentId::Idea(*i)))
            .collect();
    }

    // ---- Calendar (derived from ticks alone) ----
    // @spec SIM-ENG-001

    pub fn day(&self) -> u64 {
        self.ticks / content::TICKS_PER_DAY
    }

    pub fn season(&self) -> Season {
        // The Long Winter adventure holds the meadow in Winter regardless of
        // the tick counter — the one engine-owned twist every leaf reads here.
        // @spec SIM-ADV-002
        if self.active_adventure == Some(AdventureId::LongWinter) {
            return Season::Winter;
        }
        season_of_day(self.day())
    }

    // ---- Wave 5 derived queries (prestige / adventures) ----

    /// Elder Wisdom's production cap/slope multiplier: 1.0 + 0.05 per owned
    /// Nine Lives tradition (ladder indices 12..=21). @spec SIM-MOVE-011
    pub fn nine_lives_ratio(&self) -> f64 {
        let n = self
            .traditions
            .iter()
            .filter(|t| (12..=21).contains(&(t.ix())))
            .count();
        1.0 + 0.05 * n as f64
    }

    /// Traveling-Basket carry capacity: I +1, II +2, III +4 (default 0).
    /// @spec SIM-MOVE-002
    pub fn basket_capacity(&self) -> u32 {
        let mut cap = 0;
        if self.traditions.contains(&TraditionId::TravelingBasketsI) {
            cap += 1;
        }
        if self.traditions.contains(&TraditionId::TravelingBasketsII) {
            cap += 2;
        }
        if self.traditions.contains(&TraditionId::TravelingBasketsIII) {
            cap += 4;
        }
        cap
    }

    /// Memory Chest carry fraction: I +1.5%, II +3%, plus 1.5% per Memory Chest
    /// structure owned in the CURRENT meadow (read before the Move's reset).
    /// @spec SIM-MOVE-003
    pub fn memory_chest_ratio(&self) -> f64 {
        let mut r = 0.0;
        if self.traditions.contains(&TraditionId::MemoryChestI) {
            r += 0.015;
        }
        if self.traditions.contains(&TraditionId::MemoryChestII) {
            r += 0.03;
        }
        r += 0.015 * self.structures[StructureId::MemoryChest.ix()] as f64;
        r
    }

    /// The active adventure's goal predicate (checked at day close).
    /// @spec SIM-ADV-004
    pub(crate) fn adventure_goal_met(&self, a: AdventureId) -> bool {
        match a {
            AdventureId::QuietRetreat => self.kittens.len() >= 50,
            AdventureId::LongWinter => self.structures[StructureId::SunbeamAltar.ix()] >= 1,
            AdventureId::NapTime => self.structures[StructureId::YarnComputer.ix()] >= 1,
            AdventureId::FrugalMeadow => {
                self.structures[StructureId::DynamoWheel.ix()] >= 1
                    && self.structures[StructureId::StreamWheel.ix()] >= 1
                    && self.structures[StructureId::StarBasin.ix()] >= 1
            }
            AdventureId::SilentSun => self.memory_chest_ratio() > 0.0,
            AdventureId::GentlePaws => self.ideas.contains(&IdeaId::PenPalNetworks),
            AdventureId::TidyMeadow => self.tidy_progress >= content::DAYS_PER_YEAR as u32,
        }
    }

    /// The stacked structure-price multiplier from price-reduction Traditions:
    /// 1 − limited_dr(GlobalPriceRatioDelta, 0.5), always > 0.5. Only Traditions
    /// feed the delta, so the tradition-free hot path is a free 1.0.
    /// @spec SIM-MOVE-013
    pub(crate) fn global_price_mult(&self) -> f64 {
        if self.traditions.is_empty() {
            return 1.0;
        }
        let delta = self.effect(crate::effects::Effect::GlobalPriceRatioDelta);
        1.0 - crate::effects::limited_dr(delta, 0.5)
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

    /// Storage cap for a resource: (base + flat extensions) × (1 + cap-ratio
    /// effects). Infinite caps stay infinite and a zero cap stays zero — the
    /// Cardboard-Box gate never opens by percentage.
    // @spec SIM-ECO-001, SIM-ECO-002, SIM-CUR-011
    pub fn cap(&self, r: ResourceId) -> f64 {
        use crate::effects::Effect;
        // MaxCapRatio arrives only from Rites, so the rite-free hot path (the
        // per-tick clamp) skips that scan entirely.
        let max_cap_ratio =
            if self.rites.is_empty() { 0.0 } else { self.effect(Effect::MaxCapRatio(r)) };
        // Elder Wisdom (+0.1%/pt) and Passed-Down Wisdom (+0.05%/pt) fold into
        // the storage ratio term. @spec SIM-MOVE-007, SIM-MOVE-009
        let ratio = 1.0
            + self.effect(Effect::CapRatio)
            + max_cap_ratio
            + 0.001 * self.elder_wisdom
            + 0.0005 * self.passed_down_wisdom;
        // The Golden Hour rite multiplies the whole warmth reservoir (every
        // warmth-cap source, Sun Rooms and Sunbeam Altars alike) by
        // (1.4 + 0.1 per Golden Hour) — buying it always enlarges the pantry,
        // never shrinks it. @spec SIM-SUN-005
        if r == ResourceId::Warmth {
            let golden = self.rite_counts[RiteId::GoldenHour.ix()];
            if golden > 0 {
                let warmth_sources = content::BASE_CAPS[r.ix()] + self.effect(Effect::MaxCap(r));
                return warmth_sources * (1.4 + 0.1 * golden as f64) * ratio;
            }
        }
        (content::BASE_CAPS[r.ix()] + self.effect(Effect::MaxCap(r))) * ratio
    }

    // ---- Wave 4 derived queries ----

    /// Every Basking is multiplied by (1 + reverence); reverence grows with
    /// serenity. @spec SIM-SUN-001
    pub fn reverence(&self) -> f64 {
        crate::effects::unlimited_dr(self.serenity, 0.1) * 0.1
    }

    /// The Long Sun production bonus (layer 8), 0 without the Long Sun rite;
    /// else limited_dr(unlimited_dr(devotion, 1000) / 100, cap), where
    /// cap = 10 + 0.05 × enlightenment_tier per Sunstone. @spec SIM-SUN-004
    pub fn long_sun_bonus(&self) -> f64 {
        if !self.rites.contains(&RiteId::TheLongSun) {
            return 0.0;
        }
        let sunstones = self.structures[StructureId::Sunstone.ix()] as f64;
        // Aurora Lanterns extend the Long Sun cap (+2 each). @spec SIM-SKY-005
        let cap = 10.0 + 0.05 * self.enlightenment_tier as f64 * sunstones
            + self.effect(crate::effects::Effect::LongSunCapBonus);
        crate::effects::limited_dr(
            crate::effects::unlimited_dr(self.devotion, 1000.0) / 100.0,
            cap,
        )
    }

    /// A colony's standing: start + Pen Pal warming + dedicated standing
    /// effects. A shy colony (negative start) only ever warms toward neutral,
    /// clamped between its start floor and 0. @spec SIM-TRADE-005
    pub fn standing(&self, colony: ColonyId) -> f64 {
        let def = content::colony_def(colony);
        let warm = (content::PENPAL_STANDING_PER * self.penpal[colony.ix()] as f64)
            .min(content::PENPAL_STANDING_CAP)
            + self.effect(crate::effects::Effect::StandingFor(colony));
        let raw = def.start_standing + warm;
        // Good Neighbours (a global StandingRatio) warms standing everywhere:
        // multiplying up a warm colony, and lifting a shy one toward neutral.
        // @spec SIM-MOVE-012
        let ratio = self.effect(crate::effects::Effect::StandingRatio);
        if def.start_standing < 0.0 {
            (raw + ratio * (-def.start_standing)).clamp(def.start_standing, 0.0)
        } else {
            raw * (1.0 + ratio)
        }
    }

    pub fn stock(&self, r: ResourceId) -> f64 {
        self.resources[r.ix()]
    }

    /// Cost of the next copy of a structure: ceil(base × ratio^owned) per component.
    // @spec SIM-ECO-007
    pub fn next_cost(&self, s: StructureId) -> Vec<(ResourceId, f64)> {
        let def = content::structure_def(s);
        let owned = self.structures[s.ix()] as i32;
        // Price-reduction Traditions shave every component (DR-clamped, never
        // to zero). @spec SIM-MOVE-013
        let mult = self.global_price_mult();
        def.base_costs
            .iter()
            .map(|(r, base)| (*r, (base * def.ratio.powi(owned) * mult).ceil()))
            .collect()
    }

    pub fn structure_unlocked(&self, s: StructureId) -> bool {
        match content::structure_def(s).gate {
            StructureGate::Always => true,
            StructureGate::Idea(idea) => self.ideas.contains(&idea),
            StructureGate::Site(site) => self.sites_reached.contains(&site),
        }
    }

    // ---- Wave 6 derived queries (cosmic / dreamtime) ----

    /// The next unreached site is reachable once the cosmic surface (Skyward
    /// Longing) is open and every prior rung is reached. An already-reached
    /// site is not "reachable" (it needs no launching). @spec SIM-SKY-001
    pub fn site_reachable(&self, site: SiteId) -> bool {
        if !self.ideas.contains(&IdeaId::SkywardLonging) || self.sites_reached.contains(&site) {
            return false;
        }
        for prior in SiteId::ALL {
            if prior == site {
                break;
            }
            if !self.sites_reached.contains(&prior) {
                return false;
            }
        }
        true
    }

    /// The longest a single Deep Nap may run, in game-years: base 100, raised by
    /// the Long Slumber loom. @spec SIM-DREAM-001, SIM-DREAM-006
    pub fn max_nap_length(&self) -> u32 {
        let mut n = content::NAP_BASE_MAX_YEARS;
        if self.dream_looms.contains(&DreamLoomId::LongSlumber) {
            n += content::LONG_SLUMBER_MAX_YEARS;
        }
        n
    }

    /// The starshard cost of a `years`-long nap: an escalating geometric sum,
    /// raised by the current stirred-dreams heat, shaved by the Long Slumber
    /// loom. @spec SIM-DREAM-001, SIM-DREAM-002
    pub fn starshard_cost(&self, years: u32) -> f64 {
        let y = years as f64;
        let ratio = content::STARSHARD_COST_RATIO;
        let base = content::STARSHARD_COST_BASE * (ratio.powf(y) - 1.0) / (ratio - 1.0);
        let heat = 1.0 + content::STARSHARD_HEAT_COUPLING * self.stirred_dreams;
        let discount = if self.dream_looms.contains(&DreamLoomId::LongSlumber) {
            1.0 - content::LONG_SLUMBER_COST_DISCOUNT
        } else {
            1.0
        };
        // A whole number of starshards: exact against any (even 1e12) stock, so
        // the spend is precise. @spec SIM-DREAM-001
        (base * heat * discount).ceil()
    }

    /// The reduced transfer fraction the outdoor channel takes of the global,
    /// Elder Wisdom, and Long Sun layers — a state-independent constant.
    /// @spec SIM-SKY-004, SIM-ENG-021
    pub fn outdoor_transfer(&self) -> f64 {
        content::OUTDOOR_TRANSFER
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
        // Story Circles and Moonlight Stages pool into one DR-limited relief
        // knob: at least 10% of the crowding always bites. With no crowding the
        // relief term vanishes (it multiplies crowding), so the structure scan is
        // skipped — the quiet/small-meadow hot path, and happiness runs thrice a
        // tick.
        let relief = if crowding > 0.0 {
            let relief_sum: f64 = StructureId::ALL
                .iter()
                .map(|s| {
                    content::structure_def(*s).crowding_relief * self.structures[s.ix()] as f64
                })
                .sum();
            crate::effects::limited_dr(relief_sum, content::CROWDING_RELIEF_DR_LIMIT)
        } else {
            0.0
        };
        // Each distinct stocked luxury brightens the room (dust-guarded).
        let luxuries = content::LUXURY_DRAINS
            .iter()
            .filter(|(r, _)| self.resources[r.ix()] > EPSILON)
            .count() as f64;
        let mut h = 100.0 - content::CROWDING_POINTS * crowding * (1.0 - relief)
            + (content::LOAFER_POINTS * loafers).min(content::LOAFER_POINTS_MAX)
            + content::LUXURY_HAPPINESS_POINTS * luxuries
            + self.effect(crate::effects::Effect::HappinessPoints);
        // @spec SIM-KIT-025
        if self.jamboree_active() {
            h += content::JAMBOREE_HAPPINESS
                * (1.0 + self.effect(crate::effects::Effect::JamboreeRatio));
        }
        // The Warm Memories legacy brightens every future meadow (sqrt-DR);
        // free until a Move earns some. @spec SIM-MOVE-008, SIM-KIT-011
        if self.warm_memories > 0.0 {
            h += crate::effects::unlimited_dr(self.warm_memories, 5.0);
        }
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
    /// The pipeline composes in the specified order: inside-cap effects join
    /// the base sum before the 60% cap, on-top effects land after it, mess
    /// multiplies the whole sum, a jamboree multiplies again, clamp [0, 1].
    // @spec SIM-KIT-004
    pub fn arrival_chance(&self) -> f64 {
        let hundreds = (self.stock(ResourceId::Catnip) / 100.0).floor();
        let mut chance = (content::ARRIVAL_BASE_CHANCE
            + content::ARRIVAL_PER_100_CATNIP * hundreds
            + self.effect(crate::effects::Effect::ArrivalChanceInCap))
        .min(content::ARRIVAL_CHANCE_CAP);
        chance += self.effect(crate::effects::Effect::ArrivalChanceOnTop);
        // Serious clutter makes newcomers shy (level 1+). @spec SIM-ECO-038
        if self.mess_level() >= 1 {
            chance *= content::MESS_ARRIVAL_FACTOR;
        }
        // @spec SIM-KIT-025
        if self.jamboree_active() {
            chance *= content::JAMBOREE_ARRIVAL_BASE
                + self.effect(crate::effects::Effect::JamboreeArrivalRatio);
        }
        // Arrival-speed Traditions (Well-Rested, Bright-Eyed). Only Traditions
        // feed KittenGrowthRatio, so the tradition-free hot path skips the scan.
        // @spec SIM-MOVE-012
        if !self.traditions.is_empty() {
            chance *= 1.0 + self.effect(crate::effects::Effect::KittenGrowthRatio);
        }
        chance.clamp(0.0, 1.0)
    }

    pub fn zoomies_active(&self) -> bool {
        self.ticks < self.zoomies_until
    }

    /// A jamboree runs while ticks < jamboree_until. @spec SIM-KIT-025
    pub fn jamboree_active(&self) -> bool {
        self.ticks < self.jamboree_until
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
                StructureId::SunningDeck | StructureId::SolarArray => {
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
        // The Frugal Meadow reward (−2% per completion) and its twist (+10%
        // while active) both flow through the EnergyConsumptionRatio key: the
        // twist raises the draw, the reward lowers it, summed together. The
        // effect is 1.0 (no change) with neither active — the common hot path.
        // @spec SIM-ADV-005, SIM-ADV-002
        sum * (1.0 + self.effect(crate::effects::Effect::EnergyConsumptionRatio)).max(0.0)
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

    /// (deficit, surplus) ratios from a single production/consumption pass —
    /// the hot-path form used inside `tick()`, byte-identical to calling the
    /// two public queries but scanning the structures once instead of twice.
    /// @spec SIM-ECO-036
    pub(crate) fn energy_ratios(&self) -> (f64, f64) {
        let mut prod = 0.0;
        let mut cons = 0.0;
        for (ix, &count) in self.structures.iter().enumerate() {
            if count == 0 {
                continue;
            }
            let s = StructureId::ALL[ix];
            let n = count as f64;
            let def = content::structure_def(s);
            if def.energy_prod != 0.0 {
                prod += match s {
                    StructureId::SunningDeck | StructureId::SolarArray => {
                        def.energy_prod * n * content::sunning_deck_season_factor(self.season())
                    }
                    StructureId::DynamoWheel => def.energy_prod * n * self.dynamo_fuel_fraction(),
                    _ => def.energy_prod * n,
                };
            }
            cons += match s {
                StructureId::YarnComputer => 2.0 * n + n * n / 2.0,
                _ => def.energy_cons * n,
            };
        }
        // Frugal Meadow scales consumption through the EnergyConsumptionRatio key
        // (twist raises, reward lowers) — kept byte-identical to
        // energy_consumption(). Only Frugal Meadow ever touches this key, so the
        // effect() sum is skipped entirely when it is neither active nor earned —
        // the common hot path draws nothing extra. @spec SIM-ADV-005, SIM-ADV-002
        if self.active_adventure == Some(AdventureId::FrugalMeadow)
            || self.adventure_rewards[AdventureId::FrugalMeadow.ix()] != 0.0
        {
            cons *= (1.0 + self.effect(crate::effects::Effect::EnergyConsumptionRatio)).max(0.0);
        }
        let deficit = if cons <= EPSILON {
            1.0
        } else {
            (prod / cons).clamp(0.25, 1.0)
        };
        let surplus = (prod / cons.max(1.0)).clamp(0.0, 1.75);
        (deficit, surplus)
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
    /// The largest roster this meadow has ever held, never less than the one
    /// in front of you. A ratchet with a floor, like the kite high-water: a
    /// hand-edited or freshly-built state is judged on what it plainly has,
    /// and growth is remembered after the fact. @spec SIM-ENG-046
    pub fn roster_reach(&self) -> usize {
        self.roster_high_water.max(self.kittens.len())
    }

    pub fn availability(&self, id: ContentId) -> Availability {
        if !id.is_reveal_tracked() {
            return self.two_state_availability(id);
        }
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
            _ => unreachable!("guarded by is_reveal_tracked"),
        };
        if actionable {
            Availability::Available
        } else {
            Availability::Teased
        }
    }

    /// Availability for a kind that carries no reveal ladder: it is either
    /// there to act on or not there at all, never Teased. The presence check
    /// mirrors the same gate the command layer's Locked precondition tests
    /// for each kind, so this answer and that error can never disagree.
    /// @spec SIM-ENG-032
    fn two_state_availability(&self, id: ContentId) -> Availability {
        let unlocked = match id {
            ContentId::Structure(_) | ContentId::Idea(_) => {
                unreachable!("guarded by is_reveal_tracked")
            }
            ContentId::Craft(c) => self.craft_unlocked(c),
            ContentId::Job(j) => self.job_unlocked(j),
            ContentId::Rite(r) => {
                let silent_sun = self.active_adventure == Some(AdventureId::SilentSun);
                !silent_sun && self.peak_devotion + EPSILON >= content::rite_def(r).devotion_gate
            }
            ContentId::Arch(_) => self.ideas.contains(&IdeaId::ParadeRoutes),
            ContentId::Refinement(r) => self.ideas.contains(&content::refinement_def(r).gate),
            // House rules are bought with curiosity and live beside the ideas,
            // so they appear on the same gate: once the meadow can hold what
            // buys them. @spec SIM-ENG-032, SIM-ENG-044
            ContentId::Rule(r) => {
                self.cap(ResourceId::Curiosity) > 0.0
                    && !self.rules.contains(&content::rule_def(r).blocks)
            }
            // A Tradition's currency cannot exist before a first Move, so the
            // shop opens with the generation that earns it. Ungated, a fresh
            // meadow would report the whole prestige ladder and its prices —
            // the reveal pacing every other kind honours. @spec SIM-ENG-032
            ContentId::Tradition(_) => self.generation >= 1,
            // Likewise the loom ladder waits on the idea that introduces it.
            // @spec SIM-ENG-032
            ContentId::DreamLoom(_) => self.ideas.contains(&IdeaId::DeepNapTheory),
        };
        if unlocked {
            Availability::Available
        } else {
            Availability::Hidden
        }
    }

    /// Would this item reveal right now? (Sticky insertion happens in tick.)
    /// The tradition-free hot path: no structure-price multiplier, so the
    /// per-tick reveal scan stays exactly the wave-4 loop. The priced variant
    /// (`reveal_condition_met_priced`) is only taken once a price Tradition is
    /// bought. @spec SIM-ENG-023
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
            // Kinds with no cost-reveal ladder never enter the scan worklist.
            _ => false,
        }
    }

    /// Reveal check with price-reduction Traditions applied — the reveal-cost
    /// half of SIM-MOVE-013. Only reached post-prestige (when `traditions` is
    /// non-empty), so it need not be as lean as the hot-path variant.
    /// @spec SIM-MOVE-013
    pub(crate) fn reveal_condition_met_priced(&self, id: ContentId, price_mult: f64) -> bool {
        match id {
            ContentId::Structure(s) => {
                let def = content::structure_def(s);
                let owned = self.structures[s.ix()] as i32;
                self.structure_unlocked(s)
                    && def.base_costs.iter().all(|(r, base)| {
                        let amount = (base * def.ratio.powi(owned) * price_mult).ceil();
                        self.stock(*r) + EPSILON >= def.reveal_fraction * amount
                    })
            }
            ContentId::Idea(i) => self.idea_unlocked(i),
            // Kinds with no cost-reveal ladder never enter the scan worklist.
            _ => false,
        }
    }
}

pub(crate) fn season_index_of_day(day: u64) -> usize {
    ((day / content::DAYS_PER_SEASON) % 4) as usize
}

pub(crate) fn season_of_day(day: u64) -> Season {
    Season::ALL[season_index_of_day(day)]
}
