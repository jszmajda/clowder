---
parent: sim
prefix: SIM-EXP
---

# Expeditions

## Context and Design Philosophy

Pouncing expeditions are the active-play burst: the player spends stored pep on squads that scamper beyond the meadow and come back with luxuries, glitter, and — rarely — a unicorn who decides to follow them home. This is the inspiration's hunting loop with the violence swapped for play: squads chase the Red Dot, raid the yarn basket, and stalk the long grass; nothing is hunted, everything is found. The leaf owns the pounce command, its yield distributions, and the pounce-ratio effect key. Pep production (Pouncers) and the luxury drain live in the economy leaf; this leaf turns pep into loot.

## The Pounce

`Pounce(squads)` spends 100 pep per squad (1 ≤ squads ≤ floor(pep/100); zero affordable squads rejects with InsufficientResources). Yields use Irwin–Hall sums (IH(n) = sum of n uniform draws — mean n/2, bell-shaped) with a fixed draw order per pounce (feathers roll, bells binomial + IH, shinies binomial + IH, unicorn binomial), so the RNG schedule is deterministic given squad count:

| Yield | Distribution |
|---|---|
| Feathers | (8 + 6.5 × pounce_ratio) × IH(squads) — every squad finds some |
| Bells | hits ~ Binomial(squads, 0.45 + 0.02 × pounce_ratio); (5 + 4 × pounce_ratio) × IH(hits) |
| Shinies | hits ~ Binomial(squads, 0.40); (1 + 1 × pounce_ratio) × IH(hits), cap-clamped |
| Unicorns | Binomial(squads, 0.05) — each success is one unicorn who tags along, delighted |

`pounce_ratio` = Σ PounceRatio effects (refinement ladder, the Bold Elder). A `PounceReturned` event reports the squads and the full haul (data, not prose). Yields land immediately (expeditions are a moment, not a timer — the idle tenet keeps clocks out of active play).

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|----------|--------|------------------------|-----------|
| Instant resolution | Yields land on command | Timed expeditions with return events | Timers make active play an obligation to come back — the idle tenet says bursts, not appointments |
| Irwin–Hall yields | Sum of uniforms per squad | Flat per-squad yield; single big roll | Bell-shaped hauls feel organic; per-squad draws keep big sends exciting without jackpot variance |
| Unicorns follow home | Rare binomial per squad | Purchase; guaranteed drip | The first unicorn should be a story the player tells; the meadow structure takes it from there |
| The shinies a pounce brings | A full pep store pounced at a mid-game pounce ratio brings home about one caravan's worth | A sprinkle of a few per hundred squads | A caravan's shinies costing many full pep stores makes pouncing feel like a chore and trading out of reach. The steady supply is the Couch Mine's (economy leaf), so a pounce's shinies are a bonus worth chasing, never the only way to get them (Tenet 2) |

## Open Questions & Future Decisions

### Deferred
1. Expedition "destinations" (biome-flavored yield tables) — a post-TUI content pass if pouncing proves fun.

## References

- Parent: `../sim-design.md`
- Pep production and luxury definitions: `../economy/economy-design.md`; pounce-ratio refinements: `../curiosity/curiosity-design.md`; Bold trait: `../kittens/kittens-design.md`
