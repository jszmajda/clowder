//! For each effect key, the sources that can carry it. A tick asks for some two
//! hundred keys, most carried by few sources or none, and walking every source
//! for each was most of what a late meadow's tick cost.
//!
//! Each list is derived once from the tables that state what a source gives, and
//! says only who can contribute: a contribution is read from the state at the
//! call (`GameState::contribution`), so nothing here goes stale. A source left
//! off a key's list is one that adds zero to it, and adding zero never moves a
//! float sum, so the sum is bit-identical to one over every source.

use std::sync::OnceLock;

use super::{arch_effect, loom_effect, rite_effect, tradition_effect, trait_effect, Effect};
use crate::content;
use crate::state::{
    AdventureId, ArchId, ColonyId, CraftId, DreamLoomId, IdeaId, JobId, KittenTrait, RefinementId,
    ResourceId, RiteId, RuleId, Season, StructureId, TraditionId,
};

/// One source of an effect key, with what its definition fixes about the key.
#[derive(Debug, Clone, Copy)]
pub(super) enum Source {
    Structure {
        structure: StructureId,
        per_copy: f64,
        kite_ratcheted: bool,
        fuel_scaled: bool,
    },
    Refinement {
        refinement: RefinementId,
        value: f64,
        per_structure: Option<StructureId>,
    },
    Idea {
        idea: IdeaId,
        value: f64,
    },
    Rule {
        rule: RuleId,
        value: f64,
    },
    Rite(RiteId),
    Frescoes,
    Arch(ArchId),
    Tradition(TraditionId),
    AdventureReward {
        adventure: AdventureId,
        cap: f64,
    },
    FrugalTwist,
    KeptPromises,
    Loom(DreamLoomId),
    ElderTrait,
}

const RESOURCES: usize = ResourceId::ALL.len();
const STRUCTURES: usize = StructureId::ALL.len();

/// The keys that name nothing come first, then each family that names something.
const PLAIN_KEYS: usize = 36;
const RESOURCE_RATIO: usize = PLAIN_KEYS;
const STRUCTURE_RATIO: usize = RESOURCE_RATIO + RESOURCES;
const JOB_RATIO: usize = STRUCTURE_RATIO + STRUCTURES;
const MAX_CAP: usize = JOB_RATIO + JobId::ALL.len();
const CRAFT_RATIO_FOR: usize = MAX_CAP + RESOURCES;
const STRUCTURE_OUTPUT_ADD: usize = CRAFT_RATIO_FOR + CraftId::ALL.len();
const STRUCTURE_MESS_DELTA: usize = STRUCTURE_OUTPUT_ADD + STRUCTURES * RESOURCES;
const MAX_CAP_RATIO: usize = STRUCTURE_MESS_DELTA + STRUCTURES;
const STANDING_FOR: usize = MAX_CAP_RATIO + RESOURCES;
const SEASON_RESOURCE_RATIO: usize = STANDING_FOR + ColonyId::ALL.len();
const KEY_COUNT: usize = SEASON_RESOURCE_RATIO + Season::ALL.len() * RESOURCES;

/// A key's own place in the index. Exhaustive, so a new key cannot be left out.
#[inline]
fn slot(key: Effect) -> usize {
    use Effect::*;
    match key {
        GlobalProductionRatio => 0,
        CraftRatio => 1,
        HappinessPoints => 2,
        LuxuryDemandRatio => 3,
        CapRatio => 4,
        PounceRatio => 5,
        TinkererSpeed => 6,
        SkillXpRatio => 7,
        JamboreeRatio => 8,
        JamboreeArrivalRatio => 9,
        ArrivalChanceInCap => 10,
        ArrivalChanceOnTop => 11,
        IdeaCostRatio => 12,
        TradeRatio => 13,
        RiteCostRatio => 14,
        SkyEventChanceRatio => 15,
        AutoObserveChanceRatio => 16,
        PrismDescentChance => 17,
        RainbowShowerChance => 18,
        UnicornsRatio => 19,
        StarshardRefine => 20,
        EchoRefine => 21,
        ParadeRatio => 22,
        DreamBoost => 23,
        GlobalPriceRatioDelta => 24,
        KittenGrowthRatio => 25,
        StandingRatio => 26,
        MasterSkillMultiplier => 27,
        EnergyConsumptionRatio => 28,
        MessAccrualRatio => 29,
        OutdoorProductionRatio => 30,
        LongSunCapBonus => 31,
        DejaVuChance => 32,
        ReverieYield => 33,
        HeatEfficiency => 34,
        HeatDissipation => 35,
        ResourceRatio(r) => RESOURCE_RATIO + r.ix(),
        StructureRatio(s) => STRUCTURE_RATIO + s.ix(),
        JobRatio(j) => JOB_RATIO + j as usize,
        MaxCap(r) => MAX_CAP + r.ix(),
        CraftRatioFor(c) => CRAFT_RATIO_FOR + c as usize,
        StructureOutputAdd(s, r) => STRUCTURE_OUTPUT_ADD + s.ix() * RESOURCES + r.ix(),
        StructureMessDelta(s) => STRUCTURE_MESS_DELTA + s.ix(),
        MaxCapRatio(r) => MAX_CAP_RATIO + r.ix(),
        StandingFor(c) => STANDING_FOR + c.ix(),
        SeasonResourceRatio(season, r) => {
            SEASON_RESOURCE_RATIO + season as usize * RESOURCES + r.ix()
        }
    }
}

/// Every key the vocabulary can spell.
pub(super) fn every_key() -> Vec<Effect> {
    use Effect::*;
    let mut keys = vec![
        GlobalProductionRatio,
        CraftRatio,
        HappinessPoints,
        LuxuryDemandRatio,
        CapRatio,
        PounceRatio,
        TinkererSpeed,
        SkillXpRatio,
        JamboreeRatio,
        JamboreeArrivalRatio,
        ArrivalChanceInCap,
        ArrivalChanceOnTop,
        IdeaCostRatio,
        TradeRatio,
        RiteCostRatio,
        SkyEventChanceRatio,
        AutoObserveChanceRatio,
        PrismDescentChance,
        RainbowShowerChance,
        UnicornsRatio,
        StarshardRefine,
        EchoRefine,
        ParadeRatio,
        DreamBoost,
        GlobalPriceRatioDelta,
        KittenGrowthRatio,
        StandingRatio,
        MasterSkillMultiplier,
        EnergyConsumptionRatio,
        MessAccrualRatio,
        OutdoorProductionRatio,
        LongSunCapBonus,
        DejaVuChance,
        ReverieYield,
        HeatEfficiency,
        HeatDissipation,
    ];
    for r in ResourceId::ALL {
        keys.extend([ResourceRatio(r), MaxCap(r), MaxCapRatio(r)]);
        keys.extend(Season::ALL.map(|season| SeasonResourceRatio(season, r)));
        keys.extend(StructureId::ALL.map(|s| StructureOutputAdd(s, r)));
    }
    for s in StructureId::ALL {
        keys.extend([StructureRatio(s), StructureMessDelta(s)]);
    }
    keys.extend(JobId::ALL.map(JobRatio));
    keys.extend(CraftId::ALL.map(CraftRatioFor));
    keys.extend(ColonyId::ALL.map(StandingFor));
    keys
}

/// The sources that can carry a key, in the fixed source order.
// @spec SIM-ENG-084
#[inline]
pub(super) fn sources(key: Effect) -> &'static [Source] {
    static INDEX: OnceLock<Vec<Box<[Source]>>> = OnceLock::new();
    &INDEX.get_or_init(build)[slot(key)]
}

/// Each key's list, class by class in the fixed source order (SIM-ENG-019), and
/// within a class in the order its held set walks, which is its items' own.
// @spec SIM-ENG-019, SIM-ENG-084
fn build() -> Vec<Box<[Source]>> {
    let mut index: Vec<Vec<Source>> = vec![Vec::new(); KEY_COUNT];
    for structure in StructureId::ALL {
        structure_sources(structure, &mut index);
    }
    let mut refinements = RefinementId::ALL.to_vec();
    refinements.sort();
    for refinement in refinements {
        let def = content::refinement_def(refinement);
        for (key, value) in def.effects {
            index[slot(*key)].push(Source::Refinement {
                refinement,
                value: *value,
                per_structure: def.per_structure,
            });
        }
    }
    for (key, idea, value) in [
        (
            Effect::HappinessPoints,
            IdeaId::BiscuitMaking,
            content::BISCUIT_POINTS,
        ),
        (
            Effect::ArrivalChanceOnTop,
            IdeaId::NeighborhoodWatch,
            content::ARRIVAL_WATCH_BONUS,
        ),
    ] {
        index[slot(key)].push(Source::Idea { idea, value });
    }
    let mut rules = RuleId::ALL.to_vec();
    rules.sort();
    for rule in rules {
        for (key, value) in content::rule_def(rule).effects {
            index[slot(*key)].push(Source::Rule {
                rule,
                value: *value,
            });
        }
    }

    // What these classes give is stated by a function of the key rather than a
    // list on a definition, so each is asked about every key.
    let mut rites = RiteId::ALL.to_vec();
    rites.sort();
    let mut arches = ArchId::ALL.to_vec();
    arches.sort();
    let mut traditions = TraditionId::ALL.to_vec();
    traditions.sort();
    let mut looms = DreamLoomId::ALL.to_vec();
    looms.sort();
    for key in every_key() {
        let list = &mut index[slot(key)];
        // A share scaled by altars is non-zero at one altar.
        list.extend(
            rites
                .iter()
                .filter(|rite| rite_effect(**rite, key, 1.0) != 0.0)
                .map(|rite| Source::Rite(*rite)),
        );
        if key == Effect::MaxCap(ResourceId::Warmth) {
            list.push(Source::Frescoes);
        }
        list.extend(
            arches
                .iter()
                .filter(|arch| arch_effect(**arch, key) != 0.0)
                .map(|arch| Source::Arch(*arch)),
        );
        list.extend(
            traditions
                .iter()
                .filter(|tradition| tradition_effect(**tradition, key) != 0.0)
                .map(|tradition| Source::Tradition(*tradition)),
        );
        for adventure in AdventureId::ALL {
            let def = content::adventure_def(adventure);
            if def.reward_key == key {
                list.push(Source::AdventureReward {
                    adventure,
                    cap: def.reward_cap,
                });
            }
        }
        if key == Effect::EnergyConsumptionRatio {
            list.push(Source::FrugalTwist);
        }
        if key == Effect::GlobalProductionRatio {
            list.push(Source::KeptPromises);
        }
        list.extend(
            looms
                .iter()
                .filter(|loom| loom_effect(**loom, key) != 0.0)
                .map(|loom| Source::Loom(*loom)),
        );
        if KittenTrait::ALL
            .iter()
            .any(|trait_id| trait_effect(*trait_id, key) != 0.0)
        {
            list.push(Source::ElderTrait);
        }
    }
    index.into_iter().map(Vec::into_boxed_slice).collect()
}

/// The keys one structure's definition gives something to.
fn structure_sources(structure: StructureId, index: &mut [Vec<Source>]) {
    let def = content::structure_def(structure);
    let mut carry = |key: Effect, per_copy: f64, kite_ratcheted: bool, fuel_scaled: bool| {
        if per_copy != 0.0 {
            index[slot(key)].push(Source::Structure {
                structure,
                per_copy,
                kite_ratcheted,
                fuel_scaled,
            });
        }
    };
    for (r, amount) in def.cap_bonuses {
        // @spec SIM-ECO-031
        carry(Effect::MaxCap(*r), *amount, def.caps_kite_ratcheted, false);
    }
    for (r, ratio) in def.resource_ratios {
        carry(Effect::ResourceRatio(*r), *ratio, false, false);
    }
    // @spec SIM-ECO-032
    carry(
        Effect::GlobalProductionRatio,
        def.global_ratio,
        false,
        def.fuel_scaled,
    );
    // @spec SIM-SKY-011
    carry(Effect::CapRatio, def.cap_ratio, false, false);
    // @spec SIM-ECO-053
    carry(Effect::TradeRatio, def.trade_ratio, false, false);
    // @spec SIM-UNI-006
    carry(Effect::PrismDescentChance, def.prism_descent, false, false);
    // @spec SIM-SKY-004, SIM-SKY-009
    carry(
        Effect::OutdoorProductionRatio,
        def.outdoor_ratio,
        false,
        false,
    );
    carry(Effect::LongSunCapBonus, def.long_sun_cap, false, false);
    // @spec SIM-DREAM-004, SIM-DREAM-013
    carry(Effect::DejaVuChance, def.deja_vu_chance, false, false);
    for (key, per_copy) in [
        (Effect::HappinessPoints, def.happiness_points),
        (Effect::JamboreeRatio, def.jamboree_ratio),
        (Effect::JamboreeArrivalRatio, def.jamboree_arrival),
        (Effect::CraftRatio, def.craft_ratio),
        (Effect::LuxuryDemandRatio, def.luxury_demand),
    ] {
        carry(key, per_copy, false, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// Every key has a place of its own and none is left over, so the walk that
    /// builds the index reaches every list a sum can read.
    /// @spec SIM-ENG-084
    #[test]
    fn every_key_has_its_own_place_in_the_index() {
        let keys = every_key();
        let slots: BTreeSet<usize> = keys.iter().map(|key| slot(*key)).collect();
        assert_eq!(slots.len(), keys.len(), "no two keys share a place");
        assert_eq!(keys.len(), KEY_COUNT, "and every place has a key");
        assert_eq!(slots.last().copied(), Some(KEY_COUNT - 1));
    }

    /// Where each source stands in the fixed order: its class, then its own
    /// place in that class. Two copies of a place are one structure naming one
    /// key twice.
    fn place(source: &Source) -> (u8, usize) {
        match *source {
            Source::Structure { structure, .. } => (0, structure.ix()),
            Source::Refinement { refinement, .. } => (1, refinement as usize),
            Source::Idea { idea, .. } => (2, idea as usize),
            Source::Rule { rule, .. } => (3, rule as usize),
            Source::Rite(rite) => (4, rite.ix()),
            Source::Frescoes => (5, 0),
            Source::Arch(arch) => (6, arch.ix()),
            Source::Tradition(tradition) => (7, tradition.ix()),
            Source::AdventureReward { adventure, .. } => (8, adventure.ix()),
            Source::FrugalTwist => (9, 0),
            Source::KeptPromises => (10, 0),
            Source::Loom(loom) => (11, loom.ix()),
            Source::ElderTrait => (12, 0),
        }
    }

    /// Order is what keeps a float sum the same sum, and a wrong order can round
    /// the same way on any meadow a test happens to try, so the order is read
    /// off the lists themselves rather than left to a sum to show.
    /// @spec SIM-ENG-019, SIM-ENG-084
    #[test]
    fn each_keys_sources_stand_in_the_fixed_order() {
        for key in every_key() {
            let list = sources(key);
            for pair in list.windows(2) {
                assert!(
                    place(&pair[0]) <= place(&pair[1]),
                    "{key:?} lists {:?} ahead of {:?}",
                    pair[0],
                    pair[1]
                );
            }
        }
    }

    /// Most keys carry nothing, so the lists save most of a walk over every
    /// source; and the keys that do are never left with more than their own.
    /// @spec SIM-ENG-084
    #[test]
    fn a_key_lists_only_the_sources_that_can_carry_it() {
        assert!(sources(Effect::DreamBoost).is_empty(), "a dormant key");
        assert!(sources(Effect::StandingFor(ColonyId::ALL[0])).is_empty());
        for key in every_key() {
            for source in sources(key) {
                let carries = match *source {
                    Source::Structure { per_copy, .. } => per_copy != 0.0,
                    Source::Refinement { refinement, .. } => content::refinement_def(refinement)
                        .effects
                        .iter()
                        .any(|(k, _)| *k == key),
                    Source::Rule { rule, .. } => content::rule_def(rule)
                        .effects
                        .iter()
                        .any(|(k, _)| *k == key),
                    Source::Rite(rite) => rite_effect(rite, key, 1.0) != 0.0,
                    Source::Arch(arch) => arch_effect(arch, key) != 0.0,
                    Source::Tradition(t) => tradition_effect(t, key) != 0.0,
                    Source::Loom(loom) => loom_effect(loom, key) != 0.0,
                    Source::AdventureReward { adventure, .. } => {
                        content::adventure_def(adventure).reward_key == key
                    }
                    _ => true,
                };
                assert!(
                    carries,
                    "{source:?} is listed for {key:?}, which it gives nothing"
                );
            }
        }
    }
}
