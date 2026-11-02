# Mystic Specs (SIM-UNI)

## The Currency Chain

- [x] **SIM-UNI-001**: When HoldParade is applied after the Parade Routes idea (SIM-CUR) is researched, with at least 2500 unicorns, the system shall spend 2500 unicorns and add the sum, over every structure, of its Parade rainbows (SIM-UNI-013) × copies, × (1 + parade-ratio effects), rainbows; before the idea it rejects with Locked, and below 2500 unicorns with InsufficientResources.
- [x] **SIM-UNI-002**: The system shall multiply Unicorn Meadow production by (1 + Σ unicornsRatio effects from Arch upgrades), applied as the mystic tier's own multiplier (not the global material layers).
- [x] **SIM-UNI-003**: When RefineStarshard is applied with at least 25 prisms, the system shall spend 25 prisms and add 1 × (1 + starshard-refine effects) starshards; RefineEcho with at least 25 starshards shall spend 25 and add 1 × (1 + echo-refine effects) echoes.
- [x] **SIM-UNI-004**: When CondenseJar is applied after the Jar-condensing arch is bought, with at least 10000 rainbows and Jar cap remaining, the system shall spend 10000 rainbows and add 1 Jar of Purrs (clamped to cap); with no Jar cap remaining it rejects with StorageFull and changes nothing.

## Rainbow Arches

- [x] **SIM-UNI-005**: The system shall define exactly the eight Rainbow Arch upgrades of the mystic design's ladder — rainbow/prism/other costs and effects — buyable once each via BuyArch (Locked before the Parade Routes idea, InsufficientResources when unaffordable), their effects flowing through the effect-key system save the Sky Arch's steady prism drift and the Memory Arch's enabling of dreaming, which the dreaming rule reads directly; a rainbow shower brings 5000 rainbows.

## Prisms, Dreaming & Dreamcorns

- [x] **SIM-UNI-006**: At each day boundary, the system shall roll a prism descent with probability equal to the summed prism-descent effects (each structure's prism-descent chance (SIM-UNI-014) × copies, plus Arch-upgrade bonuses) × the Prism Sight ratio where that tradition is held (SIM-MOVE-015), adding one prism on success, in the fixed random-event draw order.
- [x] **SIM-UNI-011**: When the Parade Routes idea is researched, the system shall grant one Rainbow Arch structure free (so Parades yield rainbows and prisms can descend without a bootstrap deadlock).
- [x] **SIM-UNI-007**: While the Memory Arch upgrade is owned (which enables the dreamcorn engine), the system shall accrue dream-drift per tick as the sum, over every structure, of its dream-drift (SIM-UNI-015) × copies, × (1 + dream-boost effects), multiplied by 0.25 while any dreamcorn already exists; each whole 1 of dream-drift shall leave the drift and turn one prism into one dreamcorn — never the last prism, the drift banking meanwhile and paying out once a prism beyond the last is held.
- [x] **SIM-UNI-008**: As time passes, the system shall reduce the dreamcorn stock only as a Promise's daily upkeep, its other falls being a player-chosen spend or the Move's reset (SIM-MOVE-004), and dreaming shall never delete a prism below one.

## Promises

- [x] **SIM-UNI-009**: When MakePromise is applied after the Night Court Etiquette idea, with a free Promise slot (three once the Everdream Arch is bought, none before) and 1 echo, the system shall spend the echo and fill a slot, rejecting with Locked before the idea, NoSeats with no free slot, and InsufficientResources without the echo; each active Promise draws 0.0005 dreamcorn/day upkeep and contributes +0.05% global production through the effect-key system.
- [x] **SIM-UNI-010**: While a Promise's daily dreamcorn upkeep cannot be met, the system shall mark that Promise quiet — suspending its effect while holding its slot — and restore it automatically once dreamcorn upkeep can be paid; no Promise, alicorn-analog, or kitten skill is ever destroyed, a new meadow's Promises starting afresh under the Move's reset (SIM-MOVE-004).
- [x] **SIM-UNI-012**: The system shall give every Rainbow Arch upgrade a blurb — a short joke, then the upgrade's effect in plain words — marking each resource it names as SIM-CUR-014 marks them.
- [x] **SIM-UNI-013**: The system shall give every structure definition the rainbows each copy brings to a Parade — 2500 for the Rainbow Arch and none for any other structure.
- [x] **SIM-UNI-014**: The system shall give every structure definition the chance each copy adds to a day's prism descent — 0.02% for the Rainbow Arch and none for any other structure.
- [x] **SIM-UNI-015**: The system shall give every structure definition the dream-drift each copy accrues per tick — 0.000001 for the Rainbow Arch and none for any other structure.
