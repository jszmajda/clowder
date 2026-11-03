//! The typed effect-key system and shared curve/throttle primitives every
//! content leaf builds on. The engine owns the machinery; leaves own values.

use crate::content;
use crate::state::{
    AdventureId, ArchId, ColonyId, CraftId, GameState, JobId, KittenTrait, ResourceId, RiteId,
    Season, StructureId, TraditionId,
};

mod index;
use index::Source;

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
    /// % cap extension applied to every cap (Personal Stashes, Far Beacon).
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
    /// +% on the dreamcorn engine's dream-drift rate (dormant baseline — no
    /// content grants it yet; keeps the SIM-UNI-007 formula honest). Summed like
    /// any other key; contributes 0 until content lands. @spec SIM-UNI-007
    DreamBoost,
    /// Dedicated per-colony standing bonus (dormant baseline). @spec SIM-TRADE-005
    StandingFor(ColonyId),
    // ---- Wave 5 (prestige / adventures) ----
    /// Summed structure-price reduction from Traditions, DR-clamped in
    /// next_cost. @spec SIM-MOVE-013
    GlobalPriceRatioDelta,
    /// Multiplier delta on the kitten arrival chance (Well-Rested, Bright-Eyed).
    /// @spec SIM-MOVE-012
    KittenGrowthRatio,
    /// Global trade-standing bonus (Good Neighbours). @spec SIM-MOVE-012
    StandingRatio,
    /// Master-skill multiplier reward (Nap Time). @spec SIM-ADV-005
    MasterSkillMultiplier,
    /// Energy-consumption reduction reward (Frugal Meadow). @spec SIM-ADV-005
    EnergyConsumptionRatio,
    /// Ratio on the mess structures make, never on what they tidy (Tidy Paws).
    /// @spec SIM-ECO-052
    MessAccrualRatio,
    /// Per-season, per-resource production bonus (Quiet Retreat, Long Winter).
    /// @spec SIM-ADV-005
    SeasonResourceRatio(Season, ResourceId),
    // ---- Wave 6 (cosmic / dreamtime) ----
    /// +% on the outdoor production channel (Ranger Station, Cairn Circle,
    /// Aurora Lantern, Far Beacon, Wishing Star).
    OutdoorProductionRatio,
    /// Flat Long Sun cap extension (Aurora Lantern).
    LongSunCapBonus,
    /// Per-season déjà-vu day chance (Memory Chests + the Dream Catcher loom).
    /// @spec SIM-DREAM-004
    DejaVuChance,
    /// +% reverie gathered on a déjà-vu day (the Reverie Net loom).
    /// @spec SIM-DREAM-004
    ReverieYield,
    /// Nap heat efficiency: naps stir (1 − this) as much (Soft Pillow, Cozy
    /// Cave). @spec SIM-DREAM-002
    HeatEfficiency,
    /// +% stirred-dreams dissipation (the Warm Blanket loom). @spec SIM-DREAM-003
    HeatDissipation,
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
    /// order (structures, refinements, ideas, house rules, rites, arch upgrades,
    /// traditions, adventure rewards and the twist, kept promises, dream loom
    /// upgrades, Elder trait). Sources not yet held contribute nothing; the
    /// order is normative so float sums stay bit-identical as content grows.
    /// Only the sources that can carry the key are read, which is what makes
    /// this cheap enough for the hundreds of sums a tick asks for.
    // @spec SIM-ENG-019, SIM-ENG-084
    #[inline]
    pub fn effect(&self, key: Effect) -> f64 {
        let sources = index::sources(key);
        if sources.is_empty() {
            return 0.0;
        }
        self.sum_sources(sources, key)
    }

    fn sum_sources(&self, sources: &[Source], key: Effect) -> f64 {
        let mut sum = 0.0;
        for source in sources {
            sum += self.contribution(source, key);
        }
        sum
    }

    /// What one source gives a key in this state: zero for one the meadow does
    /// not hold, read at the call and never remembered, since a held count, a
    /// stock, or who sits in the Elder's chair can move between any two sums.
    #[inline]
    fn contribution(&self, source: &Source, key: Effect) -> f64 {
        match *source {
            // Structures × copies held. @spec SIM-ENG-019
            Source::Structure {
                structure,
                per_copy,
                kite_ratcheted,
                fuel_scaled,
            } => {
                let copies = self.structures[structure.ix()];
                if copies == 0 {
                    return 0.0;
                }
                let mut value = per_copy * copies as f64;
                // Marked caps ratchet on the kite high-water.
                // @spec SIM-ECO-031
                if kite_ratcheted {
                    value *= 1.0
                        + limited_dr(
                            content::PORCH_KITE_PER * self.kites_high_water,
                            content::PORCH_KITE_DR_LIMIT,
                        );
                }
                // A starved structure spins down; boosts amplify it.
                // @spec SIM-ECO-032
                if fuel_scaled {
                    value *= self.fuel_fraction(structure)
                        * (1.0 + self.structure_sum(|d| d.fuel_global_boost));
                }
                value
            }
            // Refinements stack additively per key; a few scale by an
            // owned-structure count (Catnip Cellars per Pantry, Thicker Blankets
            // per Hearth). @spec SIM-CUR-010, SIM-CUR-012
            Source::Refinement {
                refinement,
                value,
                per_structure,
            } => {
                if !self.refinements.contains(&refinement) {
                    return 0.0;
                }
                value * per_structure.map_or(1.0, |s| self.structures[s.ix()] as f64)
            }
            Source::Idea { idea, value } => {
                if self.ideas.contains(&idea) {
                    value
                } else {
                    0.0
                }
            }
            // @spec SIM-CUR-011, SIM-CUR-012
            Source::Rule { rule, value } => {
                if self.rules.contains(&rule) {
                    value
                } else {
                    0.0
                }
            }
            // Sunbeam Rites, scaled by their copy count; altar-hosted rites also
            // scale by the Sunbeam Altar count. @spec SIM-SUN-007
            Source::Rite(rite) => {
                if !self.rites.contains(&rite) {
                    return 0.0;
                }
                let altars = self.structures[StructureId::SunbeamAltar.ix()] as f64;
                rite_effect(rite, key, altars) * self.rite_counts[rite.ix()] as f64
            }
            // Everwarm Frescoes: each Sun Room gains warmth cap =
            // unlimited_dr(frescoes, 1.65) × Warm Memories (the prestige legacy
            // the rite reads). @spec SIM-SUN-005, SIM-MOVE-008
            Source::Frescoes => {
                let frescoes = self.rite_counts[RiteId::EverwarmFrescoes.ix()];
                if self.rites.is_empty() || frescoes == 0 || self.warm_memories <= 0.0 {
                    return 0.0;
                }
                let sun_rooms = self.structures[StructureId::SunRoom.ix()] as f64;
                unlimited_dr(frescoes as f64, 1.65) * self.warm_memories * sun_rooms
            }
            // Rainbow Arch upgrades (bought once each). @spec SIM-UNI-005
            Source::Arch(arch) => {
                if self.arches_bought.contains(&arch) {
                    arch_effect(arch, key)
                } else {
                    0.0
                }
            }
            // Traditions (permanent prestige purchases). @spec SIM-MOVE-010, SIM-MOVE-012
            Source::Tradition(tradition) => {
                if self.traditions.contains(&tradition) {
                    tradition_effect(tradition, key)
                } else {
                    0.0
                }
            }
            // Adventure rewards (LimitedDR-capped, permanent). The pool may be
            // negative (Frugal Meadow lowers consumption); LimitedDR caps its
            // magnitude, sign kept. @spec SIM-ADV-005
            Source::AdventureReward { adventure, cap } => {
                let pool = self.adventure_rewards[adventure.ix()];
                pool.signum() * limited_dr(pool.abs(), cap)
            }
            // The Frugal Meadow twist raises energy consumption +10% while active —
            // it flows through the same key the reward lowers, so both land in one
            // summed multiplier. @spec SIM-ADV-002
            Source::FrugalTwist => {
                if self.active_adventure == Some(AdventureId::FrugalMeadow) {
                    content::FRUGAL_MEADOW_TWIST
                } else {
                    0.0
                }
            }
            // Each kept (non-quiet) Promise grants its small global production
            // effect. @spec SIM-UNI-009
            Source::KeptPromises => {
                let kept = self.promises.iter().filter(|p| !p.quiet).count() as f64;
                content::PROMISE_GLOBAL_RATIO * kept
            }
            // Dream Loom upgrades (bought once each). @spec SIM-DREAM-006
            Source::Loom(loom) => {
                if self.dream_looms.contains(&loom) {
                    loom_effect(loom, key)
                } else {
                    0.0
                }
            }
            // The Elder's birth trait, active only from the chair.
            // @spec SIM-KIT-024
            Source::ElderTrait => self
                .elder
                .and_then(|ix| self.kittens.get(ix))
                .map_or(0.0, |k| trait_effect(k.trait_id, key)),
        }
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
        self.throttle_of(demands.iter().copied())
    }

    /// The same fraction over demands worked out as they are read, so a tick's
    /// converters allocate nothing to ask it.
    #[inline]
    pub(crate) fn throttle_of(&self, demands: impl Iterator<Item = (ResourceId, f64)>) -> f64 {
        let mut fraction: f64 = 1.0;
        for (r, demand) in demands {
            if demand <= 0.0 {
                continue;
            }
            fraction = fraction.min((self.stock(r) / demand).clamp(0.0, 1.0));
        }
        fraction
    }
}

/// An item whose effects can name a store, for reading what one copy of it
/// does apart from what the meadow holds.
#[derive(Debug, Clone, Copy)]
pub(crate) enum EffectItem {
    Refinement(crate::state::RefinementId),
    Rule(crate::state::RuleId),
    Rite(RiteId),
    Arch(ArchId),
    Loom(crate::state::DreamLoomId),
    Tradition(TraditionId),
}

/// Every effect key that says which store it grows: a store's production or
/// cap by name, or through the one job, structure, or recipe the key names. A
/// key across every store at once names none, since marking them all says
/// nothing. @spec SIM-ENG-081
fn store_keys() -> Vec<(Effect, ResourceId)> {
    let mut keys = vec![
        (Effect::UnicornsRatio, ResourceId::Unicorns),
        (Effect::StarshardRefine, ResourceId::Starshards),
        (Effect::ReverieYield, ResourceId::Reverie),
    ];
    for r in ResourceId::ALL {
        keys.push((Effect::ResourceRatio(r), r));
        keys.push((Effect::MaxCap(r), r));
        keys.push((Effect::MaxCapRatio(r), r));
        for season in crate::state::Season::ALL {
            keys.push((Effect::SeasonResourceRatio(season, r), r));
        }
    }
    for j in JobId::ALL {
        if let Some((r, _)) = content::job_def(j).yield_per_tick {
            keys.push((Effect::JobRatio(j), r));
        }
    }
    for c in CraftId::ALL {
        keys.push((Effect::CraftRatioFor(c), content::craft_output(c)));
    }
    for s in StructureId::ALL {
        let d = content::structure_def(s);
        for (r, _) in d.productions {
            keys.push((Effect::StructureRatio(s), *r));
        }
        if let Some(c) = &d.converter {
            for (r, _) in c.outputs {
                keys.push((Effect::StructureRatio(s), *r));
            }
        }
    }
    keys
}

/// Each resource whose production or cap the item's effects name, with the
/// signed size of what one copy does. @spec SIM-ENG-081
pub(crate) fn store_effects(item: EffectItem) -> Vec<(ResourceId, f64)> {
    let of = |key: Effect| -> f64 {
        let listed = |effects: &[(Effect, f64)]| -> f64 {
            effects
                .iter()
                .filter(|(k, _)| *k == key)
                .map(|(_, v)| *v)
                .sum()
        };
        match item {
            EffectItem::Refinement(f) => listed(content::refinement_def(f).effects),
            EffectItem::Rule(r) => listed(content::rule_def(r).effects),
            EffectItem::Rite(t) => rite_effect(t, key, 1.0),
            EffectItem::Arch(a) => arch_effect(a, key),
            EffectItem::Loom(l) => loom_effect(l, key),
            EffectItem::Tradition(t) => tradition_effect(t, key),
        }
    };
    let mut sizes: std::collections::BTreeMap<ResourceId, f64> = Default::default();
    for (key, r) in store_keys() {
        *sizes.entry(r).or_default() += of(key);
    }
    // These two rites shape the warmth cap by their own rules, not by a key.
    if matches!(
        item,
        EffectItem::Rite(RiteId::GoldenHour | RiteId::EverwarmFrescoes)
    ) {
        *sizes.entry(ResourceId::Warmth).or_default() += 1.0;
    }
    sizes.into_iter().filter(|(_, v)| *v != 0.0).collect()
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

/// One Dream Loom upgrade's contribution to one effect key — the dreamtime
/// design's loom table. The Deep Cushion's and Long Slumber's cost effects and
/// Long Slumber's nap length are derived queries (`starshard_cost`,
/// `max_nap_length`), and the Lucid Nap's reverie lands with the nap
/// (`deep_nap`), not keys.
/// @spec SIM-DREAM-006
fn loom_effect(loom: crate::state::DreamLoomId, key: Effect) -> f64 {
    use crate::state::DreamLoomId as L;
    match (loom, key) {
        (L::SoftPillow, Effect::HeatEfficiency) => 0.10,
        (L::WarmBlanket, Effect::HeatDissipation) => 0.20,
        (L::DreamCatcher, Effect::DejaVuChance) => 0.25,
        (L::CozyCave, Effect::HeatEfficiency) => 0.15,
        (L::ReverieNet, Effect::ReverieYield) => 0.50,
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

/// One Tradition's contribution to one effect key — the prestige shop table,
/// exactly. Basket capacity, memory-chest ratio, and the Nine Lives ratio are
/// derived queries (counted directly), not effect-key values. @spec SIM-MOVE-012
fn tradition_effect(t: TraditionId, key: Effect) -> f64 {
    use CraftId::{Anthologies, Quills, Stories};
    use Effect::*;
    use TraditionId as T;
    match (t, key) {
        (T::CarefulPacking, GlobalPriceRatioDelta) => 0.01,
        (T::GoldenRatio, GlobalPriceRatioDelta) => 0.016,
        (T::GrandProportion, GlobalPriceRatioDelta) => 0.018,
        (T::VitruvianCat, GlobalPriceRatioDelta) => 0.02,
        (T::Renaissance, GlobalPriceRatioDelta) => 0.0225,
        (T::ElderStories, CraftRatio) => 0.05,
        (T::ElderStories, CraftRatioFor(Quills | Stories | Anthologies)) => 0.25,
        (T::GoodNeighbours, StandingRatio) => 0.10,
        (T::WellRested, KittenGrowthRatio) => 0.75,
        (T::BrightEyed, KittenGrowthRatio) => 1.5,
        (T::TidyPaws, MessAccrualRatio) => -0.10,
        _ => 0.0,
    }
}

#[cfg(test)]
mod every_source {
    use super::index::every_key;
    use super::*;
    use crate::state::{DreamLoomId, IdeaId, Location, Promise, RefinementId, RuleId};

    /// Every source's contribution, walked in full. What a sum over only a
    /// key's own sources has to match bit for bit, so it stays a walk of every
    /// source, and it stays here because the per-source tables are private.
    fn scan(state: &GameState, key: Effect) -> f64 {
        let mut sum = 0.0;
        for (ix, &raw_count) in state.structures.iter().enumerate() {
            if raw_count == 0 {
                continue;
            }
            let s = StructureId::ALL[ix];
            let count = raw_count as f64;
            let def = content::structure_def(s);
            match key {
                Effect::MaxCap(r) => {
                    for (cr, amount) in def.cap_bonuses {
                        if *cr == r {
                            let mut value = amount * count;
                            if def.caps_kite_ratcheted {
                                value *= 1.0
                                    + limited_dr(
                                        content::PORCH_KITE_PER * state.kites_high_water,
                                        content::PORCH_KITE_DR_LIMIT,
                                    );
                            }
                            sum += value;
                        }
                    }
                }
                Effect::CapRatio => sum += def.cap_ratio * count,
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
                Effect::TradeRatio => sum += def.trade_ratio * count,
                Effect::PrismDescentChance => sum += def.prism_descent * count,
                Effect::GlobalProductionRatio => {
                    let mut value = def.global_ratio * count;
                    if def.fuel_scaled {
                        value *= state.fuel_fraction(s)
                            * (1.0 + state.structure_sum(|d| d.fuel_global_boost));
                    }
                    sum += value;
                }
                Effect::OutdoorProductionRatio => sum += def.outdoor_ratio * count,
                Effect::LongSunCapBonus => sum += def.long_sun_cap * count,
                Effect::DejaVuChance => sum += def.deja_vu_chance * count,
                _ => {}
            }
        }
        for r in &state.refinements {
            let def = content::refinement_def(*r);
            let scale = def
                .per_structure
                .map_or(1.0, |s| state.structures[s.ix()] as f64);
            for (k, value) in def.effects {
                if *k == key {
                    sum += value * scale;
                }
            }
        }
        match key {
            Effect::HappinessPoints if state.ideas.contains(&IdeaId::BiscuitMaking) => {
                sum += content::BISCUIT_POINTS;
            }
            Effect::ArrivalChanceOnTop if state.ideas.contains(&IdeaId::NeighborhoodWatch) => {
                sum += content::ARRIVAL_WATCH_BONUS;
            }
            _ => {}
        }
        for rule in &state.rules {
            for (k, value) in content::rule_def(*rule).effects {
                if *k == key {
                    sum += value;
                }
            }
        }
        if !state.rites.is_empty() {
            let altars = state.structures[StructureId::SunbeamAltar.ix()] as f64;
            for rite in &state.rites {
                let n = state.rite_counts[rite.ix()] as f64;
                if n == 0.0 {
                    continue;
                }
                sum += rite_effect(*rite, key, altars) * n;
            }
            if let Effect::MaxCap(ResourceId::Warmth) = key {
                let frescoes = state.rite_counts[RiteId::EverwarmFrescoes.ix()];
                if frescoes > 0 && state.warm_memories > 0.0 {
                    let sun_rooms = state.structures[StructureId::SunRoom.ix()] as f64;
                    sum += unlimited_dr(frescoes as f64, 1.65) * state.warm_memories * sun_rooms;
                }
            }
        }
        for arch in &state.arches_bought {
            sum += arch_effect(*arch, key);
        }
        for t in &state.traditions {
            sum += tradition_effect(*t, key);
        }
        if state.adventure_rewards.iter().any(|&x| x != 0.0) {
            for a in AdventureId::ALL {
                let def = content::adventure_def(a);
                if def.reward_key == key {
                    let pool = state.adventure_rewards[a.ix()];
                    sum += pool.signum() * limited_dr(pool.abs(), def.reward_cap);
                }
            }
        }
        if let Effect::EnergyConsumptionRatio = key {
            if state.active_adventure == Some(AdventureId::FrugalMeadow) {
                sum += content::FRUGAL_MEADOW_TWIST;
            }
        }
        if let Effect::GlobalProductionRatio = key {
            let kept = state.promises.iter().filter(|p| !p.quiet).count() as f64;
            sum += content::PROMISE_GLOBAL_RATIO * kept;
        }
        for l in &state.dream_looms {
            sum += loom_effect(*l, key);
        }
        if let Some(trait_id) = state
            .elder
            .and_then(|ix| state.kittens.get(ix))
            .map(|k| k.trait_id)
        {
            sum += trait_effect(trait_id, key);
        }
        sum
    }

    /// Everything held, `copies` of each structure, half-stocked so a
    /// fuel-scaled structure runs part-throttled.
    fn all_held(copies: u32) -> GameState {
        let mut s = GameState::new(84);
        s.structures = [copies; crate::state::STRUCTURE_COUNT];
        s.ideas.extend(IdeaId::ALL);
        s.refinements.extend(RefinementId::ALL);
        s.rules.extend(RuleId::ALL);
        s.rites.extend(RiteId::ALL);
        s.rite_counts = [2; RiteId::ALL.len()];
        s.arches_bought.extend(ArchId::ALL);
        s.traditions.extend(TraditionId::ALL);
        s.dream_looms.extend(DreamLoomId::ALL);
        s.promises = vec![Promise { quiet: false }, Promise { quiet: true }];
        s.warm_memories = 7.0;
        s.kites_high_water = 40.0;
        s.adventure_rewards = [0.3; AdventureId::ALL.len()];
        s.adventure_rewards[AdventureId::FrugalMeadow.ix()] = -0.2;
        s.active_adventure = Some(AdventureId::FrugalMeadow);
        s.resources = [0.5; crate::state::RESOURCE_COUNT];
        s
    }

    /// A kitten in the chair with this trait.
    fn seat_elder(s: &mut GameState, trait_id: KittenTrait) {
        s.kittens.clear();
        s.kittens.push(crate::state::Kitten {
            name: "Elder".to_string(),
            job: None,
            location: Location::Home,
            arrived_day: 0,
            job_seq: None,
            away_seq: None,
            skills: [0.0; 8],
            exp: 0.0,
            rank: 0,
            trait_id,
            tinkering: None,
            profile: None,
        });
        s.elder = Some(0);
    }

    fn meadows() -> Vec<(String, GameState)> {
        let mut meadows = vec![("a new meadow".to_string(), GameState::new(84))];
        // How many altars a rite's share scales by, none included, since a
        // share that is zero at one count may not be at another.
        for copies in [0, 1, 2, 5, 20] {
            meadows.push((format!("everything held, {copies} copies"), all_held(copies)));
        }
        for trait_id in KittenTrait::ALL {
            let mut s = all_held(3);
            seat_elder(&mut s, trait_id);
            meadows.push((format!("an Elder who is {trait_id:?}"), s));
        }
        // Every other item of each kind, so a key's list holds sources this
        // meadow has and sources it does not, side by side.
        let mut alternate = all_held(3);
        alternate.structures = std::array::from_fn(|ix| (ix % 2) as u32 * 4);
        alternate.refinements = RefinementId::ALL.iter().copied().step_by(2).collect();
        alternate.rules = RuleId::ALL.iter().copied().step_by(2).collect();
        alternate.rites = RiteId::ALL.iter().copied().step_by(2).collect();
        alternate.arches_bought = ArchId::ALL.iter().copied().step_by(2).collect();
        alternate.traditions = TraditionId::ALL.iter().copied().step_by(2).collect();
        alternate.dream_looms = DreamLoomId::ALL.iter().copied().step_by(2).collect();
        meadows.push(("every other item".to_string(), alternate));
        // What only a hand-edited save can say.
        let mut counts_no_set = all_held(3);
        counts_no_set.rites.clear();
        meadows.push(("rite counts without the rites".to_string(), counts_no_set));
        let mut set_no_counts = all_held(3);
        set_no_counts.rite_counts = [0; RiteId::ALL.len()];
        meadows.push(("rites with no counts".to_string(), set_no_counts));
        let mut gone_elder = all_held(3);
        gone_elder.elder = Some(99);
        meadows.push(("an Elder past the roster".to_string(), gone_elder));
        let mut no_memories = all_held(3);
        no_memories.warm_memories = 0.0;
        meadows.push(("no warm memories".to_string(), no_memories));
        let mut no_rewards = all_held(3);
        no_rewards.adventure_rewards = [0.0; AdventureId::ALL.len()];
        no_rewards.active_adventure = None;
        no_rewards.promises.clear();
        meadows.push(("no rewards, twist, or promises".to_string(), no_rewards));
        meadows
    }

    /// @spec SIM-ENG-019, SIM-ENG-084
    #[test]
    fn a_sum_is_bit_identical_to_one_taken_over_every_source() {
        for (what, meadow) in meadows() {
            for key in every_key() {
                let scanned = scan(&meadow, key);
                let summed = meadow.effect(key);
                assert_eq!(
                    summed.to_bits(),
                    scanned.to_bits(),
                    "{key:?} on {what}: {summed} where every source sums to {scanned}"
                );
            }
        }
    }

    /// The comparison above is only as good as its meadows: each kind of source
    /// has to have moved some sum in them, or it would pass with that kind gone.
    /// @spec SIM-ENG-084
    #[test]
    fn the_meadows_compared_reach_every_kind_of_source() {
        let meadow = all_held(3);
        let moved = |without: fn(&mut GameState)| {
            let mut less = meadow.clone();
            without(&mut less);
            every_key()
                .into_iter()
                .any(|key| scan(&meadow, key).to_bits() != scan(&less, key).to_bits())
        };
        assert!(moved(|s| s.structures = [0; crate::state::STRUCTURE_COUNT]));
        assert!(moved(|s| s.refinements.clear()));
        assert!(moved(|s| s.ideas.clear()));
        assert!(moved(|s| s.rules.clear()));
        assert!(moved(|s| s.rites.clear()));
        assert!(moved(|s| s.arches_bought.clear()));
        assert!(moved(|s| s.traditions.clear()));
        assert!(moved(|s| s.adventure_rewards = [0.0; AdventureId::ALL.len()]));
        assert!(moved(|s| s.active_adventure = None));
        assert!(moved(|s| s.promises.clear()));
        assert!(moved(|s| s.dream_looms.clear()));
        let mut seated = meadow.clone();
        seat_elder(&mut seated, KittenTrait::Crafty);
        assert!(every_key()
            .into_iter()
            .any(|key| scan(&seated, key).to_bits() != scan(&meadow, key).to_bits()));
    }
}
