//! Read-only derived queries: the answers that are *rules*.
//!
//! The shell renders; the core rules. Anything whose answer is a rule is
//! answered here even where a caller could assemble it from public state — a
//! second copy of a rule drifts from the first the moment the rule changes.
//! @spec SIM-ENG-030, SIM-ENG-031, SIM-ENG-032, SIM-ENG-033, SIM-ENG-062,
//! SIM-ENG-069

use std::collections::BTreeMap;

use crate::state::{ArchId, CraftId, GameState, JobId, ResourceId, StructureId, RESOURCE_COUNT};

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

impl GameState {
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
