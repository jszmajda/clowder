---
parent: sim
prefix: SIM-SUN
---

# Ceremonies

## Context and Design Philosophy

Ceremonies are clowder's faith system, reimagined as warmth and devotion rather than worship and sacrifice. Kittens gather in sunpuddles for Sunbeam Ceremonies; their quiet devotion accrues, is offered up in a Basking, and — at the deepest tiers — settles into a permanent serenity that brightens every meadow the clowder ever moves to. This is the inspiration's three-layer faith economy (faith → worship → epiphany) with its solemnity kept and its severity removed: nothing is sacrificed, only offered and remembered.

The leaf owns the three ceremony currencies and their conversions, the Long Sun production bonus (the game's largest single multiplier), the Sunbeam Rites ladder, and the Enlightenment tiers. It consumes warmth (produced by Sun Rooms / Sunwardens — economy leaf) and shinies; it contributes the Long Sun bonus into the engine's production stack (layer 8) and Rite effects through the effect-key system.

## The Three Currencies

| Currency | Tier | Gained by | Spent / role |
|---|---|---|---|
| **warmth** | transient (economy) | Sun Rooms, Sunbeam Altars, Sunwardens | fuel for Basking; Rite purchases |
| **devotion** | uncapped (this leaf) | **Basking**: `devotion += warmth × (1 + reverence)`, warmth zeroed | drives the Long Sun; unlocks Rites at thresholds |
| **serenity** | legacy — persists across the Great Meadow Move (this leaf) | **Sunrise Reverie** (needs the Reverie Rite): `serenity += devotion / 1e6 × (enlightenment_tier + 1)² × 1.01`, devotion reset to a spark | `reverence = unlimited_dr(serenity, 0.1) × 0.1` multiplies every future Basking — permanent, compounding growth |

Basking (`Bask` command) and Sunrise Reverie (`Reverie` command) are the two offerings. Both are always safe: warmth and devotion are *given*, never lost to a penalty, and serenity only ever grows.

## The Long Sun

The ceremonies' production bonus, applied multiplicatively at production-stack layer 8 (material flows only):

```
long_sun = limited_dr( unlimited_dr(devotion, 1000) / 100 , cap )
```

`cap = 10` (a +1000% ceiling) base, extended by Enlightenment content (Black-Obelisk analog: **Sunstones**, +0.05 per tier per Sunstone). The Long Sun is why devotion matters: it turns accumulated quiet gratitude into the meadow's brightest multiplier.

## Sunbeam Rites

One-time purchases (`BuyRite`), warmth + shinies priced, unlocked when *devotion ever reached* a threshold (the inspiration's worship-gate pattern — the ladder opens as you Bask, not as you spend). Rites host at the Sunbeam Altar (economy structure). The ladder (11 rites):

| # | Rite | Cost | Devotion gate | Effect |
|---|---|---|---|---|
| 0 | Morning Stretch | 100 warmth | 150 | warmth production +10% |
| 1 | Shared Sunpuddle | 250 warmth, 150 shinies | 300 | Sunbeam Altars +science-analog: +0.02 curiosity cap-ratio each |
| 2 | Golden Hour | 350 warmth, 250 shinies | 500 | warmth cap ×(1.4 + 0.1 per Golden Hour) |
| 3 | Warm Welcome | 500 warmth, 250 shinies | 750 | Altars +0.4 happiness points each |
| 4 | Stained Sunlight | 500 warmth, 250 shinies | 750 | Altars +0.05 charm/t each |
| 5 | The Long Sun | 750 warmth, 500 shinies | 1000 | **enables the Long Sun production bonus** |
| 6 | Grand Basking | 1250 warmth, 750 shinies | 10000 | Altars +charm cap and +charm/t |
| 7 | Sun Wardens' Oath | 3500 warmth, 1500 shinies | 75000 | Altars +manpower-analog: +pep cap each |
| 8 | The Reverie | 5000 warmth, 1800 shinies | 100000 | **enables Sunrise Reverie** (devotion → serenity) |
| 9 | Enlightenment | 7500 warmth, 2000 shinies | 125000 | **enables the Enlightenment tier ladder**; makes rites 0–7 re-buyable (stacking, price ×2.5 each) |
| 10 | Everwarm Frescoes | 4000 warmth, 1800 shinies, 8000 charm | 200000 | Sun Rooms gain warmth cap = unlimited_dr(frescoes, 1.65) × warm_memories (prestige leaf) |

Each rite carries a **blurb**: a short joke and then its effect in plain words, marking each resource it names as the curiosity leaf's blurbs do (SIM-CUR-014). The shell shows it as what the rite does, so every one says why it is worth having before it is bought (Tenet 3).

## Enlightenment Tiers

`Enlighten` spends *serenity* (the inspiration's transcendence ladder): reaching tier t has cumulative cost `total_cost(t) = inverse_unlimited_dr(e^t / 10, 0.1)` — brutally exponential — so each Enlighten spends the marginal `total_cost(t) − total_cost(t−1)`. Each tier squares the Reverie conversion (via `(tier+1)²`), raises the Long Sun cap through Sunstone content, and unlocks the Dream Cartography rites (echo-priced, wave 6 seam). Tiers persist across the Move (they're serenity-bought and serenity persists).

## Cozy Notes

- No faith is ever *lost*: Basking and Reverie zero their input currency by *offering* it upward, and the output (devotion, serenity) is strictly larger in value. The one place the inspiration destroys accumulated worship (Adore wiping it) becomes here "the devotion settles into serenity" — a transformation, not a loss, and serenity is permanent.
- Rites are never disabled or taken back (cozy invariant: no confiscation).

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|----------|--------|------------------------|-----------|
| Three currencies | warmth (capped) → devotion (uncapped) → serenity (persistent) | Single faith pool | The three timescales (session, run, forever) are what make faith deepen rather than plateau; it's the inspiration's proven shape |
| Long Sun formula | `LDR(UDR(devotion,1000)/100, cap)` | Linear devotion bonus | Double-DR keeps the biggest multiplier from running away while still rewarding deep devotion; cap is extendable so late content re-opens it |
| Reverie squares on tier | `(tier+1)²` | Linear tier scaling | Squaring makes each Enlightenment tier a real leap, matching the exponential serenity cost |
| Offering, not sacrifice | Input zeroed by transformation upward | Keep the sacrifice framing | Tenet 1: the mechanic (spend-to-permanently-grow) is identical; only the story changes |

## Open Questions & Future Decisions

### Deferred
1. Dream Cartography rites (echo-priced) — land with the dreamtime wave; the Enlightenment ladder reserves their unlock.
2. Whether Sunwardens should also passively convert a slice of warmth to devotion (the Void-Order analog) — a later Tradition.

## References

- Parent: `../sim-design.md` (production stack layer 8, cozy inversions)
- warmth production & Sunbeam Altar structure: `../economy/`; Sunwarden job: `../economy/` Jobs; warm_memories (Frescoes): `../prestige/`
