---
parent: high-level-design
prefix: SIM
---

# Simulation (sub-HLD)

> **Spoiler notice:** this document and everything beneath `docs/intent/sim/` describe game content. The player-owner discovers content by playing; see `AGENTS.md § Spoiler Policy`.

## Context and Design Philosophy

The simulation is the whole game: every rule, number, name, and event lives in `clowder-core`, headless and deterministic. The TUI is a window onto it. This sub-HLD holds what its child leaves share — the tick pipeline, the determinism contract, the cozy invariants, and the ownership boundaries between leaves — and owns no EARS of its own.

Three properties are load-bearing across every child:

1. **Determinism.** Given the same RNG seed and the same command sequence applied at the same tick counts, the simulation reaches an identical state. All randomness flows through one serializable RNG owned by the game state; nothing reads the wall clock inside the sim. This is what makes offline catch-up honest (same `tick()`, fast-forwarded) and balance tests reproducible.
2. **The cozy invariants** (HLD Tenet 1, made mechanical):
   - No kitten is ever removed from the roster. Absence is a *location*, never a deletion.
   - No structure is ever destroyed against the player's will.
   - No resource stock is ever reduced as a penalty. Stocks fall only through consumption, player-chosen spending, and storage-overflow trimming (clamping to caps is physics, not punishment).
   - Every negative pressure is a *multiplier on gains* (moods, seasons), never a confiscation.
3. **Events, not prose.** The sim reports what happened each tick as typed events; presentation (wording, color, layout) belongs to the shell. Flavor text templates live in core content tables (they are game content), but *when to show, where, and how* is the shell's.

## Time Model

| Unit | Definition | Real time |
|---|---|---|
| Tick | atomic sim step | 250 ms (4 ticks/s) |
| Day | 10 ticks | 2.5 s |
| Season | 25 days | 62.5 s |
| Year | 4 seasons | ~4.2 min |

All internal rates are per-tick. The shell decides when ticks happen (live timer or catch-up fast-forward); the sim never sleeps or measures time.

## Tick Pipeline

`tick()` runs phases in this fixed order. Order is normative — several behaviors (e.g., "eat before mood update") depend on it:

1. **Calendar & daily processing** — increment the tick counter. A tick is a *day-boundary tick* when the increment crosses into a new day (`day(t) > day(t−1)`, day = ticks/10). On a boundary tick, before any production: **(a) day close** — update the famine/recovery clocks from the stock as it stood at the end of the previous tick; **(b) returns** — away kittens come home; **(c) wanders** — famine departures; **(d) arrival check**. Returns precede arrivals so a homecoming kitten is never displaced by a newcomer, and precede wanders vacuously (the clocks make the two mutually exclusive). Season/year boundary events emit here. No events fire at game start (`ticks = 0` is mid-day-0; the first boundary is tick 10).
2. **Production** — structures produce; employed kittens produce (mood-multiplied); apply season modifiers.
3. **Consumption** — kittens at home eat catnip (pooled deduction; the economy leaf defines shortfall handling).
4. **Mood & famine flag** — recompute happiness; set or clear the Grumpy state based on whether consumption fell short this tick.
5. **Random events** — zoomies expiry/start and flavor events, drawn from the state RNG at day boundaries.
6. **Clamp** — trim stocks to their storage caps. (Production may transiently exceed caps between phases 2 and 6; consumption eating from that transient is intended and player-favoring.)

Commands are applied *between* ticks, never during one. All per-day counters use saturating arithmetic — only threshold crossings carry meaning, so saturation is harmless and overflow is impossible.

## Child Leaves and Ownership Boundaries

| Leaf | Prefix | Owns |
|---|---|---|
| `engine/` | `SIM-ENG` | `GameState`, command API, tick pipeline & calendar, RNG & determinism, event types, random-event scheduling |
| `economy/` | `SIM-ECO` | Resource kinds, stocks, caps; structures (definitions, costs, ramps, effects); manual actions; all resource-flow math |
| `kittens/` | `SIM-KIT` | Population lifecycle (arrival, away, return), names, job assignment, happiness/mood computation |
| `curiosity/` | `SIM-CUR` | The idea tree: definitions, costs, prerequisites, unlock effects |

Cross-leaf contracts (the boundary, stated once here, referenced by the leaves):

- **kittens → economy:** the kittens leaf exposes *effective workers per job* and the *mood multiplier*; the economy leaf turns those into resource deltas. Kittens decide who works; economy decides what work yields.
- **curiosity → economy/kittens:** ideas set unlock flags and modifiers (structure availability, job availability, rate/penalty adjustments); the other leaves consume flags, never reach into the tree.
- **engine → all:** the engine owns sequencing and randomness; leaves expose pure per-phase functions the pipeline calls in order.

## Roadmap (later passes, not in the current arrow)

Designed direction for future segments, recorded so the current leaves don't foreclose them: the Great Meadow Move (prestige with elder wisdom), Sunbeam Ceremonies (devotion/Warmth economy), Pouncing expeditions (active-play bursts), Unicorn Parades (late-game conversion loop), kitten job skills, crafting chains, trade with other animal colonies, weather variance within seasons. Each arrives as a new leaf (or a promotion) under this sub-HLD when its turn comes.

**The Great Meadow Move is first in line** once the TUI segment ships: prestige unlocks are a stated delight-priority for the player-owner, and the prestige design must reconcile with the cozy invariants (the whole clowder moves *together* — a reset never takes a kitten, a name, or an earned idea's dignity away). The save format's migration hook (persistence segment) is the planned landing path for the meta-progression state it will add.

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|----------|--------|------------------------|-----------|
| Tick length | 250 ms (4 ticks/s) | 200 ms (Kittens Game), 1 s | Lively counters without burning CPU in a TUI; a week away ≈ 2.4M ticks, still fast-forwardable in seconds |
| Randomness | Single serializable PCG-style RNG in `GameState` | `rand::StdRng`, per-system RNGs | `StdRng` isn't stable across crate versions for serde round-trips; one hand-rolled 30-line generator is deterministic, serializable, dependency-free |
| Sim/shell time boundary | Shell drives ticks; sim never reads clocks | Sim-internal timer thread | Determinism and testability; catch-up is just a loop |
| Negative pressure shape | Multipliers on gains only | Stock confiscation, entity loss | Mechanical form of Tenet 1 (cozy over grim) — enforceable as an invariant test |
| Event reporting | Typed event enum returned from `tick()`/commands | Callbacks, observer pattern, string log in core | Pure data out of a pure function; shell composes presentation; tests assert on events directly |

## Open Questions & Future Decisions

### Deferred
1. Whether flavor-event *frequency* should adapt to session length (delight without spam) — tune after first play.
2. Whether the roadmap systems arrive as new leaves or promote `economy/` into deeper nodes — decide per system when reached.

## References

- Parent: `docs/high-level-design.md` (tenets, workspace split, time-at-the-boundary decision)
- Children: `engine/`, `economy/`, `kittens/`, `curiosity/`
- Sibling segment: `docs/intent/persistence/` (save format, offline catch-up policy)
