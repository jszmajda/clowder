---
parent: sim
prefix: SIM-ENG
---

# Engine

## Context and Design Philosophy

The engine is the spine of `clowder-core`: it owns the canonical `GameState`, the command API the shell calls, the tick pipeline that sequences every other leaf's phase functions, the calendar, the RNG, the typed event vocabulary, and — at depth — the shared mechanics every content leaf builds on: the effect-key resolution system, the diminishing-returns primitives, converter throttling, unlock/visibility queries, and the game-speed field. Everything here is pure and deterministic — no IO, no clocks, no threads. The engine owns *machinery*; leaves own *content and values*.

## Effect-Key System

All modifiers in the game flow through one typed effect vocabulary (sub-HLD § Effect Resolution Architecture). The engine owns:

- `enum Effect` — compiler-checked variant per modifier kind (`ResourceRatio(ResourceId)`, `JobRatio(JobId)`, `MaxCap(ResourceId)`, `GlobalProductionRatio`, `CraftRatio`, …). Variants are added per wave; adding one is a source-level change that exhaustiveness-checks every consumer.
- `effects(state, key) -> f64` — sums contributions in a fixed source order: structures (× owned/active count as the key demands) → refinements → ideas → house rules → rites → arch upgrades → traditions → colony standing → adventure rewards → the Elder's trait. Fixed order keeps float accumulation deterministic.
- **DR primitives**: `limited_dr(v, limit)` (linear to 75% of limit, asymptotic above) and `unlimited_dr(v, stripe)` (`(sqrt(1 + 8v/stripe) − 1)/2`). Consumers of stacking sums apply one of these per the sub-HLD's DR discipline.

The production stack (sub-HLD § The Production Stack) is engine code: leaves declare rates and effect contributions; the engine computes per-tick flows in the normative layer order, exempting transient currencies from global layers.

## Converter Throttling

`throttle(inputs: &[(ResourceId, f64)], state) -> f64` returns the worst-supplied input's satisfaction fraction (0..=1); converter phase functions scale all their inputs and outputs by it. Chains degrade proportionally and never flap. A converter at fraction 0 is simply idle — never an error.

## Unlocks & Visibility

Content items (structures, ideas, refinements, house rules, rites, colonies, sites) share one availability model, engine-owned:

- An item **reveals** the first time its gate is satisfied and (for cost-revealed items like structures) every cost component has reached the reveal fraction of its current price (default 30%, content-tunable). Revelation is sticky: revealed ids live in a persisted `revealed` set in `GameState`, the scan runs at the end of the clamp phase, and each new revelation emits a `Revealed` event — the "something new is possible" moment.
- `Availability::{Hidden, Teased, Available}` — *Hidden*: not yet revealed; *Teased*: revealed but not currently affordable/unlockable; *Available*: revealed and actionable now. The shell renders Teased items dimmed; presentation is entirely the shell's.
- Purchase-time gating stays the command layer's job (`Locked` per error precedence); Availability is advisory for display.
- **Implementation note.** The per-tick reveal scan walks a derived, non-serialized worklist (the complement of `revealed`, canonical order) rather than probing the whole content space each tick — a hot-path accelerator, rebuilt from `revealed` on load so it can never drift from that serialized truth. Purely a performance concern; the observable reveal behavior is identical to a naive full scan.

## Game Speed

`speed: f64` in `GameState`, default 1.0, clamped [0.25, 8] on set and on load. A `SetSpeed(f64)` command (always legal, clamps silently, emits `SpeedChanged`) mutates it. Nothing in `tick()` reads it: the shell drives `4 × speed` ticks/second and persistence replays `elapsed × 4 × speed` — speed changes *how many* ticks happen per wall-second, never what a tick does.

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
- `grumpy: bool` — set each tick by the mood phase (consumption shortfall occurred); read by the next production phase, and by the next day close as the "did the day end in shortfall" bit for the famine/recovery clocks.
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

## Derived Queries

The shell renders; the core rules. Any question whose answer is a *rule* is answered here, even when the shell could technically assemble it from public state — a frontend that computes affordability, or explains why something is locked, is holding game knowledge the crate split exists to prevent, and it will drift from the real rule the first time the rule changes.

Four read-only queries carry that boundary:

- **Net rate per resource** — production minus consumption at current state, per second. This must be *derived from the tick pipeline itself*, not re-implemented as a parallel formula. A second copy of the production maths is exactly the shadow math the HLD rejects for offline progress, and it would drift the moment any leaf adds a multiplier.
- **Lock reason** — why a given piece of content is not actionable, as typed data rather than prose, drawn from the same gate the command layer enforces. Two sources for one answer is how a screen ends up explaining a rule the command layer no longer applies.
- **Availability, total over every content kind** — the query answers for anything revealable, not only the kinds that carry a reveal ladder. Kinds without one answer with the two states they have. Totality is the point: the shell renders exactly what it is told (TUI-NAV-053), so a query that silently has no answer for some kinds pushes the branch back into the frontend.
- **Maximum affordable count** — where a closed form exists, so a bulk action can name its count before acting instead of discovering it by attempting and failing.

## Event Vocabulary

`tick()` and `apply()` return `Vec<Event>` — typed data, no prose:

- `KittenArrived { name }`, `KittenWanderedOff { name }`, `KittenReturned { name }`
- `JobAssigned { name, job }`, `JobUnassigned { name, job }` — assignment names the kitten (kittens leaf: identity makes assignment delightful)
- `SeasonChanged { season }`, `NewYear { year }`
- `StructureBuilt { id, count }`, `IdeaResearched { id }`
- `ZoomiesStarted { until_tick }`, `ZoomiesEnded`
- `KittenMood { mood }` — emitted only on mood-tier transitions (content, not spam)
- `Flavor { template_id, kitten: Option<name> }` — pure-delight events with no mechanical effect; templates are content-table entries the shell renders

Every event carries a **classification** — flavor, mechanical, or milestone. It is a property of the vocabulary rather than a judgement the consumer makes: deciding that reaching a new site matters more than a job reassignment is a rule about the game, and a shell that made that call itself would be encoding game rules in the frontend. The classification is what lets a display rank prominence, and what lets the journal below decide what it may never evict.

## The Journal

`GameState` carries an event journal, because the events a player has lived through are part of their game and should survive closing the terminal. It holds a **bounded rolling window** of the most recent entries plus **every milestone entry, permanently**; each entry carries the tick at which it occurred, so a reader can group by day. When the window is full, the oldest non-milestone entry is evicted — growth is bounded, and the moments worth remembering are never what gets dropped.

**Catch-up replay does not append.** A year-long absence replays millions of ticks, and recording their events would write an enormous save and slow every load — the exact costs the persistence leaf's replay budget exists to prevent. Replay already folds into an `OfflineReport`; that summary is what enters the journal, and the constituent events are discarded as they always were. Live ticks and applied commands append; replay does not. This is what keeps a journal in the save file affordable.

## Random Events

Drawn in pipeline phase 5, at day boundaries, from the state RNG. The complete RNG draw schedule on a boundary tick is fixed: arrival roll (phase 1d — skipped entirely, no draw, when pity forces the arrival or the check is ineligible) → zoomies expiry check (no draw; per-tick, `ticks ≥ zoomies_until`) → zoomies start roll → flavor roll → flavor template draw → flavor kitten draw (the last two only when the preceding roll fired). Zoomies may restart on the same boundary tick they expired (expiry is checked first), and may start or run while the clowder is Grumpy — the multipliers stack (0.25 × zoomies), which is intended: zoomies shorten famines, and taking them away from a grumpy clowder would be anti-cozy.

- **Zoomies:** while inactive, daily chance 4% (expected roughly once per 25 game-days — about once a season); sets `zoomies_until = ticks + duration + 1` and emits `ZoomiesStarted`. The `+1` is deliberate: the start happens in phase 5 *after* that tick's production, so the window must extend one past the arithmetic end for exactly `duration` production ticks to be boosted — the stated duration is never quietly one short. The multiplier applies while `ticks < zoomies_until`; when phase 5 finds `ticks ≥ zoomies_until`, it resets the field to 0 and emits `ZoomiesEnded`. Base: ×1.25 for 120 ticks (30 s); the relevant idea raises this to ×1.5 for 240 ticks. **Magnitude is live, duration is fixed at start** — researching the idea mid-zoomies upgrades the multiplier immediately but never rewrites `zoomies_until` (the player only ever gains).
- **Flavor:** daily chance 8%. Templates are typed *ambient* or *needs-kitten*; the draw selects uniformly among currently eligible templates (needs-kitten templates are eligible only while at least one kitten is Home), then draws a kitten if the template wants one. Zero mechanical effect by construction.

## Determinism Contract

Same seed + same command sequence applied at the same tick counts ⇒ bit-identical serialized state. Enforced by: single RNG, fixed pipeline order, the boundary-tick RNG draw schedule above, no float operations whose order varies (accumulate flows in a fixed resource order), commands applied only between ticks, and **canonical serialization order for every GameState collection** — resources and structures as enum-indexed arrays (or BTreeMaps), the roster as an append-ordered Vec, ideas as a sorted set. "Bit-identical" is tested on serialized bytes; an unordered map anywhere in GameState would falsify it.

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|----------|--------|------------------------|-----------|
| Effect summation order | Fixed source-class order (structures→refinements→ideas→rules→rites→arches→traditions→standing→adventures→trait) | Arbitrary/iteration order | Float addition isn't associative; a fixed order is what makes bit-identical determinism survive content growth |
| Speed placement | `GameState.speed`, read only at the boundary (shell cadence, catch-up), never inside `tick()` | Speed as a tick()-internal multiplier on rates | Rate-multiplying would change what a tick *means*, breaking replay equivalence between live and catch-up play and every balance test |
| Availability model | Engine-owned Hidden/Teased/Available advisory queries | Per-leaf ad-hoc visibility; shell-side inference | One reveal model keeps the "silhouette teaser" pacing consistent across all content kinds, and the shell out of the rules business |
| Calendar storage | Derived from `ticks` | Stored day/season fields advanced by tick | Derived can't drift or double-advance during catch-up |
| Command atomicity | All-or-nothing, error enum | Partial application, panics | Shell can render precise "why not" hints; sim state can't half-change |
| RNG | PCG-32, hand-rolled, serialized with state | `rand` crate StdRng, reseeding on load | Stable serde round-trip across versions; ~30 lines; reseeding on load would break catch-up determinism |
| Events carry data, tier-gated | Typed enum; mood emitted on transitions only | Emit-every-tick, string messages in core | Tests assert on events; transition-gating keeps a week's catch-up report bounded |
| Event history | Bounded ring in `GameState` + permanent milestones | Shell-owned session history; unbounded log; milestones only | History in the save survives a quit, which is what makes a report the player hasn't read safe; unbounded turns every autosave into a growing rewrite over a months-long game |
| Event classification | A property of the event vocabulary, queryable | The shell decides which events are prominent | "This moment matters" is a rule about the game; a shell deciding it would hold game knowledge the split forbids |
| Replay and the journal | Replay events fold into the report and never append | Append replay events like any other | A year of replay is millions of events; appending them would blow up the save and the load budget in one step |
| Daily randomness cadence | Random events roll at day boundaries only | Per-tick probabilities | Fewer RNG draws keeps catch-up fast and probabilities legible ("8%/day", not "0.0083%/tick") |

## Open Questions & Future Decisions

### Resolved
1. ✅ Zoomies frequency: daily 4% ≈ once per 25 game-days (about once a season) — frequent enough to delight, rare enough to feel like an event.

### Deferred

## References

- Parent: `../sim-design.md` (pipeline order, determinism rationale, cozy invariants)
- Consumes phase functions from: `../economy/`, `../kittens/`, `../curiosity/`
