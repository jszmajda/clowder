---
parent: sim
prefix: SIM-ENG
---

# Engine

## Context and Design Philosophy

The engine is the spine of `clowder-core`: it owns the canonical `GameState`, the command API the shell calls, the tick pipeline that sequences every other leaf's phase functions, the calendar, the RNG, and the typed event vocabulary. Everything here is pure and deterministic — no IO, no clocks, no threads.

## GameState

One serde-serializable struct, the single source of truth:

- `ticks: u64` — monotone tick counter; the calendar is derived from it (no separate stored date to drift).
- `rng: Pcg32` — hand-rolled PCG-32 (64-bit state, 64-bit stream constant); serializable; the only randomness source.
- `resources` — stocks per resource kind (economy leaf defines kinds/caps).
- `structures` — owned count per structure kind.
- `kittens` — the roster (kittens leaf defines the entry shape).
- `ideas` — completed idea set, stored as a sorted set for deterministic iteration.
- `pity: u8` — consecutive failed *eligible* arrival checks (kittens leaf semantics); saturating.
- `famine_days` / `recovery_days: u8` — famine and recovery clocks (kittens leaf semantics); saturating — only threshold crossings matter, so saturation is harmless.
- `grumpy: bool` — set each tick by the mood phase (consumption shortfall occurred); read by the next production phase.
- `next_seq: u64` — monotone counter stamping kitten job assignments and departures (kittens leaf ordering semantics).
- `zoomies_until: u64` — tick until which zoomies are active (active while `ticks < zoomies_until`); 0 when inactive.

Derived values (happiness, effective rates, season) are computed, never stored — stored state is minimal so saves can't go internally inconsistent. A queryable next-season forecast is exposed once the relevant idea is researched (until then the query returns nothing).

A new game (`GameState::new(seed)`) is an empty meadow: zero stocks, no structures, no kittens, no ideas, tick 0. The seed comes from the caller — the shell picks it (typically from the OS clock, which is allowed *outside* the sim); everything after the seed is deterministic.

## Calendar

Derived entirely from `ticks`: day = `ticks / 10`, season index = `(day / 25) % 4` (order: Spring, Summer, Autumn, Winter, starting in Spring), year = `day / 100`. Day boundaries trigger daily sub-steps (arrival, wander/return); season boundaries emit `SeasonChanged`.

## Command API

```rust
pub enum Command {
    GatherCatnip,          // manual: +1 catnip (×2 with the relevant idea)
    Forage,                // manual: spend catnip, gain a twig
    Build(StructureId),
    AssignJob(JobId),      // one idle kitten takes the job
    UnassignJob(JobId),    // one worker returns to loafing
    ResearchIdea(IdeaId),
}

pub fn apply(state: &mut GameState, cmd: Command) -> Result<Vec<Event>, CommandError>;
pub fn tick(state: &mut GameState) -> Vec<Event>;
```

`CommandError` names the reason: `UnknownId`, `Locked` (structure/job/idea not yet unlocked), `AlreadyResearched`, `NoIdleKittens`, `NoWorkersAssigned`, `StorageFull` (a manual action whose yield resource sits at cap — spending a press for a trimmed-away yield would be a clarity failure), `InsufficientResources { missing }`. When several apply, the **first in that list wins** — report the most fundamental blocker, and never leak the cost of content the player hasn't unlocked. Commands either fully apply or return an error — no partial spends. A rejected command never mutates state (RNG included).

## Event Vocabulary

`tick()` and `apply()` return `Vec<Event>` — typed data, no prose:

- `KittenArrived { name }`, `KittenWanderedOff { name }`, `KittenReturned { name }`
- `SeasonChanged { season }`, `NewYear { year }`
- `StructureBuilt { id, count }`, `IdeaResearched { id }`
- `ZoomiesStarted { until_tick }`, `ZoomiesEnded`
- `KittenMood { mood }` — emitted only on mood-tier transitions (content, not spam)
- `Flavor { template_id, kitten: Option<name> }` — pure-delight events with no mechanical effect; templates are content-table entries the shell renders

## Random Events

Drawn in pipeline phase 5, at day boundaries, from the state RNG:

- **Zoomies:** while inactive, daily chance 4% (expected roughly once per 25 game-days — about once a season); sets `zoomies_until = ticks + duration` and emits `ZoomiesStarted`. The multiplier applies while `ticks < zoomies_until`; on the first tick past expiry, phase 5 resets the field to 0 and emits `ZoomiesEnded`. Base: ×1.25 for 120 ticks (30 s); the relevant idea raises this to ×1.5 for 240 ticks. **Magnitude is live, duration is fixed at start** — researching the idea mid-zoomies upgrades the multiplier immediately but never rewrites `zoomies_until` (the player only ever gains).
- **Flavor:** daily chance 8%. Templates are typed *ambient* or *needs-kitten*; the draw selects uniformly among currently eligible templates (needs-kitten templates are eligible only while at least one kitten is Home), then draws a kitten if the template wants one. Zero mechanical effect by construction.

## Determinism Contract

Same seed + same command sequence applied at the same tick counts ⇒ bit-identical serialized state. Enforced by: single RNG, fixed pipeline order, sorted iteration for all collections that feed RNG draws or ordered effects, no float operations whose order varies (accumulate flows in a fixed resource order), commands applied only between ticks.

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|----------|--------|------------------------|-----------|
| Calendar storage | Derived from `ticks` | Stored day/season fields advanced by tick | Derived can't drift or double-advance during catch-up |
| Command atomicity | All-or-nothing, error enum | Partial application, panics | Shell can render precise "why not" hints; sim state can't half-change |
| RNG | PCG-32, hand-rolled, serialized with state | `rand` crate StdRng, reseeding on load | Stable serde round-trip across versions; ~30 lines; reseeding on load would break catch-up determinism |
| Events carry data, tier-gated | Typed enum; mood emitted on transitions only | Emit-every-tick, string messages in core | Tests assert on events; transition-gating keeps a week's catch-up report bounded |
| Daily randomness cadence | Random events roll at day boundaries only | Per-tick probabilities | Fewer RNG draws keeps catch-up fast and probabilities legible ("8%/day", not "0.0083%/tick") |

## Open Questions & Future Decisions

### Resolved
1. ✅ Zoomies frequency: daily 4% ≈ once per 25 game-days (about once a season) — frequent enough to delight, rare enough to feel like an event.

### Deferred
1. Event-log ring buffer in core (for a TUI history pane) vs. shell-owned history — decide in the TUI segment; core returning `Vec<Event>` doesn't foreclose either.

## References

- Parent: `../sim-design.md` (pipeline order, determinism rationale, cozy invariants)
- Consumes phase functions from: `../economy/`, `../kittens/`, `../curiosity/`
