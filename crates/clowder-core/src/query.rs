//! Read-only derived queries: the answers that are *rules*.
//!
//! The shell renders; the core rules. Anything whose answer is a rule is
//! answered here even where a caller could assemble it from public state — a
//! second copy of a rule drifts from the first the moment the rule changes.
//! @spec SIM-ENG-030, SIM-ENG-031, SIM-ENG-032, SIM-ENG-033, SIM-ENG-062,
//! SIM-ENG-069

use std::collections::BTreeMap;

use crate::state::{
    ArchId, CraftId, GameState, JobId, KittenTrait, Location, ResourceId, Season, StructureId,
    RESOURCE_COUNT,
};

/// What a stock move is made for: the one thing a flow is credited to.
///
/// The variant order, and each id's own order within its variant, is the
/// order the flows of a resource are listed in — structures, jobs, the
/// kittens, crafts, arch upgrades, each kind in its table's order — so the
/// derived `Ord` is load-bearing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FlowSource {
    Structure(StructureId),
    Job(JobId),
    /// The kittens themselves: their appetite and the luxuries they use.
    Kittens,
    /// The tinkerers bound to a recipe.
    Craft(CraftId),
    Arch(ArchId),
}

/// The stock moves of one run of the resource phases, summed per source. Kept
/// only by the flows query; a tick passes none.
#[derive(Debug, Default)]
pub(crate) struct FlowLedger {
    by_source: BTreeMap<FlowSource, [f64; RESOURCE_COUNT]>,
}

impl FlowLedger {
    pub(crate) fn credit(&mut self, src: FlowSource, r: ResourceId, moved: f64) {
        self.by_source.entry(src).or_insert([0.0; RESOURCE_COUNT])[r.ix()] += moved;
    }
}

/// Credit `moved` of `r` to `src`, where the run is keeping a ledger.
#[inline]
pub(crate) fn credit(
    ledger: &mut Option<&mut FlowLedger>,
    src: FlowSource,
    r: ResourceId,
    moved: f64,
) {
    if let Some(l) = ledger {
        l.credit(src, r, moved);
    }
}

/// A timed state the meadow can hold. The variant order is the order the
/// timers query lists them in, so the derived `Ord` is load-bearing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Timer {
    /// A visible sky event waiting to be observed.
    SkyEvent,
    /// A Star Whale's visit.
    WhaleVisit,
    Jamboree,
    Zoomies,
    DejaVuDay,
}

/// Why a piece of content is not currently actionable — data, not prose, on
/// the same principle as the event vocabulary.
///
/// The variant order is the resolution precedence, matching the command
/// layer's: when several blockers hold at once, the earliest wins, so the
/// reason shown for an item and the error returned by acting on it can never
/// name different causes. @spec SIM-ENG-031, SIM-ENG-036
#[derive(Debug, Clone, PartialEq)]
pub enum LockReason {
    /// Not yet reached. Deliberately says nothing further: costs and
    /// prerequisites of content the player has not revealed stay unstated.
    /// @spec SIM-ENG-037
    NotYetReached,
    /// Revealed, but a prerequisite is unmet.
    Prerequisite,
    /// Already owned, or already done, for a one-shot.
    AlreadyYours,
    /// No idle kitten to take it on.
    NoOneFree,
    /// No seat available for the work.
    NoRoom,
    /// Affordable in kind but not yet in quantity.
    ShortBy { missing: Vec<(ResourceId, f64)> },
}

/// A fixed act that brings a store in. The variant order is the order the
/// sources query lists acts in, so the derived `Ord` is load-bearing.
/// @spec SIM-ENG-074
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Act {
    GatherCatnip,
    Forage,
    Rummage,
    Pounce,
    ObserveSky,
    HoldParade,
    RefineStarshard,
    RefineEcho,
    CondenseJar,
}

/// What a command does to one store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreLink {
    /// The store ends up with more, or with room for more.
    Raises,
    /// The store ends up with less.
    Lowers,
    /// Both of those.
    Both,
}

/// What the player would take or meet for a store to rise. The variant
/// order, and each id's order within its variant, is the order a resource's
/// sources are listed in, so the derived `Ord` is load-bearing.
/// @spec SIM-ENG-074
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Source {
    Structure(StructureId),
    Job(JobId),
    Craft(CraftId),
    Colony(crate::state::ColonyId),
    Act(Act),
    Arch(ArchId),
    Refinement(crate::state::RefinementId),
    Rite(crate::state::RiteId),
    Loom(crate::state::DreamLoomId),
}

impl Act {
    /// The command the act is taken by, for its gate.
    pub fn command(self) -> crate::command::Command {
        use crate::command::Command as C;
        match self {
            Act::GatherCatnip => C::GatherCatnip,
            Act::Forage => C::Forage,
            Act::Rummage => C::Rummage,
            Act::Pounce => C::Pounce(1),
            Act::ObserveSky => C::ObserveSky,
            Act::HoldParade => C::HoldParade,
            Act::RefineStarshard => C::RefineStarshard,
            Act::RefineEcho => C::RefineEcho,
            Act::CondenseJar => C::CondenseJar,
        }
    }
}

/// The paths that raise a fixed resource by rule rather than by a table —
/// each restated here beside where the rule lives, and held to the core by
/// the sources tests, which raise every stock every command and the pipeline
/// can. @spec SIM-ENG-074
const RULED_SOURCES: &[(Source, &[ResourceId])] = {
    use crate::state::{ArchId as A, DreamLoomId as L, RiteId as T, StructureId as S};
    use ResourceId as R;
    &[
        // Sky events: seen by chance at a day close (tick.rs, sky events).
        (
            Source::Structure(S::StargazingLedge),
            &[R::Curiosity, R::Skymaps],
        ),
        // Prism descent from each Rainbow Arch (effects.rs, PrismDescentChance).
        (Source::Structure(S::RainbowArch), &[R::Prisms]),
        // Déjà-vu days fill reverie (effects.rs, DejaVuChance).
        (Source::Structure(S::MemoryChest), &[R::Reverie]),
        // The fixed acts (command.rs).
        (Source::Act(Act::GatherCatnip), &[R::Catnip]),
        (Source::Act(Act::Forage), &[R::Twigs]),
        (Source::Act(Act::Rummage), &[R::Pebbles]),
        (
            Source::Act(Act::Pounce),
            &[R::Feathers, R::Bells, R::Shinies, R::Unicorns],
        ),
        (Source::Act(Act::ObserveSky), &[R::Curiosity, R::Skymaps]),
        (Source::Act(Act::HoldParade), &[R::Rainbows]),
        (Source::Act(Act::RefineStarshard), &[R::Starshards]),
        (Source::Act(Act::RefineEcho), &[R::Echoes]),
        (Source::Act(Act::CondenseJar), &[R::JarsOfPurrs]),
        // The whale's exchange (command.rs, whale_exchange).
        (
            Source::Colony(crate::state::ColonyId::StarWhales),
            &[R::Skymaps, R::Starshards, R::Echoes],
        ),
        // Arch upgrades: descent, shower, drift, and dreaming (effects.rs,
        // arch_effect; tick.rs, dream_step).
        (Source::Arch(A::CrystalArch), &[R::Prisms]),
        (Source::Arch(A::PrismArch), &[R::Rainbows]),
        (Source::Arch(A::SkyArch), &[R::Prisms]),
        (Source::Arch(A::MemoryArch), &[R::Dreamcorns]),
        // Rites that grant the altar charm (effects.rs, rite_effect).
        (Source::Rite(T::StainedSunlight), &[R::Charm]),
        (Source::Rite(T::GrandBasking), &[R::Charm]),
        // The loom that brings déjà-vu days (effects.rs, loom_effect).
        (Source::Loom(L::DreamCatcher), &[R::Reverie]),
        // The loom whose reverie lands with a nap (command.rs, deep_nap).
        (Source::Loom(L::LucidNap), &[R::Reverie]),
    ]
};

/// What a colony has shown the clowder, and what it says of the rest.
/// @spec SIM-TRADE-012
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Offers {
    /// Whether any caravan has reached the colony this meadow.
    pub reached: bool,
    /// Goods caravans have brought home from it, in the colony's own order.
    pub learned: Vec<ResourceId>,
    /// Goods it offers that no caravan has brought home yet, in that order,
    /// each with whether it has them to trade at its current Pen Pal level.
    pub unseen: Vec<(ResourceId, bool)>,
    /// Whether it tells of its unseen goods by name: a warm colony does.
    pub tells_by_name: bool,
}

/// What the simulation holds about a kitten that bears on play and no other
/// surface states.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KittenFacts {
    pub trait_id: KittenTrait,
    pub rank: u8,
    /// The job it knows best, or None while it has no skill above zero.
    pub best_at: Option<BestAt>,
    pub arrival: Arrival,
}

/// The job a kitten holds the most skill in, and the yield bonus that
/// skill's tier gives it there, as a fraction (0.025 for +2.5%).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BestAt {
    pub job: JobId,
    pub bonus: f64,
}

/// A colony's correspondence as a frontend shows it. @spec SIM-TRADE-013
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PenPalReport {
    pub level: u32,
    /// The charm the next letter costs.
    pub price: f64,
    /// The colony's standing now.
    pub standing: f64,
    /// The part of that standing the letters have added.
    pub warmth: f64,
    /// How much one more letter would raise the standing; zero once it can't.
    pub next_warmth: f64,
    /// The lowest level above this one at which a good the colony sells
    /// opens, if any.
    pub opens_at: Option<u32>,
}

/// The meadow's mess as a frontend states it. @spec SIM-ECO-049
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MessReport {
    pub amount: f64,
    /// None below the first level.
    pub level: Option<u32>,
    /// Change per game second at current state: accrual less dissipation.
    pub rate: f64,
    /// Catnip growth's multiplier from the level (1.0 with none).
    pub catnip: f64,
    /// Arrival chance's multiplier from the level (1.0 with none).
    pub arrival: f64,
    /// Happiness points the level costs (0.0 with none).
    pub happiness: f64,
}

/// When a kitten joined the clowder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrival {
    /// It arrived on this day, in this season of this year.
    Day { day: u64, season: Season, year: u64 },
    /// A Move carried it along from the old meadow.
    CarriedAlong,
}

/// The tinkerers' bench: the meadow's seats, who holds them, and who waits
/// for one. @spec SIM-ENG-073
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Bench {
    /// Seats the meadow's structures give.
    pub seats: u32,
    /// Every binding — an away kitten's included, since it keeps its seat —
    /// as (roster position, recipe), in roster order.
    pub seated: Vec<(usize, CraftId)>,
    /// Every kitten at home holding the Tinkerer job and no binding, by
    /// roster position in roster order: the kittens a bind can take.
    pub waiting: Vec<usize>,
}

/// Every source there is: each structure, job, recipe, colony, and refinement,
/// and whatever else the rules decide raises a store.
fn all_sources() -> Vec<Source> {
    use crate::state::{ColonyId, RefinementId};
    let mut all: Vec<Source> = Vec::new();
    all.extend(StructureId::ALL.map(Source::Structure));
    all.extend(JobId::ALL.map(Source::Job));
    all.extend(CraftId::ALL.map(Source::Craft));
    all.extend(ColonyId::ALL.map(Source::Colony));
    all.extend(RefinementId::ALL.map(Source::Refinement));
    all.extend(RULED_SOURCES.iter().map(|(src, _)| *src));
    all
}

/// What a source can raise: the definitions' own outputs, a structure's
/// productions and converter outputs, a job's yield, a recipe's output, a
/// colony's goods, a refinement's granted outputs, and what `RULED_SOURCES`
/// restates from the rules. Fixed by the content. @spec SIM-ENG-074
fn source_goods(src: Source) -> Vec<ResourceId> {
    use crate::content;
    use crate::effects::Effect;
    let mut goods: Vec<ResourceId> = match src {
        Source::Structure(s) => {
            let d = content::structure_def(s);
            let mut out: Vec<ResourceId> = d.productions.iter().map(|(r, _)| *r).collect();
            if let Some(c) = &d.converter {
                out.extend(c.outputs.iter().map(|(r, _)| *r));
                if let Some(fed) = c.cap_fed {
                    out.extend(fed.outputs.iter().map(|(r, _)| *r));
                }
            }
            out
        }
        Source::Job(j) => content::job_def(j)
            .yield_per_tick
            .iter()
            .map(|(r, _)| *r)
            .collect(),
        Source::Craft(c) => vec![content::craft_output(c)],
        Source::Colony(c) => content::colony_goods(c)
            .into_iter()
            .map(|(r, _)| r)
            .collect(),
        Source::Refinement(f) => content::refinement_def(f)
            .effects
            .iter()
            .filter_map(|(e, _)| match e {
                Effect::StructureOutputAdd(_, r) => Some(*r),
                _ => None,
            })
            .collect(),
        Source::Act(_) | Source::Arch(_) | Source::Rite(_) | Source::Loom(_) => Vec::new(),
    };
    for (ruled, ruled_goods) in RULED_SOURCES {
        if *ruled == src {
            goods.extend_from_slice(ruled_goods);
        }
    }
    goods
}

impl GameState {
    /// What a discovered tradeable colony's pen pals have done and what the
    /// next letter would do, read through the standing rule itself; nothing
    /// for the Star Whales or a colony not yet found. @spec SIM-TRADE-013
    pub fn pen_pal(&self, colony: crate::state::ColonyId) -> Option<PenPalReport> {
        use crate::content;
        if colony == crate::state::ColonyId::StarWhales || !self.discovered.contains(&colony) {
            return None;
        }
        let level = self.penpal[colony.ix()];
        let standing_at = |l: u32| {
            let mut probe = self.clone();
            probe.penpal[colony.ix()] = l;
            probe.standing(colony)
        };
        let standing = self.standing(colony);
        Some(PenPalReport {
            level,
            price: content::PENPAL_BASE_CHARM * content::PENPAL_CHARM_RATIO.powi(level as i32),
            standing,
            warmth: standing - standing_at(0),
            next_warmth: standing_at(level + 1) - standing,
            opens_at: content::colony_goods(colony)
                .into_iter()
                .map(|(_, min)| min)
                .filter(|min| *min > level)
                .min(),
        })
    }

    /// Each built structure that makes or uses energy, in structure order,
    /// with its signed share at current state: what its copies make, as
    /// production counts it, or minus what they use, as consumption counts it
    /// — so the shares add up to both. @spec SIM-ECO-048
    pub fn energy_shares(&self) -> Vec<(StructureId, f64)> {
        let ratio = self.energy_consumption_ratio();
        let mut out = Vec::new();
        for s in StructureId::ALL {
            let def = crate::content::structure_def(s);
            if self.structures[s.ix()] == 0 {
                continue;
            }
            let (made, used) = self.energy_of(s);
            if def.energy_prod != 0.0 {
                out.push((s, made));
            }
            if used != 0.0 {
                out.push((s, -used * ratio));
            }
        }
        out
    }

    /// The meadow's mess, its level, how fast it is changing, and what the
    /// level slows. @spec SIM-ECO-049
    pub fn mess_report(&self) -> MessReport {
        use crate::content;
        let level = self.mess_level();
        let mut probe = self.clone();
        let (change, _) = probe.resource_phases(&mut None);
        // How far the next tick moves it, stopping at zero.
        let per_tick = self.next_mess(change) - self.mess;
        MessReport {
            amount: self.mess,
            level: (level >= 0).then_some(level as u32),
            rate: per_tick * content::TICKS_PER_SECOND as f64,
            catnip: self.mess_catnip_factor(),
            arrival: if level >= 1 {
                content::MESS_ARRIVAL_FACTOR
            } else {
                1.0
            },
            happiness: if level >= 2 {
                content::MESS_HAPPINESS_POINTS * (level - 1) as f64
            } else {
                0.0
            },
        }
    }

    /// The facts about the kitten at roster position `ix`, or None where no
    /// kitten stands there. @spec SIM-KIT-041
    pub fn kitten_facts(&self, ix: usize) -> Option<KittenFacts> {
        let k = self.kittens.get(ix)?;
        // The bonus is the one production applies, the master tier scaled as
        // production scales it. @spec SIM-KIT-020
        let master_mult = 1.0 + self.effect(crate::effects::Effect::MasterSkillMultiplier);
        // The most skill, ties to the earliest job; none while all are zero.
        let best_at = JobId::ALL
            .iter()
            .fold(None::<JobId>, |best, &j| match best {
                Some(b) if k.skills[b as usize] >= k.skills[j as usize] => Some(b),
                _ if k.skills[j as usize] > 0.0 => Some(j),
                _ => best,
            })
            .map(|job| BestAt {
                job,
                bonus: crate::content::skill_tier_bonus_scaled(k.skills[job as usize], master_mult),
            });
        // No arrival check runs before the first day close, so day 0 is a
        // Move's carried kitten (SIM-MOVE-002).
        let arrival = match k.arrived_day {
            0 => Arrival::CarriedAlong,
            day => Arrival::Day {
                day,
                season: crate::state::season_of_day(day),
                year: day / crate::content::DAYS_PER_YEAR,
            },
        };
        Some(KittenFacts {
            trait_id: k.trait_id,
            rank: k.rank,
            best_at,
            arrival,
        })
    }

    /// Every source that can raise `r` during a meadow's play, in the fixed
    /// listing order, each with whether the player knows it yet.
    ///
    /// The tables carry most paths — productions, converter outputs, yields,
    /// recipe outputs, a colony's goods, a refinement's granted outputs — and
    /// `RULED_SOURCES` restates the ones the rules decide. Fixed by the
    /// content: the state decides only what is known. @spec SIM-ENG-074
    pub fn sources_of(&self, r: ResourceId) -> Vec<(Source, bool)> {
        all_sources()
            .into_iter()
            .filter(|src| source_goods(*src).contains(&r))
            .collect::<std::collections::BTreeSet<Source>>()
            .into_iter()
            .map(|src| (src, self.knows(src, r)))
            .collect()
    }

    /// What a command does to the stores: each resource it raises, lowers, or
    /// both, in the core's resource order. A store is lowered by the next
    /// price, unless the item is the player's already, and by what the item
    /// draws down while it stands; raised by the goods of the source the item
    /// is, where the player knows that source for that good, and by what the
    /// item grows by name. @spec SIM-ENG-079, SIM-ENG-080, SIM-ENG-081
    pub fn store_links(&self, cmd: &crate::command::Command) -> Vec<(ResourceId, StoreLink)> {
        use crate::command::Command as C;
        use crate::content;
        use crate::effects::{store_effects, EffectItem};
        use crate::state::ContentId as Id;

        let mut lowers: Vec<ResourceId> = crate::state::GameState::fixed_act_spends(cmd).to_vec();
        let mut raises: Vec<ResourceId> = Vec::new();
        let mut priced: Option<Id> = None;
        let mut source: Option<Source> = None;
        let mut effects: Option<EffectItem> = None;
        match *cmd {
            C::Build(s) => {
                priced = Some(Id::Structure(s));
                source = Some(Source::Structure(s));
                let d = content::structure_def(s);
                raises.extend(d.cap_bonuses.iter().map(|(r, _)| *r));
                for (r, ratio) in d.resource_ratios {
                    if *ratio > 0.0 {
                        raises.push(*r);
                    } else if *ratio < 0.0 {
                        lowers.push(*r);
                    }
                }
                if let Some(c) = &d.converter {
                    lowers.extend(c.inputs.iter().map(|(r, _)| *r));
                }
                if let Some(y) = &d.year_craft {
                    for (raw, craft) in y.pairs {
                        if self.craft_unlocked(*craft) {
                            lowers.push(*raw);
                            raises.push(content::craft_output(*craft));
                        }
                    }
                }
                if d.parade_rainbows > 0.0 {
                    raises.push(ResourceId::Rainbows);
                }
                if d.prism_descent > 0.0 {
                    raises.push(ResourceId::Prisms);
                }
                if d.deja_vu_chance > 0.0 {
                    raises.push(ResourceId::Reverie);
                }
                if d.dream_drift > 0.0 {
                    lowers.push(ResourceId::Prisms);
                    raises.push(ResourceId::Dreamcorns);
                }
            }
            C::AssignJob(j) => source = Some(Source::Job(j)),
            C::Craft(c) => {
                priced = Some(Id::Craft(c));
                source = Some(Source::Craft(c));
            }
            C::AssignTinkerer(c) => {
                source = Some(Source::Craft(c));
                lowers.extend(content::craft_def(c).inputs.iter().map(|(r, _)| *r));
            }
            C::ResearchIdea(i) => priced = Some(Id::Idea(i)),
            C::BuyRefinement(f) => {
                priced = Some(Id::Refinement(f));
                source = Some(Source::Refinement(f));
                effects = Some(EffectItem::Refinement(f));
            }
            C::AdoptRule(r) => {
                priced = Some(Id::Rule(r));
                effects = Some(EffectItem::Rule(r));
            }
            C::BuyRite(t) => {
                priced = Some(Id::Rite(t));
                source = Some(Source::Rite(t));
                effects = Some(EffectItem::Rite(t));
            }
            C::BuyArch(a) => {
                priced = Some(Id::Arch(a));
                source = Some(Source::Arch(a));
                effects = Some(EffectItem::Arch(a));
            }
            C::BuyDreamLoom(l) => {
                priced = Some(Id::DreamLoom(l));
                source = Some(Source::Loom(l));
                effects = Some(EffectItem::Loom(l));
            }
            C::BuyTradition(t) => {
                priced = Some(Id::Tradition(t));
                effects = Some(EffectItem::Tradition(t));
            }
            C::Pounce(_) => {
                // Under this adventure the squads come home with nothing.
                // @spec SIM-ENG-083
                if self.active_adventure != Some(crate::state::AdventureId::GentlePaws) {
                    source = Some(Source::Act(Act::Pounce));
                }
                lowers.extend(
                    self.unit_price(cmd)
                        .unwrap_or_default()
                        .iter()
                        .map(|(r, _)| *r),
                );
            }
            C::Trade(c, _) => {
                source = Some(Source::Colony(c));
                lowers.extend(
                    self.unit_price(cmd)
                        .unwrap_or_default()
                        .iter()
                        .map(|(r, _)| *r),
                );
            }
            C::LaunchExpedition(site) => {
                lowers.extend(content::site_def(site).mission_fuel.iter().map(|(r, _)| *r));
            }
            C::DeepNap(_) => {
                if self
                    .dream_looms
                    .contains(&crate::state::DreamLoomId::LucidNap)
                {
                    raises.push(ResourceId::Reverie);
                }
            }
            C::GatherCatnip => source = Some(Source::Act(Act::GatherCatnip)),
            C::Forage => source = Some(Source::Act(Act::Forage)),
            C::Rummage => source = Some(Source::Act(Act::Rummage)),
            C::ObserveSky => source = Some(Source::Act(Act::ObserveSky)),
            C::HoldParade => source = Some(Source::Act(Act::HoldParade)),
            C::RefineStarshard => source = Some(Source::Act(Act::RefineStarshard)),
            C::RefineEcho => source = Some(Source::Act(Act::RefineEcho)),
            C::CondenseJar => source = Some(Source::Act(Act::CondenseJar)),
            _ => {}
        }
        if let Some(id) = priced {
            if self.lock_reason(id) != Some(LockReason::AlreadyYours) {
                lowers.extend(self.cost_of_next(id, 1).iter().map(|(r, _)| *r));
            }
        }
        if let Some(src) = source {
            raises.extend(
                source_goods(src)
                    .into_iter()
                    .filter(|r| self.knows(src, *r)),
            );
        }
        if let Some(item) = effects {
            for (r, size) in store_effects(item) {
                if size > 0.0 {
                    raises.push(r);
                } else {
                    lowers.push(r);
                }
            }
        }
        ResourceId::ALL
            .into_iter()
            .filter_map(|r| match (raises.contains(&r), lowers.contains(&r)) {
                (true, true) => Some((r, StoreLink::Both)),
                (true, false) => Some((r, StoreLink::Raises)),
                (false, true) => Some((r, StoreLink::Lowers)),
                (false, false) => None,
            })
            .collect()
    }

    /// Whether the player has met `src` as a source of `r`, judged on the
    /// state at the asking. @spec SIM-ENG-075
    fn knows(&self, src: Source, r: ResourceId) -> bool {
        use crate::command::Command as C;
        use crate::state::{Availability, ColonyId, ContentId};
        let present = |id: ContentId| self.availability(id) != Availability::Hidden;
        match src {
            Source::Structure(s) => present(ContentId::Structure(s)),
            Source::Job(j) => self.job_unlocked(j),
            Source::Craft(c) => self.craft_unlocked(c),
            Source::Colony(ColonyId::StarWhales) => self.command_gate_open(&C::SummonStarWhale),
            Source::Colony(c) => self.learned_goods.get(&c).is_some_and(|g| g.contains(&r)),
            // Observing has no gate of its own; the ledge that raises sky
            // events is what the player meets first.
            Source::Act(Act::ObserveSky) => {
                present(ContentId::Structure(StructureId::StargazingLedge))
            }
            Source::Act(a) => self.command_gate_open(&a.command()),
            Source::Arch(a) => present(ContentId::Arch(a)),
            Source::Refinement(f) => present(ContentId::Refinement(f)),
            Source::Rite(t) => present(ContentId::Rite(t)),
            Source::Loom(l) => present(ContentId::DreamLoom(l)),
        }
    }

    /// What a colony has shown the clowder: whether a caravan has reached it,
    /// the goods caravans have brought home, the goods still unseen with
    /// whether its Pen Pal level has them on offer yet, and whether it is warm
    /// enough to name those unseen goods — disposition and friendship levels
    /// being rules, a shell reads them here. @spec SIM-TRADE-012
    pub fn neighbour_offers(&self, c: crate::state::ColonyId) -> Offers {
        let Some(learned) = self.learned_goods.get(&c) else {
            return Offers::default();
        };
        let level = self.penpal[c.ix()];
        let goods = crate::content::colony_goods(c);
        Offers {
            reached: true,
            learned: goods
                .iter()
                .filter(|(g, _)| learned.contains(g))
                .map(|(g, _)| *g)
                .collect(),
            unseen: goods
                .iter()
                .filter(|(g, _)| !learned.contains(g))
                .map(|(g, min)| (*g, level >= *min))
                .collect(),
            tells_by_name: self.standing(c) > 0.0,
        }
    }

    /// The tinkerers' bench: the seats the meadow's structures give, every
    /// binding, and every Tinkerer at home waiting for one.
    ///
    /// Who a bind can take and how many seats there are are both rules, so a
    /// shell reads them here rather than picking kittens out by job, home,
    /// and binding, or counting the structures that seat them.
    /// @spec SIM-ENG-073, SIM-ECO-045
    pub fn bench(&self) -> Bench {
        let seats = StructureId::ALL
            .iter()
            .map(|s| crate::content::structure_def(*s).tinkerer_seats * self.structures[s.ix()])
            .sum();
        let seated = self
            .kittens
            .iter()
            .enumerate()
            .filter_map(|(ix, k)| k.tinkering.map(|c| (ix, c)))
            .collect();
        let waiting = self
            .kittens
            .iter()
            .enumerate()
            .filter(|(_, k)| {
                k.location == Location::Home
                    && k.job == Some(JobId::Tinkerer)
                    && k.tinkering.is_none()
            })
            .map(|(ix, _)| ix)
            .collect();
        Bench {
            seats,
            seated,
            waiting,
        }
    }

    /// Why AssignTinkerer(c) would be refused now, or `None` where it would
    /// bind — the command's own checks in its own precedence, which the
    /// command reads back, so the reason shown and the refusal returned can
    /// never name different causes. @spec SIM-ENG-072, SIM-ENG-006
    pub fn tinkerer_bind_reason(&self, c: CraftId) -> Option<LockReason> {
        if !self.craft_unlocked(c) {
            return Some(LockReason::Prerequisite);
        }
        let bench = self.bench();
        if bench.waiting.is_empty() {
            return Some(LockReason::NoOneFree);
        }
        if bench.seated.len() as u32 >= bench.seats {
            return Some(LockReason::NoRoom);
        }
        None
    }

    /// Net stock change per game second at current state: production less
    /// consumption, throttling included.
    ///
    /// Derived from the same path the tick pipeline runs — never a parallel
    /// formula, which is the shadow math the HLD rejects for offline progress
    /// and which would drift the moment any leaf adds a multiplier.
    ///
    /// Speed-free, like every other rule in the sim (SIM-ENG-024): a caller
    /// wanting real-time applies speed at the boundary that already converts
    /// real time into ticks. Advisory — a cap or a supply change can overtake
    /// it. @spec SIM-ENG-030, SIM-ENG-034, SIM-ENG-035
    pub fn net_rate(&self, r: ResourceId) -> f64 {
        self.net_rates()[r.ix()]
    }

    /// Every resource's net rate in one pass.
    ///
    /// This is the primary form, and `net_rate` is sugar over it. A sidebar
    /// redraws every revealed resource every frame, and deriving that from the
    /// tick pipeline one resource at a time would run the whole pipeline once
    /// per resource — while caching the results shell-side is exactly what
    /// TUI-RENDER-001 forbids. @spec SIM-ENG-030, SIM-ENG-034, SIM-ENG-035
    pub fn net_rates(&self) -> [f64; RESOURCE_COUNT] {
        // A scratch clone runs `resource_phases` — the tick pipeline's own
        // single definition of every phase that moves a stock — without its
        // ticks counter moving — so no day boundary, no random-event RNG
        // draw, and no cap clamp folds into the number. That keeps the
        // answer "at current state" (SIM-ENG-030) rather than at whatever
        // the next tick's daily processing would do to it, and pre-cap
        // (SIM-ENG-035). `self` is untouched: probing is free.
        let before = self.resources;
        let mut probe = self.clone();
        probe.resource_phases(&mut None);
        let per_second = crate::content::TICKS_PER_SECOND as f64;
        let mut out = [0.0; RESOURCE_COUNT];
        for i in 0..RESOURCE_COUNT {
            out[i] = (probe.resources[i] - before[i]) * per_second;
        }
        out
    }

    /// The net rate of `r` taken apart by what moves it: each source's signed
    /// rate per game second, in the fixed listing order, every source that
    /// moved `r` by nothing left out.
    ///
    /// The same probe as `net_rates` — a scratch clone running the pipeline's
    /// own resource phases — but keeping a ledger, so every figure is a move
    /// the pipeline made, credited where it made it. The figures therefore sum
    /// to the net rate. Speed-free and advisory, as the net rate is; `self` is
    /// untouched. @spec SIM-ENG-062, SIM-ENG-064, SIM-ENG-066, SIM-ENG-067,
    /// SIM-ENG-068
    pub fn flows(&self, r: ResourceId) -> Vec<(FlowSource, f64)> {
        let mut ledger = FlowLedger::default();
        let mut probe = self.clone();
        probe.resource_phases(&mut Some(&mut ledger));
        let per_second = crate::content::TICKS_PER_SECOND as f64;
        ledger
            .by_source
            .iter()
            .filter(|(_, moved)| moved[r.ix()] != 0.0)
            .map(|(src, moved)| (*src, moved[r.ix()] * per_second))
            .collect()
    }

    /// Every timed state running at the current tick, in `Timer` order, each
    /// with the ticks until the first tick on which the rule that decides it
    /// no longer holds, were nothing done meanwhile. Ticks, not seconds: how
    /// long a tick lasts is the shell's cadence. @spec SIM-ENG-069, SIM-ENG-070
    pub fn timers(&self) -> Vec<(Timer, u64)> {
        let now = self.ticks;
        let until = |end: u64| (now < end).then(|| end - now);
        let year = crate::content::DAYS_PER_YEAR * crate::content::TICKS_PER_DAY;
        // A visit loses a year at each year close; it ends at the close that
        // takes its last. One standing on a close has already been counted.
        let whale = (self.whale_visit_years > 0).then(|| {
            let next_close = (now / year + 1) * year;
            next_close - now + u64::from(self.whale_visit_years - 1) * year
        });
        [
            (Timer::SkyEvent, self.pending_sky_event.and_then(until)),
            (Timer::WhaleVisit, whale),
            (Timer::Jamboree, until(self.jamboree_until)),
            (Timer::Zoomies, until(self.zoomies_until)),
            (Timer::DejaVuDay, until(self.deja_vu_until)),
        ]
        .into_iter()
        .filter_map(|(t, left)| left.map(|n| (t, n)))
        .collect()
    }
}
