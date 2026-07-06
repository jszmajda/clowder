---
parent: sim
prefix: SIM-DREAM
---

# Dreamtime — Naps, Skips, and the Distant Dawn

## Context and Design Philosophy

Dreamtime is clowder's time layer, reimagined as *napping*: a well-rested clowder can drift into a long, warm dream and wake to find the meadow has quietly advanced. This is the inspiration's Time/Chronoforge/Void arc, kept cozy — the clowder never "shatters time" or accumulates "sorrow"; it takes deep naps that fast-forward the meadow, gathers **reverie** in the strange half-lit **déjà-vu days**, and weaves it into permanent comfort. Naps generate warmth (literally, from all that dozing), which must gently dissipate before the next nap. At the very end of time — the **Distant Dawn** — the oldest legacies deepen.

The leaf owns the starshard-fueled Deep Nap (year-skipping), the stirred-dreams heat and its dissipation, the Dream Loom upgrades, the reverie currency and déjà-vu days, and the Distant Dawn era. It reads starshards/echoes (mystic) and contributes nap-boost and heat effects through the effect-key system.

## The Deep Nap

`DeepNap(years)` (needs the Deep Nap Theory idea) spends starshards to skip the meadow forward `years` game-years instantly — the meadow accrues exactly what it would have accrued awake (the same `tick()`, fast-forwarded, like offline catch-up, so no shadow math), *plus* the per-year yields (a year's Elder-Wisdom millennium trickle lands too). Each nap:

- spends `starshard_cost(years)` starshards (escalating),
- accrues **stirred dreams** (heat): `+= years × heat_per_year × (1 − heat_efficiency)` — the more you nap, the warmer the dream-space gets,
- yields the fast-forwarded meadow state.

**Stirred dreams must cool** before more napping is comfortable: while stirred_dreams is high, the starshard cost of napping rises (the inspiration's heat-raises-shatter-cost). Stirred dreams dissipate passively (`−= dissipation/tick`, faster with Dream Loom cooling upgrades) — a slowdown on napping cadence, never a loss of anything.

## Déjà-Vu Days

Occasionally a nap (or a season boundary, at `deja_vu_chance` per season from Memory Chests) produces a **déjà-vu day**: a strange, half-remembered day where ordinary production pauses and the clowder gathers **reverie** instead (`+rand-ish reverie/day`, deterministic from the RNG). Reverie is the dreamtime currency — earned only in these uncanny days, spent on the Deep-Nap infrastructure.

## The Dream Loom

Reverie-priced upgrades (`BuyDreamLoom`, ~9) that make napping deeper and cooler:

| # | Loom upgrade | Cost (reverie) | Effect |
|---|---|---|---|
| 0 | Soft Pillow | 5 | heat efficiency +10% (naps stir less) |
| 1 | Warm Blanket | 15 | stirred-dreams dissipation +20% |
| 2 | Dream Catcher | 40 | déjà-vu day chance +25% |
| 3 | Deep Cushion | 100 | per-year nap yield +5% |
| 4 | Lucid Nap | 250 | naps also advance reverie a little |
| 5 | Cozy Cave | 500 | heat efficiency +15% |
| 6 | Long Slumber | 1000 | max nap length +; starshard cost −10% |
| 7 | Reverie Net | 2500 | déjà-vu days yield +50% reverie |
| 8 | The Distant Dawn | 10000 reverie, 100 Jars | unlocks the Distant Dawn era (below) |

## The Distant Dawn

At the far end of time (the inspiration's Dark Future at year 40000, here the **Distant Dawn** — a warm sunrise after the longest night), reached by naps or play past year 40000 with the Distant Dawn loom unlocked: the era permanently **raises the Passed-Down Wisdom caps** (×4, per the prestige leaf's note), deepening the burn-tier for the longest games. It is a brightening, not a darkening — the dawn the whole game has been napping toward.

## Cozy Notes

- The inspiration's time layer is its bleakest at the edges (heat death, void, sorrow, the Dark Future). Clowder's is its dreamiest: naps, blankets, half-remembered days, and a distant dawn. Stirred dreams are *warmth from dozing*, not damage; they cool on their own.
- A nap never costs the player anything but starshards, and never skips past anything the player would have wanted to watch (they chose to nap).
- Reverie can only be gained, never lost to a penalty.

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|----------|--------|------------------------|-----------|
| Naps replay real `tick()` | Fast-forward the same simulation + per-year yields | Closed-form skip | One simulation, no shadow math (the offline-catch-up principle applied to voluntary skips) |
| Heat as a cadence brake | Stirred dreams raise nap cost, dissipate passively | Hard nap cooldown; no brake | The inspiration's heat mechanic — a soft, self-clearing brake beats a hard timer and never blocks the player permanently |
| Reverie only in déjà-vu days | Uncanny-day-gated currency | Passive reverie | Gives naps a second reward loop and makes déjà-vu days a delight rather than an interruption |
| Distant Dawn brightens | Raises Passed-Down caps | Keep the Dark Future's grimness | Tenet 1 — the end of time is a sunrise, and the mechanical role (raise the burn caps) is preserved |

## Open Questions & Future Decisions

### Deferred
1. Whether déjà-vu days should have their own flavor-event pool — a shell-era delight.
2. A reverie-priced capstone that interacts with the Furthest Star (cosmic) — decided after play.

## References

- Parent: `../sim-design.md` (dreamtime tier, cozy inversions — the time layer)
- Starshards/echoes/Jars: `../mystic/`; Deep Nap Theory / Dream Loomcraft ideas: `../curiosity/`; Memory Chest déjà-vu chance: `../economy/`; the Distant Dawn raises `../prestige/` Passed-Down caps; naps replay `tick()` from `../engine/` like `../../persistence/` catch-up
