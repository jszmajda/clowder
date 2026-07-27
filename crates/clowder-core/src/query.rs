//! Read-only derived queries: the answers that are *rules*.
//!
//! The shell renders; the core rules. Anything whose answer is a rule is
//! answered here even where a caller could assemble it from public state — a
//! second copy of a rule drifts from the first the moment the rule changes.
//! @spec SIM-ENG-030, SIM-ENG-031, SIM-ENG-032, SIM-ENG-033

use crate::state::{GameState, ResourceId, RESOURCE_COUNT};

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
        probe.resource_phases();
        let per_second = crate::content::TICKS_PER_SECOND as f64;
        let mut out = [0.0; RESOURCE_COUNT];
        for i in 0..RESOURCE_COUNT {
            out[i] = (probe.resources[i] - before[i]) * per_second;
        }
        out
    }
}
