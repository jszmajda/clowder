use crate::state::{
    AdventureId, ArchId, ColonyId, ContentId, DreamLoomId, IdeaId, JobId, MoodTier, RefinementId,
    RiteId, RuleId, Season, SiteId, StructureId, TraditionId,
};
use serde::{Deserialize, Serialize};

/// How prominent an occurrence is. A property of the vocabulary rather than a
/// judgement the consumer makes: deciding that one occurrence matters more
/// than another is a rule about the game, and a shell deciding it would hold
/// knowledge the crate split exists to keep out. It is also what the journal
/// reads to know which entries it may never evict.
/// @spec SIM-ENG-026
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventClass {
    /// Pure delight, no mechanical effect. Rendered quietest.
    Flavor,
    /// An ordinary mechanical occurrence.
    Mechanical,
    /// A moment worth remembering. Never evicted from the journal.
    Milestone,
}

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
    // ---- Wave 4: ceremonies (SIM-SUN) ----
    Basked { devotion: f64 },
    Reveried { serenity: f64 },
    RiteBought { id: RiteId },
    Enlightened { tier: u32 },
    // ---- Wave 4: mystic (SIM-UNI) ----
    ParadeHeld { rainbows: f64 },
    ArchBought { id: ArchId },
    StarshardRefined,
    EchoRefined,
    JarCondensed,
    PromiseMade,
    PromiseQuieted { index: usize },
    PromiseResumed { index: usize },
    PrismDescended,
    // ---- Wave 4: colonies (SIM-TRADE) ----
    TradeReturned { colony: ColonyId, caravans: u32 },
    PenPalSent { colony: ColonyId, level: u32 },
    ColonyDiscovered { colony: ColonyId },
    // ---- Wave 5: prestige / adventures (SIM-MOVE / SIM-ADV) ----
    MeadowMoved { generation: u32 },
    Passed { passed_down_wisdom: f64 },
    TraditionBought { id: TraditionId },
    AdventureDeclared { id: Option<AdventureId> },
    AdventureCompleted { id: AdventureId },
    // ---- Wave 6: cosmic (SIM-SKY) / dreamtime (SIM-DREAM) ----
    SiteReached { site: SiteId },
    WhaleSummoned,
    WhaleFed { contentment: u32 },
    /// A summarized nap report — never a per-tick stream, and the same
    /// `OfflineReport` kind offline absence folds. @spec SIM-DREAM-001, SIM-DREAM-008
    Napped { years: u32, report: Box<crate::save::OfflineReport> },
    DreamLoomBought { id: DreamLoomId },
    DejaVuDay,
    DistantDawn,
}

impl Event {
    /// This occurrence's prominence — a property of which variant this is,
    /// never its payload. Milestones are the moments worth remembering:
    /// permanent unlocks, new content reached, one-time turning points.
    /// Ordinary state changes are mechanical; pure delight with no mechanical
    /// effect is flavor. @spec SIM-ENG-026
    pub fn class(&self) -> EventClass {
        // Only the *first* of a structure is the moment worth keeping; later
        // copies are ordinary. Milestones are permanent, so every one must be
        // bounded over a playthrough by construction. @spec SIM-ENG-026, SIM-ENG-042
        if let Event::StructureBuilt { count, .. } = self {
            return if *count == 1 { EventClass::Milestone } else { EventClass::Mechanical };
        }
        match self {
            Event::Flavor { .. } => EventClass::Flavor,

            Event::KittenArrived { .. }
            | Event::KittenWanderedOff { .. }
            | Event::KittenReturned { .. }
            | Event::JobAssigned { .. }
            | Event::JobUnassigned { .. }
            | Event::SeasonChanged { .. }
            | Event::ZoomiesStarted { .. }
            | Event::ZoomiesEnded
            | Event::KittenMood { .. }
            | Event::Revealed { .. }
            | Event::SkyEventVisible
            | Event::SpeedChanged { .. }
            | Event::PounceReturned { .. }
            | Event::JamboreeStarted { .. }
            | Event::JamboreeEnded
            | Event::RefinementBought { .. }
            | Event::RuleAdopted { .. }
            | Event::Basked { .. }
            | Event::Reveried { .. }
            | Event::ParadeHeld { .. }
            | Event::StarshardRefined
            | Event::EchoRefined
            | Event::JarCondensed
            | Event::PromiseMade
            | Event::PromiseQuieted { .. }
            | Event::PromiseResumed { .. }
            | Event::PrismDescended
            | Event::TradeReturned { .. }
            | Event::PenPalSent { .. }
            | Event::Passed { .. }
            | Event::AdventureDeclared { .. }
            | Event::WhaleFed { .. }
            | Event::DejaVuDay
            | Event::Napped { .. }
            // A year turns every ~31 real seconds at the fastest speed; as a
            // milestone it would write permanent entries without bound.
            // @spec SIM-ENG-042
            | Event::NewYear { .. } => EventClass::Mechanical,

            Event::IdeaResearched { .. }
            | Event::ElderAppointed { .. }
            | Event::KittenPromoted { .. }
            | Event::RiteBought { .. }
            | Event::Enlightened { .. }
            | Event::ArchBought { .. }
            | Event::ColonyDiscovered { .. }
            | Event::MeadowMoved { .. }
            | Event::TraditionBought { .. }
            | Event::AdventureCompleted { .. }
            | Event::SiteReached { .. }
            | Event::WhaleSummoned
            | Event::DreamLoomBought { .. }
            | Event::DistantDawn => EventClass::Milestone,

            // Decided above, on its count.
            Event::StructureBuilt { .. } => unreachable!("handled before the match"),
        }
    }
}
