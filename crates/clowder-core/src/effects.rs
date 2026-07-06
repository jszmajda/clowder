//! The typed effect-key system and shared curve/throttle primitives every
//! content leaf builds on. The engine owns the machinery; leaves own values.

use crate::content;
use crate::state::{
    ArchId, ColonyId, CraftId, GameState, IdeaId, JobId, KittenTrait, ResourceId, RiteId,
    StructureId,
};

/// Compiler-checked modifier vocabulary. Variants are added per content wave;
/// exhaustiveness checks keep every consumer honest.
// @spec SIM-ENG-019
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Effect {
    /// +% on a resource's base production.
    ResourceRatio(ResourceId),
    /// +% on one structure's output.
    StructureRatio(StructureId),
    /// +% on one job's yield.
    JobRatio(JobId),
    /// Flat storage-cap extension.
    MaxCap(ResourceId),
    /// Global production multiplier contribution (stack layer 6).
    GlobalProductionRatio,
    /// Craft output bonus.
    CraftRatio,
    /// Flat happiness percentage points.
    HappinessPoints,
    /// Delta on the per-kitten luxury drain (Gift Stands ease it).
    LuxuryDemandRatio,
    /// % cap extension applied to every cap (Personal Stashes).
    CapRatio,
    /// Craft output bonus on one recipe (Brassy/Alchemical Elders, bindings).
    CraftRatioFor(CraftId),
    /// Pounce yield ratio (the refinement ladder, the Bold Elder).
    PounceRatio,
    /// Tinkerer batch-rate bonus (the paws ladder).
    TinkererSpeed,
    /// Skill-XP accrual bonus.
    SkillXpRatio,
    /// Bonus on the jamboree's +30 happiness points.
    JamboreeRatio,
    /// Bonus on the jamboree's ×2 arrival factor.
    JamboreeArrivalRatio,
    /// Arrival-chance points inside the 60% cap.
    ArrivalChanceInCap,
    /// Arrival-chance points on top of the cap.
    ArrivalChanceOnTop,
    /// Delta on an idea's curiosity price (the Curious Elder).
    IdeaCostRatio,
    /// Trade ratio (dormant until the colonies leaf).
    TradeRatio,
    /// Rite cost delta (dormant until the ceremonies leaf).
    RiteCostRatio,
    /// Flat extra output of one resource per copy of one structure
    /// (Bellows, Ember Beds, the Story Press).
    StructureOutputAdd(StructureId, ResourceId),
    /// Delta on one structure's per-copy mess accrual (the Tidy-Up Rota).
    StructureMessDelta(StructureId),
    /// Sky-event chance multiplier delta (Star Diaries).
    SkyEventChanceRatio,
    /// Auto-observe chance multiplier delta (Patient Eyes).
    AutoObserveChanceRatio,
    // ---- Wave 4 (ceremonies / mystic / colonies) ----
    /// +% cap ratio on one resource (the Shared Sunpuddle rite's altars).
    /// Joins CapRatio inside cap(). @spec SIM-SUN-005
    MaxCapRatio(ResourceId),
    /// Per-day prism-descent probability (Rainbow Arch structures + upgrades).
    /// @spec SIM-UNI-006
    PrismDescentChance,
    /// Per-day rainbow-shower probability (the Prism Arch). @spec SIM-UNI-005
    RainbowShowerChance,
    /// +% Unicorn Meadow output, the mystic tier's own multiplier (Arch
    /// upgrades). @spec SIM-UNI-002
    UnicornsRatio,
    /// +refine on the 25 prisms → 1 starshard conversion. @spec SIM-UNI-003
    StarshardRefine,
    /// +refine on the 25 starshards → 1 echo conversion. @spec SIM-UNI-003
    EchoRefine,
    /// +% on Parade rainbow yield. @spec SIM-UNI-001
    ParadeRatio,
    /// Dedicated per-colony standing bonus (dormant baseline). @spec SIM-TRADE-005
    StandingFor(ColonyId),
}

/// Diminishing-returns primitive: linear to 75% of `limit`, asymptotic to
/// (never reaching) `limit` above it.
// @spec SIM-ENG-020
pub fn limited_dr(value: f64, limit: f64) -> f64 {
    let threshold = 0.75 * limit;
    if value <= threshold {
        value
    } else {
        // Asymptote: remaining quarter of the limit absorbs all excess.
        let excess = value - threshold;
        let room = limit - threshold;
        threshold + room * (excess / (excess + room))
    }
}

/// Diminishing-returns primitive: sqrt-shaped unbounded growth.
// @spec SIM-ENG-020
pub fn unlimited_dr(value: f64, stripe: f64) -> f64 {
    ((1.0 + 8.0 * value / stripe).sqrt() - 1.0) / 2.0
}

impl GameState {
    /// Sum every source's contribution to an effect key, in the fixed source
    /// order (structures, refinements, ideas, house rules, rites, arch
    /// upgrades, traditions, colony standing, adventure rewards, Elder trait).
    /// Sources not yet built contribute nothing; the order is normative so
    /// float sums stay bit-identical as content grows.
    // @spec SIM-ENG-019
    pub fn effect(&self, key: Effect) -> f64 {
        let mut sum = 0.0;
        // 1. Structures (× owned count; global/energy terms count actives —
        // the active set is the built set until toggling lands).
        for s in StructureId::ALL {
            let count = self.structures[s.ix()] as f64;
            if count == 0.0 {
                continue;
            }
            let def = content::structure_def(s);
            match key {
                Effect::MaxCap(r) => {
                    for (cr, amount) in def.cap_bonuses {
                        if *cr == r {
                            let mut value = amount * count;
                            // The Porch's caps ratchet on the kite high-water.
                            // @spec SIM-ECO-031
                            if s == StructureId::Porch {
                                value *= 1.0
                                    + limited_dr(
                                        content::PORCH_KITE_PER * self.kites_high_water,
                                        content::PORCH_KITE_DR_LIMIT,
                                    );
                            }
                            sum += value;
                        }
                    }
                }
                Effect::HappinessPoints => sum += def.happiness_points * count,
                Effect::JamboreeRatio => sum += def.jamboree_ratio * count,
                Effect::JamboreeArrivalRatio => sum += def.jamboree_arrival * count,
                Effect::ResourceRatio(r) => {
                    for (rr, ratio) in def.resource_ratios {
                        if *rr == r {
                            sum += ratio * count;
                        }
                    }
                }
                Effect::CraftRatio => sum += def.craft_ratio * count,
                Effect::LuxuryDemandRatio => sum += def.luxury_demand * count,
                // Each Rainbow Arch *structure* seeds the prism supply.
                // @spec SIM-UNI-006
                Effect::PrismDescentChance if s == StructureId::RainbowArch => {
                    sum += content::RAINBOW_ARCH_PRISM_DESCENT * count;
                }
                // @spec SIM-ECO-032
                Effect::GlobalProductionRatio => {
                    let mut value = def.global_ratio * count;
                    if s == StructureId::DynamoWheel {
                        // A starved wheel spins down; Cogworks amplify it.
                        value *= self.dynamo_fuel_fraction()
                            * (1.0
                                + content::COGWORKS_DYNAMO_BOOST
                                    * self.structures[StructureId::Cogworks.ix()] as f64);
                    }
                    sum += value;
                }
                _ => {}
            }
        }
        // 2. Refinements. Ladder entries stack additively per key; a few
        // scale by an owned-structure count (Catnip Cellars per Pantry,
        // Thicker Blankets per Hearth). Iterate the owned set (BTree-sorted =
        // the same fixed order as RefinementId::ALL) so an empty set is free.
        // @spec SIM-CUR-010, SIM-CUR-012
        for r in &self.refinements {
            let def = content::refinement_def(*r);
            let scale = def
                .per_structure
                .map_or(1.0, |s| self.structures[s.ix()] as f64);
            for (k, value) in def.effects {
                if *k == key {
                    sum += value * scale;
                }
            }
        }
        // 3. Ideas.
        match key {
            Effect::HappinessPoints if self.ideas.contains(&IdeaId::BiscuitMaking) => {
                sum += content::BISCUIT_POINTS;
            }
            Effect::ArrivalChanceOnTop if self.ideas.contains(&IdeaId::NeighborhoodWatch) => {
                sum += content::ARRIVAL_WATCH_BONUS;
            }
            _ => {}
        }
        // 4. House rules — iterate the owned set (BTree order = RuleId::ALL
        // order), free when none adopted. @spec SIM-CUR-011, SIM-CUR-012
        for rule in &self.rules {
            for (k, value) in content::rule_def(*rule).effects {
                if *k == key {
                    sum += value;
                }
            }
        }
        // 5. Sunbeam Rites, scaled by their copy count; altar-hosted rites also
        // scale by the Sunbeam Altar count. The whole section is skipped when no
        // rite is bought (the common hot path). @spec SIM-SUN-007
        if !self.rites.is_empty() {
            let altars = self.structures[StructureId::SunbeamAltar.ix()] as f64;
            for rite in &self.rites {
                let n = self.rite_counts[rite.ix()] as f64;
                if n == 0.0 {
                    continue;
                }
                sum += rite_effect(*rite, key, altars) * n;
            }
        }
        // 6. Rainbow Arch upgrades (bought once each). @spec SIM-UNI-005
        if !self.arches_bought.is_empty() {
            for arch in &self.arches_bought {
                sum += arch_effect(*arch, key);
            }
        }
        // 7–9. Traditions, adventure rewards — arrive with their waves. Colony
        // standing effects are dormant (StandingFor baseline 0).
        // Promises: each kept (non-quiet) Promise grants its small global
        // production effect. @spec SIM-UNI-009
        if !self.promises.is_empty() {
            if let Effect::GlobalProductionRatio = key {
                let kept = self.promises.iter().filter(|p| !p.quiet).count() as f64;
                sum += content::PROMISE_GLOBAL_RATIO * kept;
            }
        }
        // 10. The Elder's birth trait, active only from the chair.
        // @spec SIM-KIT-024
        if let Some(trait_id) = self.elder.and_then(|ix| self.kittens.get(ix)).map(|k| k.trait_id)
        {
            sum += trait_effect(trait_id, key);
        }
        sum
    }

    /// Combined craft-output ratio for one recipe: the global craft bonus
    /// plus any per-recipe bonuses (Brassy/Alchemical Elders, bindings).
    // @spec SIM-ECO-033, SIM-KIT-024
    pub(crate) fn craft_ratio_for(&self, c: CraftId) -> f64 {
        self.effect(Effect::CraftRatio) + self.effect(Effect::CraftRatioFor(c))
    }

    /// Worst-supplied-input fraction for a converter's demand list (0..=1).
    /// Scaling all inputs and outputs by this keeps chains proportional.
    // @spec SIM-ENG-022
    pub fn throttle_fraction(&self, demands: &[(ResourceId, f64)]) -> f64 {
        let mut fraction: f64 = 1.0;
        for (r, demand) in demands {
            if *demand <= 0.0 {
                continue;
            }
            fraction = fraction.min((self.stock(*r) / demand).clamp(0.0, 1.0));
        }
        fraction
    }
}

/// One Sunbeam Rite's per-copy contribution to one effect key. Altar-hosted
/// rites scale additionally by `altars`. Golden Hour (rite 2) is not here — it
/// reshapes the warmth cap directly in `cap()`. @spec SIM-SUN-005, SIM-SUN-007
fn rite_effect(rite: RiteId, key: Effect, altars: f64) -> f64 {
    use ResourceId::{Charm, Curiosity, Pep, Warmth};
    match (rite, key) {
        (RiteId::MorningStretch, Effect::ResourceRatio(Warmth)) => 0.10,
        (RiteId::SharedSunpuddle, Effect::MaxCapRatio(Curiosity)) => 0.02 * altars,
        (RiteId::WarmWelcome, Effect::HappinessPoints) => 0.4 * altars,
        (RiteId::StainedSunlight, Effect::StructureOutputAdd(StructureId::SunbeamAltar, Charm)) => {
            0.05
        }
        (RiteId::GrandBasking, Effect::MaxCap(Charm)) => 60.0 * altars,
        (RiteId::GrandBasking, Effect::StructureOutputAdd(StructureId::SunbeamAltar, Charm)) => 0.1,
        (RiteId::SunWardensOath, Effect::MaxCap(Pep)) => 25.0 * altars,
        _ => 0.0,
    }
}

/// One Rainbow Arch upgrade's contribution to one effect key. @spec SIM-UNI-005
fn arch_effect(arch: ArchId, key: Effect) -> f64 {
    use ResourceId::JarsOfPurrs;
    match (arch, key) {
        (ArchId::RibbonArch, Effect::UnicornsRatio) => 0.05,
        (ArchId::CrystalArch, Effect::UnicornsRatio) => 0.10,
        (ArchId::CrystalArch, Effect::PrismDescentChance) => 0.0005,
        (ArchId::PrismArch, Effect::UnicornsRatio) => 0.25,
        (ArchId::PrismArch, Effect::RainbowShowerChance) => 0.0005,
        (ArchId::SkyArch, Effect::UnicornsRatio) => 0.50,
        (ArchId::DreamingArch, Effect::UnicornsRatio) => 2.50,
        (ArchId::DreamingArch, Effect::StarshardRefine) => 0.05,
        (ArchId::StarlitArch, Effect::UnicornsRatio) => 5.00,
        (ArchId::StarlitArch, Effect::StarshardRefine) => 0.10,
        (ArchId::EverdreamArch, Effect::MaxCap(JarsOfPurrs)) => 2.0,
        _ => 0.0,
    }
}

/// One trait's contribution to one key — the kittens design's trait list,
/// exactly. Charming and Serene are dormant keys until their subsystems land.
// @spec SIM-KIT-024
fn trait_effect(trait_id: KittenTrait, key: Effect) -> f64 {
    use CraftId::*;
    match (trait_id, key) {
        (KittenTrait::Crafty, Effect::CraftRatio) => 0.05,
        (KittenTrait::Brassy, Effect::CraftRatioFor(Badges | Brass | Cogs | Moonsteel)) => 0.10,
        (
            KittenTrait::Alchemical,
            Effect::CraftRatioFor(Pavers | Moonmilk | Dreamweave | Cometsilk),
        ) => 0.075,
        (KittenTrait::Charming, Effect::TradeRatio) => 0.03,
        (KittenTrait::Bold, Effect::PounceRatio) => 0.5,
        (KittenTrait::Curious, Effect::IdeaCostRatio) => -0.05,
        (KittenTrait::Serene, Effect::RiteCostRatio) => -0.09,
        _ => 0.0,
    }
}
