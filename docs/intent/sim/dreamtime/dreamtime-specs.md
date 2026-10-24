# Dreamtime Specs (SIM-DREAM)

## The Deep Nap

- [x] **SIM-DREAM-001**: When DeepNap(years) is applied after the Deep Nap Theory idea is researched, with starshard_cost(years) starshards affordable and 1 ≤ years ≤ max_nap_length (base 100, raised by the Long Slumber loom), the system shall spend the starshards and fast-forward the meadow by years game-years purely through the standard tick() — the identical code path as offline catch-up, no added per-year yields (the advancing tick counter alone delivers the Move-time millennium trickle) — returning a summarized nap report (elapsed years and net deltas, not a per-tick event stream); before the idea it rejects with Locked, over max_nap_length or unaffordable with InsufficientResources. Reaching the Distant Dawn (meadow-year 40000) takes repeated bounded naps within one un-Moved meadow.
- [x] **SIM-DREAM-002**: Each Deep Nap shall accrue stirred dreams (heat) of years × heat_per_year × (1 − heat_efficiency effects), and the starshard cost of napping shall rise with current stirred dreams; stirred dreams shall never reduce any stock or entity — they are only a cadence brake.
- [x] **SIM-DREAM-003**: The system shall dissipate stirred dreams passively each tick (−= dissipation × (1 + cooling effects)), flooring at zero.

## Déjà-Vu Days & Reverie

- [x] **SIM-DREAM-004**: At each season boundary, the system shall roll a déjà-vu day with probability equal to the summed déjà-vu-chance effects (Memory Chests, Dream Loom); on a déjà-vu day both ordinary production AND catnip consumption pause (so a dreamy day never causes a shortfall — Tenet 1) and the clowder gathers reverie (a deterministic amount from the RNG, raised by reverie-yield effects) instead.
- [x] **SIM-DREAM-005**: The system shall only ever increase reverie (earned in déjà-vu days and, with the Lucid Nap upgrade, a little per nap); no mechanic shall reduce reverie except a player-chosen Dream Loom purchase.

## The Dream Loom

- [x] **SIM-DREAM-006**: The system shall define exactly the nine Dream Loom upgrades of the dreamtime design's ladder — reverie (and Jar) costs and effects — buyable once each via BuyDreamLoom (InsufficientResources when unaffordable), their effects flowing through the effect-key system.

## The Distant Dawn

- [x] **SIM-DREAM-007**: When the Distant Dawn loom upgrade is bought and the meadow reaches year 40000, the system shall set a persisted distant_dawn flag (never cleared, preserved across Moves) that raises the Passed-Down Wisdom production cap from +100% to +400%; the flag is read in the production stack, not re-derived from the (Move-resettable) tick counter.
- [x] **SIM-DREAM-008**: The system shall deliver a bulk-time action's summary by the same route offline absence uses — a report folded from the replay and retained in the journal exactly once, as its away entry — so that one mechanism serves every way time passes in bulk and a caller has one place to read it. A deliberate sleep is always retained, however short: the minimum span below which nothing is recorded exists for absences, which are accidents of the clock, not for an act the player chose.
- [x] **SIM-DREAM-011**: When DeepNap(years) completes with the Lucid Nap loom woven, the system shall add 0.1 × years reverie, counted in the nap's report; and where the Deep Cushion loom is woven, it shall multiply a nap's starshard cost by 0.95 before rounding, alongside Long Slumber's discount.
- [x] **SIM-DREAM-012**: The system shall give every Dream Loom upgrade a blurb — a short joke, then the upgrade's effect in plain words — marking each resource it names as SIM-CUR-014 marks them.
