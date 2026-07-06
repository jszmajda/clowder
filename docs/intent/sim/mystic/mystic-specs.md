# Mystic Specs (SIM-UNI)

## The Currency Chain

- [ ] **SIM-UNI-001**: When HoldParade is applied after the Parade Routes idea (SIM-CUR) is researched, with at least 2500 unicorns, the system shall spend 2500 unicorns and add 2500 × (Rainbow Arches built) × parade-ratio rainbows; before the idea it rejects with Locked, and below 2500 unicorns with InsufficientResources.
- [ ] **SIM-UNI-002**: The system shall multiply Unicorn Meadow production by (1 + Σ unicornsRatio effects from Arch upgrades and rites), applied as the mystic tier's own multiplier (not the global material layers).
- [ ] **SIM-UNI-003**: When RefineStarshard is applied with at least 25 prisms, the system shall spend 25 prisms and add 1 × (1 + starshard-refine effects) starshards; RefineEcho with at least 25 starshards shall spend 25 and add 1 × (1 + echo-refine effects) echoes.
- [ ] **SIM-UNI-004**: When CondenseJar is applied after the Jar-condensing arch is bought, with at least 10000 rainbows and Jar cap remaining, the system shall spend 10000 rainbows and add 1 Jar of Purrs (clamped to cap).

## Rainbow Arches

- [ ] **SIM-UNI-005**: The system shall define exactly the eight Rainbow Arch upgrades of the mystic design's ladder — rainbow/prism/other costs and effects — buyable once each via BuyArch (Locked before the Parade Routes idea, InsufficientResources when unaffordable), their effects flowing through the effect-key system.

## Prisms, Dreaming & Dreamcorns

- [ ] **SIM-UNI-006**: At each day boundary, the system shall roll a prism descent with probability equal to the summed prism-descent effects (0.02%/day per Rainbow Arch structure, plus Arch-upgrade bonuses), adding one prism on success, in the fixed random-event draw order.
- [ ] **SIM-UNI-011**: When the Parade Routes idea is researched, the system shall grant one Rainbow Arch structure free (so Parades yield rainbows and prisms can descend without a bootstrap deadlock).
- [ ] **SIM-UNI-007**: The system shall accrue dream-drift per tick as 0.000001 × Memory Arches × (1 + dream-boost effects), multiplied by 0.25 while any dreamcorn already exists; when dream-drift reaches 1 it shall reset by 1 and turn one prism into one dreamcorn — never the last prism.
- [ ] **SIM-UNI-008**: The system shall never reduce the dreamcorn stock except as a Promise's daily upkeep, and dreaming shall never delete a prism below one.

## Promises

- [ ] **SIM-UNI-009**: When MakePromise is applied with a free Promise slot and enough echoes, the system shall spend the echoes and fill a slot; each active Promise draws 0.0005 dreamcorn/day upkeep and contributes its small global effect through the effect-key system.
- [ ] **SIM-UNI-010**: While a Promise's daily dreamcorn upkeep cannot be met, the system shall mark that Promise quiet — suspending its effect while holding its slot — and restore it automatically once dreamcorn upkeep can be paid; no Promise, alicorn-analog, or kitten skill is ever destroyed.
