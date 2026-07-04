---
parent: sim
prefix: SIM-ECO
---

# Economy

## Context and Design Philosophy

The economy is the material world of the game: what exists, what it costs, what produces what. It owns resource kinds and caps, structures and their price ramps, the manual actions, and all resource-flow math. It consumes two inputs from the kittens leaf — *effective workers per job* and the *mood multiplier* — and unlock flags from the curiosity leaf; it never inspects the roster or the idea tree directly.

Balance philosophy: the early game is hand-fed (manual gathering), the mid game is delegated (kittens work), and every season cycle should create gentle planning pressure — stock up before winter — that is felt as *slowdown*, never as loss (cozy invariant: negative pressures are multipliers on gains).

## Resources

| Resource | Role | Base cap | Cap extension |
|---|---|---|---|
| **Catnip** | food & primary currency | 200 | +250 per Pantry |
| **Twigs** | building material | 50 | +40 per Twig Pile |
| **Fluff** | soft material (grooming) | 25 | +10 per Cat Tree (fluff-storage nooks) |
| **Curiosity** | idea currency | 0 | +50 per Cardboard Box |

Curiosity's base cap of 0 is the gate: nothing accrues until the first Cardboard Box exists. Stocks are `f64`, clamped to caps in the engine's clamp phase. Spends require the full amount (no debt), with an epsilon-tolerant affordability check (`stock + 1e-9 ≥ cost`) so float drift can never make a purchase the display calls affordable get rejected; deductions floor at zero. Without the Cat Tree's fluff extension, escalating fluff costs would quickly outgrow the static base cap and leave later copies permanently unbuyable — cap extensions must keep every structure's first 20 copies reachable. Far beyond that horizon (copies ~24+ of self-capping chains), exponential costs outrun linear cap growth by design: the late game plateaus softly rather than walling, and the roadmap's deeper systems arrive before any player grinds there.

## Manual Actions

| Action | Effect | Notes |
|---|---|---|
| **Gather catnip** | +1 catnip | ×2 once the gather-doubling idea (curiosity leaf) is researched |
| **Forage** | −10 catnip, +1 twig | the founding cat snacks up and drags a twig home; available from the start |

A manual action whose *yield* resource is at cap is rejected with `StorageFull` rather than spending a press (or catnip) on a trimmed-away yield — the player always learns why nothing happened (Tenet 3).

## Structures

Cost to buy the *n*-th copy (0-indexed count `k`): `ceil(base × ratio^k)` per cost component. All structures are available from the start unless gated by an idea (flag from the curiosity leaf).

| Structure | Base cost | Ratio | Effect | Gate |
|---|---|---|---|---|
| **Catnip Patch** | 10 catnip | 1.12 | +0.125 catnip/tick each, ×season modifier | — |
| **Cozy Cushion** | 25 catnip, 5 twigs | 1.15 | houses 1 kitten | — |
| **Cardboard Box** | 8 twigs | 1.15 | +50 curiosity cap; +0.005 curiosity/tick each (an unattended box radiates mystery) | — |
| **Pantry** | 20 twigs | 1.15 | +250 catnip cap | idea |
| **Twig Pile** | 50 catnip, 10 twigs | 1.15 | +40 twig cap | idea |
| **Scratching Post** | 15 twigs, 5 fluff | 1.15 | +5% happiness each (kittens leaf consumes) | idea |
| **Cat Tree** | 30 twigs, 10 fluff | 1.15 | houses 3 kittens; +10 fluff cap | idea |

## Jobs — Production Rates

The kittens leaf owns who works; this leaf owns what work yields, per effective worker per tick, multiplied by the mood multiplier:

| Job | Yield/tick | Gate |
|---|---|---|
| **Gardener** | +0.15 catnip | — (available with first kitten) |
| **Twig Collector** | +0.03 twigs | — |
| **Groomer** | +0.02 fluff | idea |
| **Observer** | +0.025 curiosity | requires ≥1 Cardboard Box |

## Seasons

Season modifier applies to **Catnip Patch production only** (gardeners work the sheltered beds; their rate is season-independent):

| Season | Catnip Patch modifier |
|---|---|
| Spring | ×1.5 |
| Summer | ×1.0 |
| Autumn | ×0.85 |
| Winter | ×0.25 (×0.40 with the winter-easing idea) |

## Consumption

Each kitten **at home** eats 0.085 catnip/tick (0.34/s). Away kittens neither eat nor work. Consumption is not mood-modified — grumpy kittens still get fed when food exists.

Consumption is **pooled**: total demand = rate × home kittens, deducted as one sum. When stock < demand, the stock goes to zero and the tick is marked a **shortfall** (the engine's mood phase turns shortfall into Grumpiness). There is no per-kitten hunger state — the clowder shares, and shortfall is a collective mood, not an individual injury.

Balance anchors (design targets, verified by tests): one patch in summer feeds ~1.5 kittens; a gardener feeds ~1.75 (0.6/s produced vs 0.34/s eaten at full mood); in winter an unaided patch feeds ~0.37 kittens — hence the pantry-stocking rhythm.

## Zoomies Interaction

The zoomies multiplier (engine leaf) applies to **job production only**, same scope as mood — structures don't get the zoomies.

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|----------|--------|------------------------|-----------|
| Price ramp | Exponential `base × ratio^k`, ceil per component | Linear, triangular | Genre-standard pacing; ceil keeps prices integral and legible in a TUI |
| Twig acquisition bootstrap | Forage action converts catnip→twigs | Free twig-gather key, twigs from patches | A conversion makes catnip meaningfully the *currency* pre-kittens and teaches the spend-to-build loop |
| Season scope | Patches only, not gardeners | All catnip production seasonal | Keeps winter a planning problem (stock + gardeners carry you) rather than a shutdown; softer per Tenet 1 |
| Curiosity gating | Cap 0 until first Box | Idea-gated Observer job only | Physical gate is discoverable by play ("the box did something!") without reading docs — Tenet 3 |
| Fluff cap static in v1 | 25, no extender | Cap-extending structure now | v1 fluff sinks are small; an extender lands with the crafting roadmap segment |

## Open Questions & Future Decisions

### Resolved
1. ✅ Consumption immune to mood: hungry-and-grumpy reducing eating would spiral kittens out of grumpiness slower — the opposite of cozy recovery.

### Deferred
1. Whether Forage should scale with an idea (batch foraging) — revisit when mid-game twig demand is play-tested.
2. Fluff sources beyond Groomers (seasonal shedding?) — crafting segment.

## References

- Parent: `../sim-design.md` (ownership boundaries, cozy invariants, pipeline order)
- Rates consumed by the engine's production phase; worker counts and mood from `../kittens/`; unlock flags from `../curiosity/`
