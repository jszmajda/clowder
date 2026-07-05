use crate::content;
use crate::events::Event;
use crate::state::{CraftId, GameState, IdeaId, JobId, Location, ResourceId, StructureId, EPSILON};
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
        let batch = 1.0 * (1.0 + self.effect(crate::effects::Effect::CraftRatio));
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
        let mut costs = vec![(ResourceId::Curiosity, def.cost)];
        costs.extend_from_slice(def.gate);
        self.spend(&costs)?;
        self.ideas.insert(i);
        Ok(vec![Event::IdeaResearched { id: i }])
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
