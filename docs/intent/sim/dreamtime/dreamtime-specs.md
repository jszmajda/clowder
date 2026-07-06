# Dreamtime Specs (SIM-DREAM)

## The Deep Nap

- [ ] **SIM-DREAM-001**: When DeepNap(years) is applied after the Deep Nap Theory idea is researched, with starshard_cost(years) starshards affordable and years within the current max nap length, the system shall spend the starshards and fast-forward the meadow by years game-years through the standard tick() (the same code path as live play and offline catch-up), applying per-year yields; before the idea or over the max length it rejects with Locked or InsufficientResources.
- [ ] **SIM-DREAM-002**: Each Deep Nap shall accrue stirred dreams (heat) of years × heat_per_year × (1 − heat_efficiency effects), and the starshard cost of napping shall rise with current stirred dreams; stirred dreams shall never reduce any stock or entity — they are only a cadence brake.
- [ ] **SIM-DREAM-003**: The system shall dissipate stirred dreams passively each tick (−= dissipation × (1 + cooling effects)), flooring at zero.

## Déjà-Vu Days & Reverie

- [ ] **SIM-DREAM-004**: At each season boundary, the system shall roll a déjà-vu day with probability equal to the summed déjà-vu-chance effects (Memory Chests, Dream Loom); on a déjà-vu day ordinary production pauses and the clowder gathers reverie (a deterministic amount from the RNG, raised by reverie-yield effects) instead.
- [ ] **SIM-DREAM-005**: The system shall only ever increase reverie (earned in déjà-vu days and, with the Lucid Nap upgrade, a little per nap); no mechanic shall reduce reverie except a player-chosen Dream Loom purchase.

## The Dream Loom

- [ ] **SIM-DREAM-006**: The system shall define exactly the nine Dream Loom upgrades of the dreamtime design's ladder — reverie (and Jar) costs and effects — buyable once each via BuyDreamLoom (InsufficientResources when unaffordable), their effects flowing through the effect-key system.

## The Distant Dawn

- [ ] **SIM-DREAM-007**: While the Distant Dawn loom upgrade is bought and the meadow has reached year 40000 (by naps or play), the system shall enter the Distant Dawn era, permanently raising the Passed-Down Wisdom production cap by a factor of four; the era is a permanent brightening and is never lost.
