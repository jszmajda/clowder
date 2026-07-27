//! Read-only derived queries: the answers that are *rules*.
//!
//! The shell renders; the core rules. Anything whose answer is a rule is
//! answered here even where a caller could assemble it from public state — a
//! second copy of a rule drifts from the first the moment the rule changes.
//! @spec SIM-ENG-030, SIM-ENG-031, SIM-ENG-032, SIM-ENG-033

use crate::state::{ContentId, GameState, ResourceId, RESOURCE_COUNT};

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
        todo!("SIM-ENG-030: one pass over the tick pipeline's production/consumption")
    }

    /// Why this content is not actionable, or `None` when it is.
    /// @spec SIM-ENG-031, SIM-ENG-036, SIM-ENG-037
    pub fn lock_reason(&self, _id: ContentId) -> Option<LockReason> {
        todo!("SIM-ENG-031: resolve by the command layer's blocker precedence")
    }

    /// How many times a repeatable action could be taken right now, where a
    /// closed form exists — so a caller can name the count in advance instead
    /// of discovering it by attempting until rejected. `None` where the
    /// action has no closed form. @spec SIM-ENG-033
    pub fn max_affordable(&self, _id: ContentId) -> Option<u32> {
        todo!("SIM-ENG-033: closed-form affordable count where one exists")
    }

    /// The total cost of taking a repeatable action `n` times from here,
    /// escalation included.
    ///
    /// A count prompt previews this before committing (TUI-NAV-011). The
    /// escalation is a rule, and a caller summing it from single-step costs
    /// would be reimplementing that rule outside the core — the same shadow
    /// math the HLD rejects. @spec SIM-ENG-038
    pub fn cost_of_next(&self, _id: ContentId, _n: u32) -> Vec<(ResourceId, f64)> {
        todo!("SIM-ENG-038: escalating cost summed over n steps")
    }

    /// How far the stock covers a cost, as a fraction in [0, 1] — 1 meaning
    /// affordable now. What separates a partially-affordable row from an
    /// unaffordable one, decided here rather than by a caller inventing a
    /// threshold. @spec SIM-ENG-039
    pub fn affordability(&self, _id: ContentId) -> f64 {
        todo!("SIM-ENG-039: fraction of the next cost currently covered")
    }
}
