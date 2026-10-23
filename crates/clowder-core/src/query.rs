//! Read-only derived queries: the answers that are *rules*.
//!
//! The shell renders; the core rules. Anything whose answer is a rule is
//! answered here even where a caller could assemble it from public state — a
//! second copy of a rule drifts from the first the moment the rule changes.
//! @spec SIM-ENG-030, SIM-ENG-031, SIM-ENG-032, SIM-ENG-033, SIM-ENG-062,
//! SIM-ENG-069

use std::collections::BTreeMap;

use crate::state::{
    ArchId, CraftId, GameState, JobId, Location, ResourceId, StructureId, RESOURCE_COUNT,
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

impl GameState {
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
