use crate::content;
use crate::events::Event;
use crate::state::{
    CraftId, GameState, IdeaId, JobId, Location, RefinementId, ResourceId, RuleId, StructureId,
    EPSILON,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Command {
    GatherCatnip,
    Forage,
    /// Idea-gated pebble foraging. @spec SIM-ECO-027
    Rummage,
    Build(StructureId),
    AssignJob(JobId),
    UnassignJob(JobId),
    ResearchIdea(IdeaId),
    /// One batch of a recipe. @spec SIM-ECO-033
    Craft(CraftId),
    /// Bank a pending sky event's yields before its window closes.
    /// @spec SIM-ENG-025
    ObserveSky,
    SetSpeed(f64),
    /// Send squads beyond the meadow: 100 pep each, instant loot.
    /// @spec SIM-EXP-001
    Pounce(u32),
    /// 800 pep + 1500 charm + 40 quills buy 100 days of celebration.
    /// @spec SIM-KIT-025
    HostJamboree,
    /// Spend a kitten's exp (plus shinies) to raise its rank.
    /// @spec SIM-KIT-021
    Promote(usize),
    /// Name a kitten the Elder (replacing any prior one, freely).
    /// @spec SIM-KIT-022
    AppointElder(usize),
    // @spec SIM-CUR-010
    BuyRefinement(RefinementId),
    // @spec SIM-CUR-011
    AdoptRule(RuleId),
    /// Bind the newest unbound Home Tinkerer to a recipe (seat permitting).
    /// @spec SIM-ECO-043
    AssignTinkerer(CraftId),
    /// Release the most recently assigned bound holder of a recipe.
    /// @spec SIM-ECO-043
    UnassignTinkerer(CraftId),
}

/// Why a command was rejected. Listed in precedence order: when several
/// reasons apply, the earliest variant is reported.
// @spec SIM-ENG-006
#[derive(Debug, Clone, PartialEq)]
pub enum CommandError {
    UnknownId,
    Locked,
    AlreadyResearched,
    NoIdleKittens,
    /// Every Tinkerer seat (Toy Workshops) is taken. @spec SIM-ENG-006
    NoSeats,
    NoWorkersAssigned,
    StorageFull,
    InsufficientResources { missing: Vec<(ResourceId, f64)> },
}

impl GameState {
    /// Apply a command between ticks. All-or-nothing: a rejected command has
    /// mutated nothing, RNG included. Commands never draw randomness.
    // @spec SIM-ENG-005
    pub fn apply(&mut self, cmd: Command) -> Result<Vec<Event>, CommandError> {
        match cmd {
            Command::GatherCatnip => self.gather(),
            Command::Forage => self.forage(),
            Command::Rummage => self.rummage(),
            Command::Build(s) => self.build(s),
            Command::AssignJob(j) => self.assign(j),
            Command::UnassignJob(j) => self.unassign(j),
            Command::ResearchIdea(i) => self.research(i),
            Command::Craft(c) => self.craft(c),
            Command::ObserveSky => self.observe_sky(),
            Command::SetSpeed(v) => self.set_speed(v),
            Command::Pounce(squads) => self.pounce(squads),
            Command::HostJamboree => self.host_jamboree(),
            Command::Promote(ix) => self.promote(ix),
            Command::AppointElder(ix) => self.appoint_elder(ix),
            Command::BuyRefinement(r) => self.buy_refinement(r),
            Command::AdoptRule(r) => self.adopt_rule(r),
            Command::AssignTinkerer(c) => self.assign_tinkerer(c),
            Command::UnassignTinkerer(c) => self.unassign_tinkerer(c),
        }
    }

    /// Always legal; clamps silently. Speed is boundary cadence, not a rule.
    // @spec SIM-ENG-024
    fn set_speed(&mut self, v: f64) -> Result<Vec<Event>, CommandError> {
        let clamped = if v.is_finite() {
            v.clamp(crate::state::SPEED_MIN, crate::state::SPEED_MAX)
        } else {
            1.0
        };
        self.speed = clamped;
        Ok(vec![Event::SpeedChanged { speed: clamped }])
    }

    // @spec SIM-ECO-004, SIM-ECO-006
    fn gather(&mut self) -> Result<Vec<Event>, CommandError> {
        let cap = self.cap(ResourceId::Catnip);
        if self.stock(ResourceId::Catnip) + EPSILON >= cap {
            return Err(CommandError::StorageFull);
        }
        let amount = if self.ideas.contains(&IdeaId::SelectiveHearing) {
            content::GATHER_YIELD * 2.0
        } else {
            content::GATHER_YIELD
        };
        let ix = ResourceId::Catnip.ix();
        self.resources[ix] = (self.resources[ix] + amount).min(cap);
        Ok(Vec::new())
    }

    // @spec SIM-ECO-005, SIM-ECO-006
    fn forage(&mut self) -> Result<Vec<Event>, CommandError> {
        let twig_cap = self.cap(ResourceId::Twigs);
        if self.stock(ResourceId::Twigs) + EPSILON >= twig_cap {
            return Err(CommandError::StorageFull);
        }
        let catnip = self.stock(ResourceId::Catnip);
        if catnip + EPSILON < content::FORAGE_COST {
            return Err(CommandError::InsufficientResources {
                missing: vec![(ResourceId::Catnip, content::FORAGE_COST - catnip)],
            });
        }
        self.resources[ResourceId::Catnip.ix()] = (catnip - content::FORAGE_COST).max(0.0);
        let twigs = ResourceId::Twigs.ix();
        self.resources[twigs] = (self.resources[twigs] + content::FORAGE_YIELD).min(twig_cap);
        Ok(Vec::new())
    }

    /// Rummage the couch: Locked before its idea, then +0.5 pebbles under the
    /// same StorageFull rule as every manual action.
    // @spec SIM-ECO-027
    fn rummage(&mut self) -> Result<Vec<Event>, CommandError> {
        if !self.ideas.contains(&IdeaId::CouchSpelunking) {
            return Err(CommandError::Locked);
        }
        let cap = self.cap(ResourceId::Pebbles);
        if self.stock(ResourceId::Pebbles) + EPSILON >= cap {
            return Err(CommandError::StorageFull);
        }
        let ix = ResourceId::Pebbles.ix();
        self.resources[ix] = (self.resources[ix] + content::RUMMAGE_YIELD).min(cap);
        Ok(Vec::new())
    }

    /// One craft batch: all-or-nothing inputs, craft-ratio-boosted output.
    // @spec SIM-ECO-033
    fn craft(&mut self, c: CraftId) -> Result<Vec<Event>, CommandError> {
        if !self.craft_unlocked(c) {
            return Err(CommandError::Locked);
        }
        let def = content::craft_def(c);
        self.spend(def.inputs)?;
        let output = content::craft_output(c);
        let batch = 1.0 * (1.0 + self.craft_ratio_for(c));
        let ix = output.ix();
        self.resources[ix] = (self.resources[ix] + batch).min(self.cap(output));
        Ok(Vec::new())
    }

    /// Observe a pending, unexpired sky event for its yields; anything else
    /// (nothing pending, or a faded event) is Locked and a perfect no-op.
    // @spec SIM-ENG-025
    fn observe_sky(&mut self) -> Result<Vec<Event>, CommandError> {
        match self.pending_sky_event {
            Some(expiry) if self.ticks < expiry => {
                self.grant_sky_observation();
                self.pending_sky_event = None;
                Ok(Vec::new())
            }
            _ => Err(CommandError::Locked),
        }
    }

    // @spec SIM-ECO-007
    fn build(&mut self, s: StructureId) -> Result<Vec<Event>, CommandError> {
        if !self.structure_unlocked(s) {
            return Err(CommandError::Locked);
        }
        let costs = self.next_cost(s);
        self.spend(&costs)?;
        self.structures[s.ix()] += 1;
        Ok(vec![Event::StructureBuilt { id: s, count: self.structures[s.ix()] }])
    }

    // @spec SIM-KIT-008
    fn assign(&mut self, j: JobId) -> Result<Vec<Event>, CommandError> {
        if !self.job_unlocked(j) {
            return Err(CommandError::Locked);
        }
        // Most recently arrived idle Home kitten: the roster is append-ordered
        // by arrival, so scan from the back.
        let Some(ix) = self
            .kittens
            .iter()
            .rposition(|k| k.location == Location::Home && k.job.is_none())
        else {
            return Err(CommandError::NoIdleKittens);
        };
        let seq = self.next_seq;
        self.next_seq += 1;
        let kitten = &mut self.kittens[ix];
        kitten.job = Some(j);
        kitten.job_seq = Some(seq);
        Ok(vec![Event::JobAssigned { name: kitten.name.clone(), job: j }])
    }

    // @spec SIM-KIT-009
    fn unassign(&mut self, j: JobId) -> Result<Vec<Event>, CommandError> {
        let Some(ix) = self
            .kittens
            .iter()
            .enumerate()
            .filter(|(_, k)| k.location == Location::Home && k.job == Some(j))
            .max_by_key(|(_, k)| k.job_seq)
            .map(|(ix, _)| ix)
        else {
            return Err(CommandError::NoWorkersAssigned);
        };
        let kitten = &mut self.kittens[ix];
        kitten.job = None;
        kitten.job_seq = None;
        // A Tinkerer leaving the job releases its recipe binding (and seat).
        // @spec SIM-ECO-043
        kitten.tinkering = None;
        Ok(vec![Event::JobUnassigned { name: kitten.name.clone(), job: j }])
    }

    /// Research spends curiosity and any gate-craft components together,
    /// all-or-nothing.
    // @spec SIM-CUR-003, SIM-CUR-006, SIM-CUR-007
    fn research(&mut self, i: IdeaId) -> Result<Vec<Event>, CommandError> {
        if !self.idea_unlocked(i) {
            return Err(CommandError::Locked);
        }
        if self.ideas.contains(&i) {
            return Err(CommandError::AlreadyResearched);
        }
        let def = content::idea_def(i);
        // The Curious Elder discounts the curiosity price at spend time;
        // gate crafts stay full price. @spec SIM-KIT-024
        let curiosity = def.cost * (1.0 + self.effect(crate::effects::Effect::IdeaCostRatio));
        let mut costs = vec![(ResourceId::Curiosity, curiosity)];
        costs.extend_from_slice(def.gate);
        self.spend(&costs)?;
        self.ideas.insert(i);
        Ok(vec![Event::IdeaResearched { id: i }])
    }

    /// Squads scamper beyond the meadow and come back at once with loot:
    /// affordability is checked before any draw (a rejected pounce leaves
    /// the RNG untouched), then yields resolve in the fixed draw order —
    /// feathers, bells, shinies (cap-clamped), unicorns.
    // @spec SIM-EXP-001, SIM-EXP-002, SIM-EXP-003, SIM-EXP-005, SIM-EXP-006
    fn pounce(&mut self, squads: u32) -> Result<Vec<Event>, CommandError> {
        if !self.ideas.contains(&IdeaId::PouncingPractice) {
            return Err(CommandError::Locked);
        }
        if squads == 0 {
            return Err(CommandError::UnknownId); // a zero-squad pounce is no pounce
        }
        self.spend(&[(ResourceId::Pep, content::POUNCE_PEP_PER_SQUAD * squads as f64)])?;

        // @spec SIM-EXP-004
        let ratio = self.effect(crate::effects::Effect::PounceRatio);
        let feathers =
            (content::POUNCE_FEATHERS_BASE + content::POUNCE_FEATHERS_PER_RATIO * ratio)
                * self.irwin_hall(squads);
        let bell_hits = self.binomial(
            squads,
            content::POUNCE_BELLS_CHANCE + content::POUNCE_BELLS_CHANCE_PER_RATIO * ratio,
        );
        let bells = (content::POUNCE_BELLS_BASE + content::POUNCE_BELLS_PER_RATIO * ratio)
            * self.irwin_hall(bell_hits);
        let shiny_hits = self.binomial(squads, content::POUNCE_SHINIES_CHANCE);
        let shinies_rolled = (content::POUNCE_SHINIES_BASE
            + content::POUNCE_SHINIES_PER_RATIO * ratio)
            * self.irwin_hall(shiny_hits);
        let unicorns = self.binomial(squads, content::POUNCE_UNICORN_CHANCE);

        self.resources[ResourceId::Feathers.ix()] += feathers;
        self.resources[ResourceId::Bells.ix()] += bells;
        // Shinies clamp to cap at yield time; the event reports the amount
        // that actually landed.
        let cap = self.cap(ResourceId::Shinies);
        let held = self.stock(ResourceId::Shinies);
        let shinies = shinies_rolled.min((cap - held).max(0.0));
        self.resources[ResourceId::Shinies.ix()] = held + shinies;
        self.resources[ResourceId::Unicorns.ix()] += unicorns as f64;

        Ok(vec![Event::PounceReturned { squads, feathers, bells, shinies, unicorns }])
    }

    /// Irwin–Hall: the sum of n uniform draws (mean n/2, bell-shaped).
    // @spec SIM-EXP-002
    fn irwin_hall(&mut self, n: u32) -> f64 {
        (0..n).map(|_| self.rng.uniform()).sum()
    }

    /// Binomial(n, p) as n Bernoulli draws — the schedule stays fixed given n.
    fn binomial(&mut self, n: u32, p: f64) -> u32 {
        (0..n).filter(|_| self.rng.chance(p)).count() as u32
    }

    /// Host a jamboree: gated by its idea, one at a time ("already
    /// celebrating" is Locked), all-or-nothing bill, 100 days of festivity.
    // @spec SIM-KIT-025
    fn host_jamboree(&mut self) -> Result<Vec<Event>, CommandError> {
        if !self.ideas.contains(&IdeaId::TeaCeremonies) || self.jamboree_active() {
            return Err(CommandError::Locked);
        }
        self.spend(&[
            (ResourceId::Pep, content::JAMBOREE_PEP),
            (ResourceId::Charm, content::JAMBOREE_CHARM),
            (ResourceId::Quills, content::JAMBOREE_QUILLS),
        ])?;
        self.jamboree_until = self.ticks + content::JAMBOREE_TICKS;
        Ok(vec![Event::JamboreeStarted { until_tick: self.jamboree_until }])
    }

    /// Promote a kitten: 500 × 1.75^rank exp from its own pool plus
    /// 25 × (rank+1) shinies, both all-or-nothing.
    // @spec SIM-KIT-021
    fn promote(&mut self, ix: usize) -> Result<Vec<Event>, CommandError> {
        let Some(kitten) = self.kittens.get(ix) else {
            return Err(CommandError::UnknownId);
        };
        let rank = kitten.rank;
        let exp_cost = content::PROMOTE_EXP_BASE * content::PROMOTE_EXP_RATIO.powi(rank as i32);
        let shinies_cost = content::PROMOTE_SHINIES_STEP * (rank as f64 + 1.0);
        let shinies = self.stock(ResourceId::Shinies);
        let mut missing = Vec::new();
        if shinies + EPSILON < shinies_cost {
            missing.push((ResourceId::Shinies, shinies_cost - shinies));
        }
        if kitten.exp + EPSILON < exp_cost || !missing.is_empty() {
            return Err(CommandError::InsufficientResources { missing });
        }
        self.resources[ResourceId::Shinies.ix()] = (shinies - shinies_cost).max(0.0);
        let kitten = &mut self.kittens[ix];
        kitten.exp = (kitten.exp - exp_cost).max(0.0);
        kitten.rank += 1;
        Ok(vec![Event::KittenPromoted { name: kitten.name.clone(), rank: kitten.rank }])
    }

    /// Name a kitten the Elder. Free, replaceable anytime; the game itself
    /// never removes one.
    // @spec SIM-KIT-022
    fn appoint_elder(&mut self, ix: usize) -> Result<Vec<Event>, CommandError> {
        let Some(kitten) = self.kittens.get(ix) else {
            return Err(CommandError::UnknownId);
        };
        self.elder = Some(ix);
        Ok(vec![Event::ElderAppointed { name: kitten.name.clone() }])
    }

    /// Buy a refinement: Locked before its gate idea, purchasable once,
    /// curiosity + materials spent together, all-or-nothing, permanent.
    // @spec SIM-CUR-010
    fn buy_refinement(&mut self, r: RefinementId) -> Result<Vec<Event>, CommandError> {
        let def = content::refinement_def(r);
        if !self.ideas.contains(&def.gate) {
            return Err(CommandError::Locked);
        }
        if self.refinements.contains(&r) {
            return Err(CommandError::AlreadyResearched);
        }
        let mut costs = vec![(ResourceId::Curiosity, def.cost)];
        costs.extend_from_slice(def.materials);
        self.spend(&costs)?;
        self.refinements.insert(r);
        Ok(vec![Event::RefinementBought { id: r }])
    }

    /// Adopt a house rule: charm all-or-nothing; the sibling rule is
    /// permanently Locked for this meadow, in both directions.
    // @spec SIM-CUR-011
    fn adopt_rule(&mut self, r: RuleId) -> Result<Vec<Event>, CommandError> {
        let def = content::rule_def(r);
        if self.rules.contains(&def.blocks) {
            return Err(CommandError::Locked);
        }
        if self.rules.contains(&r) {
            return Err(CommandError::AlreadyResearched);
        }
        self.spend(&[(ResourceId::Charm, def.cost_charm)])?;
        self.rules.insert(r);
        Ok(vec![Event::RuleAdopted { id: r }])
    }

    /// Bind the most recently arrived unbound Home Tinkerer to a recipe.
    /// Error precedence: Locked (the recipe), NoIdleKittens (no unbound Home
    /// Tinkerer), NoSeats (bindings would exceed the Toy Workshop count —
    /// an absent Tinkerer still holds its seat).
    // @spec SIM-ECO-043, SIM-ENG-006
    fn assign_tinkerer(&mut self, c: CraftId) -> Result<Vec<Event>, CommandError> {
        if !self.craft_unlocked(c) {
            return Err(CommandError::Locked);
        }
        let Some(ix) = self.kittens.iter().rposition(|k| {
            k.location == Location::Home && k.job == Some(JobId::Tinkerer) && k.tinkering.is_none()
        }) else {
            return Err(CommandError::NoIdleKittens);
        };
        let seats = self.structures[StructureId::ToyWorkshop.ix()];
        let bound = self.kittens.iter().filter(|k| k.tinkering.is_some()).count() as u32;
        if bound + 1 > seats {
            return Err(CommandError::NoSeats);
        }
        self.kittens[ix].tinkering = Some(c);
        Ok(Vec::new())
    }

    /// Release the bound Home holder of this recipe with the highest
    /// job_seq — the later assignee unbinds first.
    // @spec SIM-ECO-043
    fn unassign_tinkerer(&mut self, c: CraftId) -> Result<Vec<Event>, CommandError> {
        let Some(ix) = self
            .kittens
            .iter()
            .enumerate()
            .filter(|(_, k)| k.location == Location::Home && k.tinkering == Some(c))
            .max_by_key(|(_, k)| k.job_seq)
            .map(|(ix, _)| ix)
        else {
            return Err(CommandError::NoWorkersAssigned);
        };
        self.kittens[ix].tinkering = None;
        Ok(Vec::new())
    }

    /// All-or-nothing spend with epsilon-tolerant affordability.
    // @spec SIM-ECO-022
    fn spend(&mut self, costs: &[(ResourceId, f64)]) -> Result<(), CommandError> {
        let missing: Vec<(ResourceId, f64)> = costs
            .iter()
            .filter(|(r, amount)| self.stock(*r) + EPSILON < *amount)
            .map(|(r, amount)| (*r, amount - self.stock(*r)))
            .collect();
        if !missing.is_empty() {
            return Err(CommandError::InsufficientResources { missing });
        }
        for (r, amount) in costs {
            let ix = r.ix();
            self.resources[ix] = (self.resources[ix] - amount).max(0.0);
        }
        Ok(())
    }
}
