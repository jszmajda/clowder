//! The typed effect-key system and shared curve/throttle primitives every
//! content leaf builds on. The engine owns the machinery; leaves own values.

use crate::content;
use crate::state::{GameState, IdeaId, JobId, ResourceId, StructureId};

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
        // 1. Structures (× owned count).
        for s in StructureId::ALL {
            let count = self.structures[s.ix()] as f64;
            if count == 0.0 {
                continue;
            }
            let def = content::structure_def(s);
            match key {
                Effect::MaxCap(r) => {
                    if let Some((cr, amount)) = def.cap_bonus {
                        if cr == r {
                            sum += amount * count;
                        }
                    }
                }
                Effect::HappinessPoints => sum += def.happiness_points * count,
                _ => {}
            }
        }
        // 2. Refinements — arrive with the economy wave.
        // 3. Ideas.
        match key {
            Effect::HappinessPoints if self.ideas.contains(&IdeaId::BiscuitMaking) => {
                sum += content::BISCUIT_POINTS;
            }
            _ => {}
        }
        // 4–10. House rules, rites, arch upgrades, traditions, colony
        // standing, adventure rewards, Elder trait — arrive with their waves.
        sum
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
