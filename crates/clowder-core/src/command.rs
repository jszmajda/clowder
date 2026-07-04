use crate::content;
use crate::events::Event;
use crate::state::{GameState, IdeaId, JobId, Location, ResourceId, StructureId, EPSILON};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Command {
    GatherCatnip,
    Forage,
    Build(StructureId),
    AssignJob(JobId),
    UnassignJob(JobId),
    ResearchIdea(IdeaId),
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
            Command::Build(s) => self.build(s),
            Command::AssignJob(j) => self.assign(j),
            Command::UnassignJob(j) => self.unassign(j),
            Command::ResearchIdea(i) => self.research(i),
        }
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

    // @spec SIM-CUR-003, SIM-CUR-006
    fn research(&mut self, i: IdeaId) -> Result<Vec<Event>, CommandError> {
        if !self.idea_unlocked(i) {
            return Err(CommandError::Locked);
        }
        if self.ideas.contains(&i) {
            return Err(CommandError::AlreadyResearched);
        }
        let cost = content::idea_def(i).cost;
        self.spend(&[(ResourceId::Curiosity, cost)])?;
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
