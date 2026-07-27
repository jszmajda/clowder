//! Gating queries: why something cannot be done, and what it would cost.
//!
//! These answers are *rules*, so they live core-side even where a caller could
//! assemble them from public state — a second copy of a rule drifts from the
//! first the moment the rule changes.
//! @spec SIM-ENG-031, SIM-ENG-033, SIM-ENG-036, SIM-ENG-037, SIM-ENG-038, SIM-ENG-039

use crate::content;
use crate::effects::Effect;
use crate::state::{AdventureId, ContentId, GameState, IdeaId, Location, ResourceId, RiteId, StructureId, EPSILON};

pub use crate::query::LockReason;

impl GameState {
    /// Why this content is not actionable, or `None` when it is.
    ///
    /// Resolved one content kind at a time by the same gate → already-owned →
    /// seat/hands → shortfall order the command layer's `CommandError`
    /// precedence follows (SIM-ENG-006), so the reason named here and the
    /// error a command returns can never disagree.
    /// @spec SIM-ENG-031, SIM-ENG-036, SIM-ENG-037
    pub fn lock_reason(&self, id: ContentId) -> Option<LockReason> {
        // An unrevealed item's price and prerequisites are not the player's
        // yet: say only that it is not yet reached. @spec SIM-ENG-037
        if id.is_reveal_tracked() && !self.revealed.contains(&id) {
            return Some(LockReason::NotYetReached);
        }
        match id {
            ContentId::Structure(s) => {
                if !self.structure_unlocked(s) {
                    return Some(LockReason::NotYetReached);
                }
                self.shortfall(&self.unit_cost(id, 0))
            }
            ContentId::Idea(i) => {
                if !self.idea_unlocked(i) {
                    return Some(LockReason::NotYetReached);
                }
                if self.ideas.contains(&i) {
                    return Some(LockReason::AlreadyYours);
                }
                self.shortfall(&self.unit_cost(id, 0))
            }
            ContentId::Craft(c) => {
                if !self.craft_unlocked(c) {
                    return Some(LockReason::Prerequisite);
                }
                self.shortfall(&self.unit_cost(id, 0))
            }
            ContentId::Job(j) => {
                if !self.job_unlocked(j) {
                    return Some(LockReason::Prerequisite);
                }
                let idle = self.kittens.iter().any(|k| k.location == Location::Home && k.job.is_none());
                if !idle {
                    return Some(LockReason::NoOneFree);
                }
                None
            }
            ContentId::Rite(r) => {
                if self.active_adventure == Some(AdventureId::SilentSun) {
                    return Some(LockReason::Prerequisite);
                }
                let def = content::rite_def(r);
                if self.peak_devotion + EPSILON < def.devotion_gate {
                    return Some(LockReason::Prerequisite);
                }
                let count = self.rite_counts[r.ix()];
                let restackable = self.rites.contains(&RiteId::Enlightenment) && content::rite_restackable(r);
                if count >= 1 && !restackable {
                    return Some(LockReason::AlreadyYours);
                }
                self.shortfall(&self.unit_cost(id, 0))
            }
            ContentId::Arch(a) => {
                if !self.ideas.contains(&IdeaId::ParadeRoutes) {
                    return Some(LockReason::Prerequisite);
                }
                if self.arches_bought.contains(&a) {
                    return Some(LockReason::AlreadyYours);
                }
                self.shortfall(&self.unit_cost(id, 0))
            }
            ContentId::Refinement(r) => {
                let def = content::refinement_def(r);
                if !self.ideas.contains(&def.gate) {
                    return Some(LockReason::Prerequisite);
                }
                if self.refinements.contains(&r) {
                    return Some(LockReason::AlreadyYours);
                }
                self.shortfall(&self.unit_cost(id, 0))
            }
            ContentId::Rule(r) => {
                let def = content::rule_def(r);
                if self.rules.contains(&def.blocks) {
                    return Some(LockReason::Prerequisite);
                }
                if self.rules.contains(&r) {
                    return Some(LockReason::AlreadyYours);
                }
                self.shortfall(&self.unit_cost(id, 0))
            }
            ContentId::Tradition(t) => {
                if self.traditions.contains(&t) {
                    return Some(LockReason::AlreadyYours);
                }
                let cost = content::tradition_def(t).cost_wisdom;
                if self.elder_wisdom + EPSILON < cost {
                    // Elder Wisdom is not a `ResourceId`, so — exactly as the
                    // command layer does — the shortfall names no amount.
                    return Some(LockReason::ShortBy { missing: Vec::new() });
                }
                None
            }
            ContentId::DreamLoom(l) => {
                if self.dream_looms.contains(&l) {
                    return Some(LockReason::AlreadyYours);
                }
                self.shortfall(&self.unit_cost(id, 0))
            }
        }
    }

    /// How many times a repeatable action could be taken right now, where a
    /// closed form exists — so a caller can name the count in advance instead
    /// of discovering it by attempting until rejected. `None` where the
    /// action has no closed form. @spec SIM-ENG-033
    ///
    /// Only structures carry the escalating, purely-resource-gated shape a
    /// closed form solves for; every other kind is one-shot (bounded by
    /// `AlreadyYours` after a single purchase) or gated by something other
    /// than stock (a kitten to assign, a seat to fill), so naming a count in
    /// advance would either be trivial or misrepresent a non-resource block.
    pub fn max_affordable(&self, id: ContentId) -> Option<u32> {
        match id {
            ContentId::Structure(s) => Some(self.max_affordable_structure(s)),
            _ => None,
        }
    }

    /// The total cost of taking a repeatable action `n` times from here,
    /// escalation included.
    ///
    /// A count prompt previews this before committing (TUI-NAV-011). The
    /// escalation is a rule, and a caller summing it from single-step costs
    /// would be reimplementing that rule outside the core — the same shadow
    /// math the HLD rejects. @spec SIM-ENG-038
    pub fn cost_of_next(&self, id: ContentId, n: u32) -> Vec<(ResourceId, f64)> {
        let mut total: Vec<(ResourceId, f64)> = Vec::new();
        for step in 0..n {
            for (r, amount) in self.unit_cost(id, step) {
                match total.iter_mut().find(|(tr, _)| *tr == r) {
                    Some((_, t)) => *t += amount,
                    None => total.push((r, amount)),
                }
            }
        }
        total
    }

    /// How far the stock covers a cost, as a fraction in [0, 1] — 1 meaning
    /// affordable now. What separates a partially-affordable row from an
    /// unaffordable one, decided here rather than by a caller inventing a
    /// threshold.
    ///
    /// The worst-supplied component decides, matching the throttle precedent
    /// of SIM-ENG-022: a heap of one ingredient can never stand in for a
    /// shortfall in another. @spec SIM-ENG-039
    pub fn affordability(&self, id: ContentId) -> f64 {
        let cost = self.unit_cost(id, 0);
        if cost.is_empty() {
            return 1.0;
        }
        cost.iter()
            .map(|(r, amount)| if *amount <= EPSILON { 1.0 } else { (self.stock(*r) / amount).clamp(0.0, 1.0) })
            .fold(f64::INFINITY, f64::min)
    }

    /// The reason a cost list is not currently covered, or `None` when it is
    /// — the shared shortfall math behind every `ShortBy` lock reason, kept
    /// identical to `CommandError::InsufficientResources`'s (`spend`,
    /// command.rs) so the two can never disagree.
    fn shortfall(&self, costs: &[(ResourceId, f64)]) -> Option<LockReason> {
        let missing: Vec<(ResourceId, f64)> = costs
            .iter()
            .filter(|(r, amount)| self.stock(*r) + EPSILON < *amount)
            .map(|(r, amount)| (*r, amount - self.stock(*r)))
            .collect();
        if missing.is_empty() {
            None
        } else {
            Some(LockReason::ShortBy { missing })
        }
    }

    /// The cost of one more unit of `id`, `extra` steps past the current
    /// count for kinds whose price escalates with copies already held
    /// (structures, restackable rites) — the shared per-step formula behind
    /// `lock_reason`'s shortfall, `cost_of_next`'s total, and
    /// `affordability`'s fraction.
    fn unit_cost(&self, id: ContentId, extra: u32) -> Vec<(ResourceId, f64)> {
        match id {
            ContentId::Structure(s) => {
                let def = content::structure_def(s);
                let owned = self.structures[s.ix()] as i32 + extra as i32;
                let mult = self.global_price_mult();
                def.base_costs
                    .iter()
                    .map(|(r, base)| (*r, (base * def.ratio.powi(owned) * mult).ceil()))
                    .collect()
            }
            ContentId::Idea(i) => {
                let def = content::idea_def(i);
                let curiosity = def.cost * (1.0 + self.effect(Effect::IdeaCostRatio));
                let mut costs = vec![(ResourceId::Curiosity, curiosity)];
                costs.extend_from_slice(def.gate);
                costs
            }
            ContentId::Craft(c) => content::craft_def(c).inputs.to_vec(),
            ContentId::Job(_) => Vec::new(),
            ContentId::Rite(r) => {
                let def = content::rite_def(r);
                let count = self.rite_counts[r.ix()] + extra;
                let ramp = content::RITE_RESTACK_RATIO.powi(count as i32);
                let discount = 1.0 + self.effect(Effect::RiteCostRatio);
                def.cost.iter().map(|(res, amt)| (*res, amt * ramp * discount)).collect()
            }
            ContentId::Arch(a) => content::arch_def(a).cost.to_vec(),
            ContentId::Refinement(r) => {
                let def = content::refinement_def(r);
                let mut costs = vec![(ResourceId::Curiosity, def.cost)];
                costs.extend_from_slice(def.materials);
                costs
            }
            ContentId::Rule(r) => vec![(ResourceId::Charm, content::rule_def(r).cost_charm)],
            // Elder Wisdom isn't a `ResourceId`, so a Tradition's price has no
            // representation in this vocabulary; `lock_reason` handles its
            // shortfall directly.
            ContentId::Tradition(_) => Vec::new(),
            ContentId::DreamLoom(l) => content::dream_loom_def(l).cost.to_vec(),
        }
    }

    /// The closed-form affordable count for a structure: solve the geometric
    /// series per cost component for the estimated boundary, take the
    /// tightest component, then walk to the exact ceil-rounded boundary
    /// `cost_of_next` draws (a handful of steps at most, since the estimate
    /// starts within a unit or two of it).
    fn max_affordable_structure(&self, s: StructureId) -> u32 {
        let def = content::structure_def(s);
        let owned = self.structures[s.ix()] as i32;
        let mult = self.global_price_mult();
        let mut estimate = u32::MAX;
        for (r, base) in def.base_costs {
            let start = base * def.ratio.powi(owned) * mult;
            let stock = self.stock(*r);
            if start <= EPSILON {
                continue;
            }
            let count = if (def.ratio - 1.0).abs() > EPSILON {
                let inner = 1.0 + stock * (def.ratio - 1.0) / start;
                if inner <= 1.0 {
                    0
                } else {
                    inner.log(def.ratio).floor().max(0.0) as u32
                }
            } else {
                (stock / start).floor().max(0.0) as u32
            };
            estimate = estimate.min(count);
        }
        let n = if estimate == u32::MAX { 0 } else { estimate };
        self.walk_to_boundary(s, n)
    }

    /// Correct a geometric estimate to the exact ceil-rounded boundary that
    /// `cost_of_next` (and thus a real sequence of builds) actually draws.
    fn walk_to_boundary(&self, s: StructureId, mut n: u32) -> u32 {
        let covers = |n: u32| -> bool {
            self.cost_of_next(ContentId::Structure(s), n)
                .iter()
                .all(|(r, amount)| self.stock(*r) + EPSILON >= *amount)
        };
        while n > 0 && !covers(n) {
            n -= 1;
        }
        while covers(n + 1) {
            n += 1;
        }
        n
    }
}
